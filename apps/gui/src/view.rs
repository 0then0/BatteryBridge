//! Native layout retains BatteryBridge's observatory visual language.
use crate::{App, Reply, i18n, model, platform};
use batterybridge_protocol::{CapabilityState, Command, Snapshot};
use egui::{Align, Color32, FontId, Frame, Layout, RichText, Stroke, pos2, vec2};

#[derive(Clone, Copy)]
struct Palette {
    page: Color32,
    panel: Color32,
    text: Color32,
    muted: Color32,
    line: Color32,
    accent: Color32,
    signal: Color32,
}
impl Palette {
    fn new(dark: bool) -> Self {
        if dark {
            Self {
                page: Color32::BLACK,
                panel: Color32::from_rgb(13, 13, 15),
                text: Color32::from_rgb(216, 234, 255),
                muted: Color32::from_rgb(160, 173, 188),
                line: Color32::from_rgb(64, 68, 74),
                accent: Color32::from_rgb(31, 88, 242),
                signal: Color32::from_rgb(255, 65, 5),
            }
        } else {
            Self {
                page: Color32::from_rgb(232, 237, 244),
                panel: Color32::from_rgb(245, 247, 250),
                text: Color32::from_rgb(21, 36, 58),
                muted: Color32::from_rgb(82, 97, 118),
                line: Color32::from_rgb(198, 205, 215),
                accent: Color32::from_rgb(31, 88, 242),
                signal: Color32::from_rgb(196, 56, 0),
            }
        }
    }
}
pub fn style(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::System);
    ctx.all_styles_mut(|style| {
        let p = Palette::new(style.visuals.dark_mode);
        style.visuals.panel_fill = p.panel;
        style.visuals.window_fill = p.panel;
        style.visuals.extreme_bg_color = p.page;
        style.visuals.override_text_color = Some(p.text);
        style.visuals.selection.bg_fill = p.accent;
        style.visuals.selection.stroke = Stroke::new(1.0, Color32::WHITE);
        style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.line);
        style.visuals.widgets.inactive.bg_fill = p.panel;
        style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, p.muted);
        style.visuals.widgets.hovered.bg_fill = p.page;
        style.spacing.item_spacing = vec2(10.0, 12.0);
        style.spacing.button_padding = vec2(12.0, 10.0);
        style.spacing.interact_size.y = 40.0;
        style
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(14.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(14.0));
    });
}
fn note(ui: &mut egui::Ui, title: Option<String>, body: String, error: bool) {
    let p = Palette::new(ui.visuals().dark_mode);
    Frame::new()
        .fill(p.page)
        .stroke(Stroke::new(1.0, if error { p.signal } else { p.line }))
        .corner_radius(8)
        .inner_margin(14)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if let Some(title) = title {
                ui.label(RichText::new(title).strong());
            }
            ui.label(RichText::new(body).color(if error { p.signal } else { p.muted }));
        });
    ui.add_space(10.0);
}
fn eyebrow(ui: &mut egui::Ui, text: String) {
    ui.label(
        RichText::new(text.to_uppercase())
            .monospace()
            .size(10.0)
            .color(Palette::new(ui.visuals().dark_mode).muted),
    );
}
fn button(ui: &mut egui::Ui, text: String, enabled: bool) -> egui::Response {
    ui.add_enabled(
        enabled,
        egui::Button::new(text)
            .wrap()
            .min_size(vec2(ui.available_width(), 42.0)),
    )
}
fn status_card(ui: &mut egui::Ui, locale: &str, snapshot: Option<&Snapshot>) {
    let p = Palette::new(ui.visuals().dark_mode);
    let battery = snapshot.and_then(|s| s.batteries.first());
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), 230.0), egui::Sense::hover());
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_stroke(
        rect.shrink(0.5),
        16.0,
        Stroke::new(1.0, p.line),
        egui::StrokeKind::Inside,
    );
    let center = pos2(rect.right() - 75.0, rect.center().y);
    let radius = 98.0;
    let orbit = Stroke::new(0.7, p.line.gamma_multiply(0.65));
    painter.circle_stroke(center, radius, orbit);
    for squash in [0.38, 0.75] {
        let points = (0..=64)
            .map(|i| {
                let a = i as f32 * std::f32::consts::TAU / 64.0;
                center + vec2(a.cos() * radius * squash, a.sin() * radius)
            })
            .collect();
        painter.add(egui::Shape::line(points, orbit));
    }
    let points = (0..=64)
        .map(|i| {
            let a = i as f32 * std::f32::consts::TAU / 64.0;
            center + vec2(a.cos() * radius, a.sin() * radius * 0.38)
        })
        .collect();
    painter.add(egui::Shape::line(points, orbit));
    for a in [-1.57_f32, -2.35, 0.78, 2.8] {
        painter.circle_filled(center + vec2(a.cos(), a.sin()) * radius, 3.0, p.accent);
    }
    let left = rect.left() + 20.0;
    painter.text(
        pos2(left, rect.top() + 22.0),
        egui::Align2::LEFT_TOP,
        i18n::text(locale, "batteryStatus").to_uppercase(),
        FontId::monospace(10.0),
        p.muted,
    );
    let percent = battery
        .and_then(|b| b.percent)
        .map(|v| format!("{v}%"))
        .unwrap_or_else(|| "—%".into());
    painter.text(
        pos2(left, rect.top() + 58.0),
        egui::Align2::LEFT_TOP,
        &percent,
        FontId::proportional(((rect.width() - 40.0) / 3.0).clamp(62.0, 88.0)),
        p.text,
    );
    let bar = egui::Rect::from_min_size(
        pos2(left, rect.bottom() - 56.0),
        vec2(rect.width() - 40.0, 5.0),
    );
    painter.rect_stroke(
        bar,
        0.0,
        Stroke::new(1.0, p.muted),
        egui::StrokeKind::Inside,
    );
    if let Some(percent) = battery.and_then(|b| b.percent) {
        painter.rect_filled(
            egui::Rect::from_min_size(bar.min, vec2(bar.width() * f32::from(percent) / 100.0, 5.0)),
            0.0,
            p.text,
        );
    }
    let state = battery
        .map(|b| i18n::battery_state(locale, &b.state))
        .unwrap_or_else(|| {
            i18n::text(
                locale,
                if snapshot.is_some() {
                    "noBattery"
                } else {
                    "connecting"
                },
            )
        });
    let power = battery
        .and_then(|b| b.on_ac)
        .map(|ac| i18n::text(locale, if ac { "ac" } else { "onBattery" }));
    let stacked = power.as_ref().is_some_and(|power| {
        let state_width = painter
            .layout_no_wrap(state.clone(), FontId::proportional(12.0), p.text)
            .size()
            .x;
        let power_width = painter
            .layout_no_wrap(power.clone(), FontId::proportional(11.0), p.muted)
            .size()
            .x;
        state_width + power_width + 16.0 > rect.width() - 40.0
    });
    painter.text(
        pos2(left, rect.bottom() - if stacked { 35.0 } else { 30.0 }),
        egui::Align2::LEFT_CENTER,
        &state,
        FontId::proportional(12.0),
        p.text,
    );
    if let Some(power) = power {
        painter.text(
            if stacked {
                pos2(left, rect.bottom() - 17.0)
            } else {
                pos2(rect.right() - 20.0, rect.bottom() - 30.0)
            },
            if stacked {
                egui::Align2::LEFT_CENTER
            } else {
                egui::Align2::RIGHT_CENTER
            },
            power,
            FontId::proportional(11.0),
            p.muted,
        );
    }
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Image,
            true,
            format!(
                "{}: {percent}, {state}",
                i18n::text(locale, "batteryStatus")
            ),
        )
    });
    ui.add_space(22.0);
}
impl App {
    pub fn render(&mut self, outer: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(outer, |ui| {
                let width = (ui.available_width() - 32.0).clamp(240.0, 520.0);
                ui.with_layout(Layout::top_down(Align::Center), |ui| {
                    ui.allocate_ui_with_layout(
                        vec2(width, 0.0),
                        Layout::top_down(Align::Min),
                        |ui| {
                            ui.set_width(width);
                            ui.add_space(18.0);
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(RichText::new("BatteryBridge").size(24.0).strong());
                                    eyebrow(ui, self.text("tagline"));
                                });
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    let refresh = ui
                                        .add_enabled(
                                            !self.busy && !self.preview,
                                            egui::Button::new(RichText::new("↻").size(25.0)),
                                        )
                                        .on_hover_text(self.text("refresh"));
                                    refresh.widget_info(|| {
                                        egui::WidgetInfo::labeled(
                                            egui::WidgetType::Button,
                                            refresh.enabled(),
                                            self.text("refresh"),
                                        )
                                    });
                                    if refresh.clicked() {
                                        self.request(Command::Status);
                                    }
                                });
                            });
                            ui.add_space(14.0);
                            ui.separator();
                            ui.add_space(16.0);
                            status_card(ui, &self.locale, self.model.snapshot.as_ref());
                            self.notices(ui);
                            self.policy(ui);
                            if let Some(snapshot) = &self.model.snapshot {
                                let snapshot = snapshot.clone();
                                Frame::new()
                                    .fill(Palette::new(ui.visuals().dark_mode).page)
                                    .inner_margin(16)
                                    .corner_radius(10)
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        eyebrow(ui, self.text("backend"));
                                        ui.label(RichText::new(&snapshot.backend).size(20.0));
                                        ui.label(format!(
                                            "{} {}",
                                            snapshot.device.vendor, snapshot.device.model
                                        ));
                                    });
                                ui.add_space(12.0);
                                egui::CollapsingHeader::new(self.text("capabilities")).show(
                                    ui,
                                    |ui| {
                                        for (key, cap) in [
                                            ("chargeLimit", &snapshot.capabilities.charge_limit),
                                            (
                                                "startThreshold",
                                                &snapshot.capabilities.start_threshold,
                                            ),
                                            (
                                                "stopThreshold",
                                                &snapshot.capabilities.stop_threshold,
                                            ),
                                            (
                                                "forceDischarge",
                                                &snapshot.capabilities.force_discharge,
                                            ),
                                            (
                                                "adapterDisable",
                                                &snapshot.capabilities.adapter_disable,
                                            ),
                                            (
                                                "topUpFeature",
                                                &snapshot.capabilities.temporary_full_charge,
                                            ),
                                        ] {
                                            ui.separator();
                                            ui.label(RichText::new(self.text(key)).strong());
                                            let state = match cap.state {
                                                CapabilityState::Supported => "supported",
                                                CapabilityState::Experimental => "experimental",
                                                CapabilityState::Unsupported => "unsupported",
                                                CapabilityState::Unknown => "unknown",
                                            };
                                            eyebrow(ui, self.text(state));
                                            ui.label(i18n::reason(&self.locale, &cap.reason));
                                        }
                                    },
                                );
                                if button(
                                    ui,
                                    self.text("reportButton"),
                                    !self.busy && !self.preview,
                                )
                                .clicked()
                                {
                                    self.request(Command::Probe);
                                }
                            }
                            self.report_view(ui);
                            ui.add_space(16.0);
                            ui.separator();
                            let label = ui.label(self.text("language"));
                            let before = self.language.clone();
                            egui::ComboBox::from_id_salt("language")
                                .width(ui.available_width())
                                .selected_text(if self.language == "system" {
                                    self.text("systemLanguage")
                                } else {
                                    i18n::LANGUAGES
                                        .iter()
                                        .find(|(code, _)| *code == self.language)
                                        .map_or("English", |(_, name)| *name)
                                        .into()
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.language,
                                        "system".into(),
                                        i18n::text(&self.locale, "systemLanguage"),
                                    );
                                    for (code, name) in i18n::LANGUAGES {
                                        ui.selectable_value(&mut self.language, code.into(), name);
                                    }
                                })
                                .response
                                .labelled_by(label.id);
                            if before != self.language {
                                self.language_changed();
                            }
                            if platform::can_uninstall() && !self.preview {
                                ui.add_space(16.0);
                                ui.separator();
                                if self.confirm_uninstall {
                                    ui.label(self.text("uninstallConfirm"));
                                    if button(ui, self.text("uninstall"), !self.busy).clicked() {
                                        self.spawn(|| Reply::Uninstalled(platform::uninstall()));
                                    }
                                    if button(ui, self.text("cancel"), !self.busy).clicked() {
                                        self.confirm_uninstall = false;
                                    }
                                } else if button(ui, self.text("uninstall"), !self.busy).clicked() {
                                    self.confirm_uninstall = true;
                                }
                            }
                            ui.add_space(18.0);
                            eyebrow(ui, self.text("footer"));
                            ui.add_space(24.0);
                        },
                    );
                });
            });
    }
    fn notices(&self, ui: &mut egui::Ui) {
        if let Some(error) = &self.model.error {
            note(ui, Some(self.text("requestTitle")), error.clone(), true);
        }
        if let Some(snapshot) = &self.model.snapshot {
            if let Some(error) = &snapshot.last_error {
                note(ui, Some(self.text("policyError")), error.to_string(), true);
            }
            if snapshot.config.is_none() {
                note(
                    ui,
                    Some(self.text("readOnly")),
                    self.text("readOnlyBody"),
                    false,
                );
            }
            if snapshot.capabilities.charge_limit.state == CapabilityState::Experimental {
                note(ui, None, self.text("experimentalNotice"), false);
            }
            if model::charging_above_limit(snapshot) {
                note(ui, None, self.text("limitExceeded"), true);
            }
            if ["Apple Silicon SMC", "Intel SMC"].contains(&snapshot.backend.as_str()) {
                note(ui, None, self.text("sleepNotice"), false);
            }
        }
        if self.model.updated {
            note(ui, None, self.text("updated"), false);
        }
    }
    fn policy(&mut self, ui: &mut egui::Ui) {
        let Some(snapshot) = self.model.snapshot.clone() else {
            return;
        };
        let caps = &snapshot.capabilities;
        if !caps.charge_limit.available() {
            note(
                ui,
                Some(self.text("limitUnavailable")),
                i18n::reason(&self.locale, &caps.charge_limit.reason),
                false,
            );
            return;
        }
        ui.separator();
        ui.add_space(14.0);
        ui.label(RichText::new(self.text("chargeLimit")).size(24.0));
        let state = if snapshot.config.is_none() {
            self.text("settingsUnavailable")
        } else if snapshot.top_up_active {
            self.text("topUpActive")
        } else if let Some(stop) = snapshot.config.as_ref().and_then(|c| c.charge_limit) {
            self.text("activeLimit")
                .replace("{percent}", &stop.to_string())
        } else {
            self.text("off")
        };
        eyebrow(ui, state);
        ui.label(self.text("firstBattery"));
        ui.add_space(8.0);
        let enabled =
            !self.busy && !self.preview && !self.model.read_only() && self.model.error.is_none();
        ui.add_enabled_ui(enabled, |ui| {
            let label = ui.label(self.text("stopAt"));
            if let Some(limits) = &caps.allowed_limits {
                egui::ComboBox::from_id_salt("stop")
                    .selected_text(format!("{}%", self.model.stop))
                    .show_ui(ui, |ui| {
                        for value in limits {
                            if ui
                                .selectable_value(&mut self.model.stop, *value, format!("{value}%"))
                                .changed()
                            {
                                self.model.edited = true;
                            }
                        }
                    })
                    .response
                    .labelled_by(label.id);
            } else if ui
                .add(
                    egui::DragValue::new(&mut self.model.stop)
                        .range(1..=100)
                        .suffix("%"),
                )
                .labelled_by(label.id)
                .changed()
            {
                self.model.edited = true;
            }
            if caps.start_threshold.available() {
                if ui
                    .checkbox(
                        &mut self.model.use_start,
                        i18n::text(&self.locale, "useStart"),
                    )
                    .changed()
                {
                    self.model.edited = true;
                }
                if self.model.use_start {
                    let label = ui.label(self.text("resumeAt"));
                    if ui
                        .add(
                            egui::DragValue::new(&mut self.model.start)
                                .range(0..=99)
                                .suffix("%"),
                        )
                        .labelled_by(label.id)
                        .changed()
                    {
                        self.model.edited = true;
                    }
                }
            }
            if button(ui, self.text("apply"), !snapshot.batteries.is_empty()).clicked() {
                match self.model.limit_command() {
                    Ok(command) => self.request(command),
                    Err(error) => self.model.error = Some(error.to_string()),
                }
            }
            if button(ui, self.text("turnOff"), true).clicked() {
                self.request(Command::Limit {
                    stop: None,
                    start: None,
                });
            }
            if caps.temporary_full_charge.available() {
                if button(
                    ui,
                    self.text("topUp"),
                    snapshot
                        .config
                        .as_ref()
                        .is_some_and(|c| c.charge_limit.is_some())
                        && !snapshot.top_up_active
                        && !snapshot.batteries.is_empty(),
                )
                .clicked()
                {
                    self.request(Command::TopUp);
                }
            } else {
                ui.label(format!(
                    "{}: {}",
                    self.text("topUpUnavailable"),
                    i18n::reason(&self.locale, &caps.temporary_full_charge.reason)
                ));
            }
        });
        ui.add_space(18.0);
    }
    fn report_view(&mut self, ui: &mut egui::Ui) {
        if let Some(report) = &mut self.report {
            let locale = &self.locale;
            ui.add_space(16.0);
            ui.heading(i18n::text(locale, "reportTitle"));
            ui.label(i18n::text(locale, "reportPrivacy"));
            let label = ui.label(i18n::text(locale, "reportJson"));
            ui.add(
                egui::TextEdit::multiline(&mut report.as_str())
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(10)
                    .desired_width(f32::INFINITY),
            )
            .labelled_by(label.id);
            if ui.button(i18n::text(locale, "copyReport")).clicked() {
                ui.ctx().copy_text(report.clone());
            }
            let label = ui.label(i18n::text(locale, "reportDestination"));
            if ui
                .add(
                    egui::TextEdit::singleline(&mut self.report_destination)
                        .desired_width(f32::INFINITY),
                )
                .labelled_by(label.id)
                .changed()
            {
                self.report_saved = false;
            }
            if self.report_saved {
                ui.label(i18n::text(locale, "reportSaved"));
            }
            if button(
                ui,
                i18n::text(locale, "saveReport"),
                !self.busy && !self.report_saved,
            )
            .clicked()
            {
                let report = report.clone();
                let path = self.report_destination.clone();
                self.spawn(move || {
                    Reply::Saved(platform::save_report(std::path::Path::new(&path), &report))
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Options, model::tests::fixture};
    use egui::{RawInput, Rect, ThemePreference};
    use fastframe_shell::Waker;

    #[test]
    fn catalogs_render_at_supported_widths_without_horizontal_overflow() {
        let ctx = egui::Context::default();
        fastframe_fonts::FontSetup::default().install(&ctx);
        style(&ctx);
        let mut app = App::new(
            Waker::default(),
            Options {
                preview: Some(
                    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/status.json"),
                ),
                language: None,
            },
        )
        .unwrap();
        for dark in [false, true] {
            ctx.set_theme(if dark {
                ThemePreference::Dark
            } else {
                ThemePreference::Light
            });
            for width in [320.0, 420.0, 720.0] {
                for (locale, _) in i18n::LANGUAGES {
                    app.locale = locale.into();
                    for offline in [false, true] {
                        let mut snapshot = fixture();
                        if offline {
                            snapshot.config = None;
                        }
                        app.model.accept(snapshot);
                        let input = RawInput {
                            screen_rect: Some(Rect::from_min_size(
                                egui::Pos2::ZERO,
                                vec2(width, 730.0),
                            )),
                            ..Default::default()
                        };
                        let mut output = ctx.run_ui(input, |root| {
                            egui::CentralPanel::default().show(root, |ui| {
                                app.render(ui);
                                assert!(
                                    ui.min_rect().width() <= width + 1.0,
                                    "{locale}, width {width}, bounds {:?}",
                                    ui.min_rect()
                                );
                            });
                        });
                        output.textures_delta.clear();
                    }
                }
            }
        }
    }
    #[test]
    fn installed_fallback_fonts_cover_every_supported_language() {
        let ctx = egui::Context::default();
        fastframe_fonts::FontSetup::default().install(&ctx);
        ctx.begin_pass(RawInput::default());
        {
            for (locale, catalog) in i18n::catalogs() {
                for (key, text) in catalog {
                    for c in text.chars().filter(|c| !c.is_whitespace()) {
                        assert!(
                            ctx.fonts_mut(|fonts| fonts.has_glyph(&FontId::proportional(14.0), c)),
                            "missing {c:?} in {locale}.{key}"
                        );
                    }
                }
            }
        }
        let mut output = ctx.end_pass();
        output.textures_delta.clear();
    }
}
