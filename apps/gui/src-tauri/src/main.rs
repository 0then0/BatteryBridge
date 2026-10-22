#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use batterybridge_protocol::{Command, Error, Response};
use tauri::{
    Manager,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};

#[tauri::command]
async fn request(command: Command) -> Result<Response, Error> {
    command.validate()?;
    tauri::async_runtime::spawn_blocking(move || {
        batterybridge_platform::client::query(command, batterybridge_ipc::call)
    })
    .await
    .map_err(|_| Error::PlatformError("IPC worker failed".into()))?
}
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![request])
        .setup(|app| {
            let open = MenuItem::with_id(app, "open", "Open BatteryBridge", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit BatteryBridge", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;
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
