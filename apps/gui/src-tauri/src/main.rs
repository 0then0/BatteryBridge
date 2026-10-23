#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use batterybridge_protocol::{Command, Error, Response};
use tauri::{
    Manager,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};

struct TrayLabels {
    open: MenuItem<tauri::Wry>,
    quit: MenuItem<tauri::Wry>,
}

fn language_catalog(locale: &str) -> Option<&'static str> {
    match locale {
        "en" => Some(include_str!("../../src/locales/en.json")),
        "ru" => Some(include_str!("../../src/locales/ru.json")),
        "es" => Some(include_str!("../../src/locales/es.json")),
        "de" => Some(include_str!("../../src/locales/de.json")),
        "fr" => Some(include_str!("../../src/locales/fr.json")),
        "ja" => Some(include_str!("../../src/locales/ja.json")),
        "pt-BR" => Some(include_str!("../../src/locales/pt-BR.json")),
        _ => None,
    }
}

#[tauri::command]
fn set_language(locale: String, labels: tauri::State<'_, TrayLabels>) -> Result<(), String> {
    let source = language_catalog(&locale).ok_or("Unsupported language")?;
    let catalog: std::collections::BTreeMap<String, String> =
        serde_json::from_str(source).map_err(|e| e.to_string())?;
    labels
        .open
        .set_text(&catalog["trayOpen"])
        .map_err(|e| e.to_string())?;
    labels
        .quit
        .set_text(&catalog["trayQuit"])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn request(command: Command) -> Result<Response, Error> {
    command.validate()?;
    tauri::async_runtime::spawn_blocking(move || {
        batterybridge_platform::client::query(command, batterybridge_ipc::call)
    })
    .await
    .map_err(|_| Error::PlatformError("IPC worker failed".into()))?
}
#[tauri::command]
fn can_uninstall() -> bool {
    cfg!(target_os = "macos")
        && std::path::Path::new("/Library/PrivilegedHelperTools/io.github.batterybridge.service")
            .exists()
}

#[tauri::command]
async fn uninstall(app: tauri::AppHandle) -> Result<(), String> {
    if !can_uninstall() {
        return Err("The packaged macOS installation was not found.".into());
    }
    #[cfg(target_os = "macos")]
    {
        tauri::async_runtime::spawn_blocking(|| {
            // No paths, commands or script text are accepted from the webview.
            let script = include_str!("../../../../packaging/macos/uninstall.sh");
            let quoted = script
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n");
            let output = std::process::Command::new("/usr/bin/osascript")
                .args([
                    "-e",
                    &format!("do shell script \"{quoted}\" with administrator privileges"),
                ])
                .output()
                .map_err(|e| e.to_string())?;
            if !output.status.success() {
                return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
            }
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())??;
        app.exit(0);
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Err("Graphical uninstall is available only for the macOS package.".into())
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            request,
            set_language,
            can_uninstall,
            uninstall
        ])
        .setup(|app| {
            let open = MenuItem::with_id(app, "open", "Open BatteryBridge", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit BatteryBridge", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;
            app.manage(TrayLabels { open, quit });
            let icon = app
                .default_window_icon()
                .cloned()
                .ok_or("application icon missing")?;
            TrayIconBuilder::new()
                .icon(icon)
                .tooltip("BatteryBridge")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "open" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.unminimize();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!())
        .expect("BatteryBridge GUI failed to start");
}

#[cfg(test)]
mod language_tests {
    #[test]
    fn tray_catalogs_cover_supported_languages_and_reject_unknown_values() {
        for locale in ["en", "ru", "es", "de", "fr", "ja", "pt-BR"] {
            let source = super::language_catalog(locale).expect("catalog missing");
            let catalog: std::collections::BTreeMap<String, String> =
                serde_json::from_str(source).expect("invalid catalog");
            assert!(!catalog["trayOpen"].trim().is_empty());
            assert!(!catalog["trayQuit"].trim().is_empty());
        }
        assert!(super::language_catalog("../en").is_none());
        assert!(super::language_catalog("unsupported").is_none());
    }
}

#[cfg(all(test, feature = "custom-protocol"))]
mod tests {
    #[test]
    fn standalone_gui_embeds_frontend_instead_of_loading_dev_server() {
        assert!(!tauri::is_dev(), "standalone GUI must not load the dev URL");
        let context: tauri::Context<tauri::Wry> = tauri::generate_context!();
        let index = context
            .assets()
            .get(&"index.html".into())
            .expect("embedded index.html missing");
        let index = std::str::from_utf8(&index).expect("index.html must be UTF-8");
        assert!(
            index.contains("/assets/"),
            "frontend must be built before embedding"
        );
    }
}
