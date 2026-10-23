#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod i18n;
mod model;
mod platform;
mod view;

use batterybridge_protocol::{Command, Response};
use fastframe_shell::{Closed, Headless, Held, Resident, Shell, Waker};
use fastframe_tray::{Event, MenuItem, Tray};
use std::{
    path::PathBuf,
    sync::{OnceLock, mpsc},
    time::{Duration, Instant},
};

const REFRESH: Duration = Duration::from_secs(30);
const APP_ID: &str = "io.github.batterybridge";

#[derive(Default)]
struct Options {
    preview: Option<PathBuf>,
    language: Option<String>,
}
impl Options {
    fn parse() -> Result<Option<Self>, String> {
        let mut options = Self::default();
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--help" | "-h" => {
                    println!(
                        "BatteryBridge\nUsage: batterybridge-gui [--preview SNAPSHOT.json [--language LOCALE]]"
                    );
                    return Ok(None);
                }
                "--version" | "-V" => {
                    println!("BatteryBridge {}", env!("CARGO_PKG_VERSION"));
                    return Ok(None);
                }
                "--preview" => {
                    options.preview =
                        Some(args.next().ok_or("--preview needs a snapshot path")?.into())
                }
                "--language" => {
                    options.language = Some(args.next().ok_or("--language needs a locale")?)
                }
                _ => return Err(format!("Unknown argument: {arg}")),
            }
        }
        if options
            .language
            .as_ref()
            .is_some_and(|s| !i18n::valid_choice(s))
        {
            return Err("Unsupported language".into());
        }
        if options.language.is_some() && options.preview.is_none() {
            return Err(
                "--language is a preview option; use the application's language selector".into(),
            );
        }
        Ok(Some(options))
    }
}

enum Reply {
    Protocol(Result<Response, String>),
    Uninstalled(Result<(), String>),
    Saved(Result<(), String>),
}

pub(crate) struct App {
    model: model::Model,
    language: String,
    locale: String,
    preferences_loaded: bool,
    tray: Option<Tray>,
    waker: Waker,
    sender: mpsc::Sender<Reply>,
    replies: mpsc::Receiver<Reply>,
    busy: bool,
    refreshed: Instant,
    report: Option<String>,
    report_destination: String,
    report_saved: bool,
    confirm_uninstall: bool,
    wants_show: bool,
    hide: bool,
    quit: bool,
    preview: bool,
}
impl App {
    fn new(waker: Waker, options: Options) -> Result<Self, String> {
        let (sender, replies) = mpsc::channel();
        let mut model = model::Model::default();
        let preview = options.preview.is_some();
        if let Some(path) = options.preview {
            model.accept(
                serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?,
            );
        }
        let language = options.language.unwrap_or_else(|| "system".into());
        let locale = i18n::resolve(&language, fastframe_i18n::system_languages()).to_owned();
        let tray = if preview {
            None
        } else {
            let wake = waker.clone();
            Tray::spawn(
                fastframe_tray::Config {
                    id: APP_ID,
                    title: "BatteryBridge".into(),
                    icon: icon_rgba,
                    template_icon: None,
                    themed_icon: true,
                    menu: vec![
                        MenuItem::action("open", i18n::text(&locale, "trayOpen")),
                        MenuItem::action("quit", i18n::text(&locale, "trayQuit")),
                    ],
                },
                move || wake.wake(),
            )
        };
        Ok(Self {
            model,
            language,
            locale,
            preferences_loaded: preview,
            tray,
            waker,
            sender,
            replies,
            busy: false,
            refreshed: Instant::now() - REFRESH,
            report: None,
            report_destination: platform::report_path().to_string_lossy().into_owned(),
            report_saved: false,
            confirm_uninstall: false,
            wants_show: false,
            hide: false,
            quit: false,
            preview,
        })
    }
    fn text(&self, key: &str) -> String {
        i18n::text(&self.locale, key)
    }
    fn language_changed(&mut self) {
        self.locale = i18n::resolve(&self.language, fastframe_i18n::system_languages()).to_owned();
        if let Some(tray) = &mut self.tray {
            tray.set_label("open", i18n::text(&self.locale, "trayOpen"));
            tray.set_label("quit", i18n::text(&self.locale, "trayQuit"));
        }
    }
    fn attach(&mut self, cc: &eframe::CreationContext<'_>) {
        if !self.preferences_loaded {
            self.preferences_loaded = true;
            if let Some(saved) = cc.storage.and_then(|s| s.get_string("language"))
                && i18n::valid_choice(&saved)
            {
                self.language = saved;
            }
            self.language_changed();
        }
        fastframe_fonts::FontSetup::default().install(&cc.egui_ctx);
        view::style(&cc.egui_ctx);
        if let Some(tray) = &mut self.tray {
            tray.attach();
            #[cfg(target_os = "macos")]
            if tray.is_shown() {
                fastframe_dock::set_dock_icon_visible(false);
            }
        }
        self.refreshed = Instant::now() - REFRESH;
    }
    fn spawn(&mut self, job: impl FnOnce() -> Reply + Send + std::panic::UnwindSafe + 'static) {
        if self.busy {
            return;
        }
        self.busy = true;
        let sender = self.sender.clone();
        let waker = self.waker.clone();
        std::thread::spawn(move || {
            let _ = sender
                .send(std::panic::catch_unwind(job).unwrap_or_else(|_| {
                    Reply::Protocol(Err("Background operation failed".into()))
                }));
            waker.wake();
        });
    }
    fn request(&mut self, command: Command) {
        if self.preview || self.busy {
            return;
        }
        if !matches!(command, Command::Status | Command::Probe) && self.model.read_only() {
            return;
        }
        if let Err(error) = command.validate() {
            self.model.error = Some(error.to_string());
            return;
        }
        if !matches!(command, Command::Status) {
            self.model.updated = false;
        }
        self.model.error = None;
        self.spawn(move || {
            Reply::Protocol(
                batterybridge_platform::client::query(command, batterybridge_ipc::call)
                    .map_err(|e| e.to_string()),
            )
        });
    }
    fn events(&mut self, ctx: &egui::Context, visible: bool) {
        while let Ok(reply) = self.replies.try_recv() {
            self.busy = false;
            match reply {
                Reply::Protocol(Ok(Response::Status(snapshot))) => {
                    self.model.accept(*snapshot);
                    self.refreshed = Instant::now();
                }
                Reply::Protocol(Ok(Response::Ok)) => {
                    self.model.edited = false;
                    self.model.updated = true;
                    self.request(Command::Status);
                }
                Reply::Protocol(Ok(Response::Probe(report))) => {
                    match serde_json::to_string_pretty(&report) {
                        Ok(report) => {
                            self.report = Some(report);
                            self.report_saved = false;
                        }
                        Err(error) => self.model.error = Some(error.to_string()),
                    }
                }
                Reply::Protocol(Ok(Response::Error(error))) => {
                    self.model.error = Some(error.to_string())
                }
                Reply::Protocol(Ok(_)) => {
                    self.model.error = Some("Unexpected service response".into())
                }
                Reply::Protocol(Err(error)) => {
                    self.model.error = Some(error);
                    self.refreshed = Instant::now();
                }
                Reply::Uninstalled(Ok(())) => self.quit = true,
                Reply::Uninstalled(Err(error)) | Reply::Saved(Err(error)) => {
                    self.model.error = Some(error)
                }
                Reply::Saved(Ok(())) => self.report_saved = true,
            }
        }
        if let Some(tray) = &self.tray {
            for event in tray.events() {
                match event {
                    Event::Menu("quit") => self.quit = true,
                    Event::Toggle | Event::Show | Event::Menu("open") => {
                        if visible {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                        } else {
                            self.wants_show = true;
                        }
                    }
                    Event::Menu(_) => {}
                }
            }
        }
        if self.quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
impl Resident for App {
    fn closed(&self) -> Closed {
        if self.hide && !self.quit {
            Closed::Hide
        } else {
            Closed::Quit
        }
    }
    fn window_gone(&mut self) {
        self.hide = false;
        self.wants_show = false;
    }
    fn headless_frame(&mut self, ctx: &egui::Context) -> Headless {
        self.events(ctx, false);
        if self.quit {
            Headless::Quit
        } else if self.wants_show {
            Headless::Show
        } else {
            Headless::WaitFor(REFRESH)
        }
    }
    fn shutdown(&mut self) {}
}
struct Window {
    app: Held<App>,
    focused: bool,
}
impl eframe::App for Window {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.app.events(ctx, true);
        if ctx.input(|i| i.viewport().close_requested()) {
            self.app.hide = self.app.tray.as_ref().is_some_and(Tray::is_shown) && !self.app.quit;
        }
        let focused = ctx.input(|i| i.viewport().focused.unwrap_or(false));
        if !self.app.preview
            && !self.app.busy
            && (self.app.refreshed.elapsed() >= REFRESH || focused && !self.focused)
        {
            self.app.request(Command::Status);
        }
        self.focused = focused;
        if !self.app.preview {
            ctx.request_repaint_after(
                REFRESH
                    .saturating_sub(self.app.refreshed.elapsed())
                    .max(Duration::from_secs(1)),
            );
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| self.app.render(ui));
    }
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if !self.app.preview {
            storage.set_string("language", self.app.language.clone());
        }
    }
    fn persist_egui_memory(&self) -> bool {
        false
    }
}

fn icon_data() -> &'static egui::IconData {
    static ICON: OnceLock<egui::IconData> = OnceLock::new();
    ICON.get_or_init(|| {
        eframe::icon_data::from_png_bytes(include_bytes!("../icons/128x128.png"))
            .expect("bundled icon")
    })
}
fn icon_rgba(size: usize) -> Vec<u8> {
    let icon = icon_data();
    let mut rgba = vec![0; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let index = ((y * icon.height as usize / size) * icon.width as usize
                + x * icon.width as usize / size)
                * 4;
            rgba[(y * size + x) * 4..(y * size + x + 1) * 4]
                .copy_from_slice(&icon.rgba[index..index + 4]);
        }
    }
    rgba
}
fn run() -> Result<(), String> {
    let Some(options) = Options::parse()? else {
        return Ok(());
    };
    let preview = options.preview.is_some();
    let waker = Waker::default();
    let app = App::new(waker.clone(), options)?;
    let shell = Shell::new(app, &waker);
    #[cfg(target_os = "macos")]
    let shell = shell.idle(fastframe_tray::idle);
    shell
        .run(|lease| {
            let mut options = eframe::NativeOptions {
                #[cfg(target_os = "windows")]
                renderer: eframe::Renderer::Wgpu,
                #[cfg(not(target_os = "windows"))]
                renderer: eframe::Renderer::Glow,
                viewport: egui::ViewportBuilder::default()
                    .with_app_id(if preview {
                        "io.github.batterybridge.preview"
                    } else {
                        APP_ID
                    })
                    .with_inner_size([420.0, 730.0])
                    .with_min_inner_size([320.0, 480.0])
                    .with_icon(icon_data().clone()),
                persist_window: !preview,
                ..Default::default()
            };
            if preview {
                options.persistence_path =
                    Some(std::env::temp_dir().join("batterybridge-preview.ron"));
            }
            eframe::run_native(
                if preview {
                    "BatteryBridge (Preview)"
                } else {
                    "BatteryBridge"
                },
                options,
                Box::new(move |cc| {
                    let mut app = lease.take(&cc.egui_ctx);
                    app.attach(cc);
                    Ok(Box::new(Window {
                        app,
                        focused: false,
                    }))
                }),
            )
        })
        .map_err(|e| e.to_string())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("BatteryBridge: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preview() -> App {
        App::new(
            Waker::default(),
            Options {
                preview: Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/status.json")),
                language: Some("ru".into()),
            },
        )
        .unwrap()
    }

    #[test]
    fn preview_never_starts_ipc_or_tray() {
        let mut app = preview();
        assert!(app.tray.is_none());
        for command in [
            Command::Status,
            Command::Probe,
            Command::Limit {
                stop: Some(80),
                start: None,
            },
        ] {
            app.request(command);
            assert!(!app.busy);
        }
    }

    #[test]
    fn offline_mode_rejects_writes_before_starting_a_worker() {
        let mut app = preview();
        app.preview = false;
        app.model.snapshot.as_mut().unwrap().config = None;
        app.request(Command::Limit {
            stop: Some(80),
            start: None,
        });
        assert!(!app.busy);
    }

    #[test]
    fn closing_without_a_tray_quits_and_quit_overrides_hide() {
        let mut app = preview();
        assert!(matches!(app.closed(), Closed::Quit));
        app.hide = true;
        assert!(matches!(app.closed(), Closed::Hide));
        app.quit = true;
        assert!(matches!(app.closed(), Closed::Quit));
    }
}
