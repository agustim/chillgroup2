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

            // On Linux/WebKitGTK: apply WebRTC settings + RTCRtpScriptTransform feature, then
            // reload the hidden main window so those settings take effect for the document.
            // Only after the reload completes do we show the window (no visible flash).
            // On other platforms: show directly.
            let configured = server_url_is_configured(app.handle());

            #[cfg(target_os = "linux")]
            {
                let on_ready: Option<Box<dyn Fn() + Send + 'static>> = if configured {
                    let h = app.handle().clone();
                    Some(Box::new(move || show_main_window(&h)))
                } else {
                    None
                };
                linux_init_webrtc_and_show(app.handle(), on_ready);
            }

            if !configured {
                create_setup_window(app.handle())?;
            } else {
                #[cfg(not(target_os = "linux"))]
                show_main_window(app.handle());
            }

            // Listen for setup-complete event from the setup webview
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

// Apply WebRTC settings to the hidden main window, reload it so RTCRtpScriptTransform is
// exposed in the reloaded document, then call on_ready (shows the window if configured).
// Uses dlsym for the WebKitGTK 2.42+ Feature API so CI (Ubuntu 22.04 / WebKitGTK 2.36)
// still links cleanly — the feature enabling just silently no-ops on older versions.
#[cfg(target_os = "linux")]
fn linux_init_webrtc_and_show(
    app: &AppHandle,
    on_ready: Option<Box<dyn Fn() + Send + 'static>>,
) {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    };
    use webkit2gtk::{LoadEvent, SettingsExt, WebViewExt};

    let Some(win) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        // Window not yet created — fall back
        if let Some(f) = on_ready {
            f();
        }
        return;
    };

    let on_ready_cell: Arc<Mutex<Option<Box<dyn Fn() + Send + 'static>>>> =
        Arc::new(Mutex::new(on_ready));

    let result = win.with_webview(move |wv| {
        let view = wv.inner();

        // 1. Enable WebRTC + media APIs
        if let Some(settings) = WebViewExt::settings(&view) {
            settings.set_enable_media_stream(true);
            settings.set_enable_webrtc(true);
            settings.set_enable_encrypted_media(true);
            unsafe { try_enable_rtc_script_transform_feature(&settings) };
        }

        // 2. After the upcoming reload finishes, call on_ready (shows the window).
        //    The flag is set to true right before reload() so the initial-load Finished
        //    event (which may fire before we connect) is ignored.
        let on_ready_ref = on_ready_cell.clone();
        let reload_started = Arc::new(AtomicBool::new(false));
        let reload_started_ref = reload_started.clone();

        view.connect_load_changed(move |_, event| {
            if reload_started_ref.load(Ordering::Relaxed) && event == LoadEvent::Finished {
                if let Ok(mut guard) = on_ready_ref.lock() {
                    if let Some(f) = guard.take() {
                        f();
                    }
                }
            }
        });

        // 3. Reload so the new document sees updated settings
        reload_started.store(true, Ordering::Relaxed);
        view.reload();
    });

    if result.is_err() {
        // with_webview failed (e.g., window not realised) — show directly
        if let Some(w) = app.get_webview_window(MAIN_WINDOW_LABEL) {
            w.show().ok();
            w.set_focus().ok();
        }
    }
}

// Enable RTCRtpScriptTransform via WebKitGTK 2.42+ Feature API.
// All symbols resolved at runtime via dlsym — safe on older WebKitGTK.
#[cfg(target_os = "linux")]
unsafe fn try_enable_rtc_script_transform_feature(settings: &webkit2gtk::Settings) {
    use std::ffi::CStr;
    use webkit2gtk::glib::translate::ToGlibPtr;

    #[repr(C)]
    struct WKFeatureList([u8; 0]);
    #[repr(C)]
    struct WKFeature([u8; 0]);

    type FnGetAll = unsafe extern "C" fn() -> *mut WKFeatureList;
    type FnLen = unsafe extern "C" fn(*mut WKFeatureList) -> libc::size_t;
    type FnGet = unsafe extern "C" fn(*mut WKFeatureList, libc::size_t) -> *mut WKFeature;
    type FnId = unsafe extern "C" fn(*const WKFeature) -> *const libc::c_char;
    type FnEnable =
        unsafe extern "C" fn(*mut webkit2gtk::ffi::WebKitSettings, *const WKFeature, libc::c_int);
    type FnUnref = unsafe extern "C" fn(*mut WKFeatureList);

    macro_rules! load {
        ($sym:expr, $ty:ty) => {{
            let ptr = libc::dlsym(libc::RTLD_DEFAULT, $sym.as_ptr() as *const libc::c_char);
            if ptr.is_null() {
                eprintln!("[ChillGroup] WebKitGTK Feature API not found ({}), skipping RTCRtpScriptTransform enable", stringify!($sym));
                return;
            }
            std::mem::transmute::<*mut libc::c_void, $ty>(ptr)
        }};
    }

    let get_all: FnGetAll = load!(b"webkit_settings_get_all_features\0", FnGetAll);
    let list_len: FnLen = load!(b"webkit_feature_list_get_length\0", FnLen);
    let list_get: FnGet = load!(b"webkit_feature_list_get\0", FnGet);
    let get_id: FnId = load!(b"webkit_feature_get_identifier\0", FnId);
    let set_enabled: FnEnable = load!(b"webkit_settings_set_feature_enabled\0", FnEnable);
    let list_unref: FnUnref = load!(b"webkit_feature_list_unref\0", FnUnref);

    let list = get_all();
    if list.is_null() {
        return;
    }



    let settings_ptr: *mut webkit2gtk::ffi::WebKitSettings = settings.to_glib_none().0;
    let len = list_len(list);

    for i in 0..len {
        let feat = list_get(list, i);
        if feat.is_null() {
            continue;
        }
        let id_ptr = get_id(feat);
        if id_ptr.is_null() {
            continue;
        }
        let id = CStr::from_ptr(id_ptr).to_string_lossy();

        // Enable any feature that could expose RTCRtpScriptTransform or the older
        // insertable-streams API (RTCRtpSender.prototype.createEncodedStreams).
        // NOTE: WebKitGTK with GStreamer WebRTC backend does not implement either API —
        // these enables are best-effort and have no effect on current WebKitGTK builds.
        let should_enable = id.contains("RTCRtpScriptTransform")
            || id.contains("WebRTCEncoded")
            || id.contains("RTCRtpSend")
            || id == "RTCEncodedStreamsQuirk"
            || id.to_lowercase().contains("insertable");

        if should_enable {
            set_enabled(settings_ptr, feat, 1);
        }
    }

    list_unref(list);
}
