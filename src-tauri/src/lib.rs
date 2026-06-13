use std::path::PathBuf;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Listener, Manager, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_store::StoreExt;

const STORE_KEY_SERVER_URL: &str = "server_url";
const SETUP_WINDOW_LABEL: &str = "setup";
const MAIN_WINDOW_LABEL: &str = "main";

#[tauri::command]
fn get_server_url(app: AppHandle) -> Result<String, String> {
    let store = app
        .store("config.json")
        .map_err(|e| e.to_string())?;
    match store.get(STORE_KEY_SERVER_URL) {
        Some(v) => v
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "invalid url in store".to_string()),
        None => Err("not configured".to_string()),
    }
}

#[tauri::command]
fn set_server_url(app: AppHandle, url: String) -> Result<(), String> {
    let store = app
        .store("config.json")
        .map_err(|e| e.to_string())?;
    store.set(STORE_KEY_SERVER_URL, serde_json::Value::String(url));
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn open_setup_window(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window(SETUP_WINDOW_LABEL) {
        w.show().ok();
        w.set_focus().ok();
        return Ok(());
    }
    create_setup_window(&app).map_err(|e| e.to_string())?;
    Ok(())
}

fn create_setup_window(app: &AppHandle) -> tauri::Result<()> {
    // setup.html lives in frontend/public/ and is served by Tauri's asset protocol
    WebviewWindowBuilder::new(
        app,
        SETUP_WINDOW_LABEL,
        WebviewUrl::App(PathBuf::from("setup.html")),
    )
    .title("Configurar ChillGroup")
    .inner_size(440.0, 300.0)
    .resizable(false)
    .center()
    .build()?;
    Ok(())
}

fn server_url_is_configured(app: &AppHandle) -> bool {
    app.store("config.json")
        .ok()
        .and_then(|s| s.get(STORE_KEY_SERVER_URL))
        .and_then(|v| v.as_str().map(|s| !s.is_empty()))
        .unwrap_or(false)
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            get_server_url,
            set_server_url,
            open_setup_window,
        ])
        .setup(|app| {
            // Build tray menu
            let open_i = MenuItem::with_id(app, "open", "Obrir ChillGroup", true, None::<&str>)?;
            let server_i = MenuItem::with_id(app, "server", "Canviar servidor", true, None::<&str>)?;
            let sep = tauri::menu::PredefinedMenuItem::separator(app)?;
            let quit_i = MenuItem::with_id(app, "quit", "Sortir", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open_i, &server_i, &sep, &quit_i])?;

            TrayIconBuilder::new()
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show_main_window(app),
                    "server" => {
                        let _ = open_setup_window(app.clone());
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main_window(tray.app_handle());
                    }
                })
                .build(app)?;

            // First run: show setup window if no URL configured
            if !server_url_is_configured(app.handle()) {
                create_setup_window(app.handle())?;
            } else {
                show_main_window(app.handle());
            }

            // Listen for setup-complete event emitted by the setup webview JS
            let handle = app.handle().clone();
            app.listen("setup-complete", move |_event| {
                if let Some(w) = handle.get_webview_window(SETUP_WINDOW_LABEL) {
                    w.close().ok();
                }
                show_main_window(&handle);
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            // Minimize to tray instead of closing
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == MAIN_WINDOW_LABEL {
                    api.prevent_close();
                    window.hide().ok();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running ChillGroup desktop");
}

fn show_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        w.show().ok();
        w.set_focus().ok();
    }
}
