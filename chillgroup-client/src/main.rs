mod api;
mod realtime;
mod settings;
mod storage;

use slint::{Model, ModelRc, VecModel};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

slint::include_modules!();

#[derive(Debug, Default)]
struct Session {
    token: String,
    user_id: String,
    username: String,
    device_id: String,
    active_server_id: String,
    active_channel_id: String,
    active_channel_type: String,
}

#[derive(Debug)]
enum Cmd {
    Login { server_url: String, username: String, password: String },
    SelectServer { server_id: String },
    SelectChannel { channel_id: String, channel_type: String },
    SendMessage { content: String },
}

fn initial(s: &str) -> String {
    s.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "?".to_string())
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("chillgroup_client=debug")),
        )
        .init();

    let cfg = settings::load();

    let vault = Arc::new(Mutex::new(
        storage::Vault::open(&cfg.vault.path).expect("Cannot open vault"),
    ));

    let (cmd_tx, mut cmd_rx) = mpsc::channel::<Cmd>(32);
    let (event_tx, mut event_rx) = mpsc::channel::<realtime::RealtimeEvent>(64);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    // Login window
    let login_win = LoginWindow::new().unwrap();
    login_win.set_server_url(cfg.server.url.clone().into());

    login_win.on_login({
        let cmd_tx = cmd_tx.clone();
        let handle = login_win.as_weak();
        move || {
            let win = handle.unwrap();
            let cmd = Cmd::Login {
                server_url: win.get_server_url().to_string(),
                username: win.get_username().to_string(),
                password: win.get_password().to_string(),
            };
            win.set_loading(true);
            win.set_error_message("".into());
            let _ = cmd_tx.try_send(cmd);
        }
    });

    login_win.on_open_settings({
        let cfg_ref = cfg.clone();
        move || open_settings(cfg_ref.clone(), None)
    });

    // Main window (hidden until login)
    let main_win = MainWindow::new().unwrap();
    main_win.set_servers(ModelRc::new(VecModel::default()));
    main_win.set_channels(ModelRc::new(VecModel::default()));
    main_win.set_messages(ModelRc::new(VecModel::default()));

    main_win.on_select_server({
        let cmd_tx = cmd_tx.clone();
        move |server_id| {
            let _ = cmd_tx.try_send(Cmd::SelectServer { server_id: server_id.to_string() });
        }
    });

    main_win.on_select_channel({
        let cmd_tx = cmd_tx.clone();
        move |channel_id, channel_type| {
            let _ = cmd_tx.try_send(Cmd::SelectChannel {
                channel_id: channel_id.to_string(),
                channel_type: channel_type.to_string(),
            });
        }
    });

    main_win.on_send_message({
        let cmd_tx = cmd_tx.clone();
        move |content| {
            let _ = cmd_tx.try_send(Cmd::SendMessage { content: content.to_string() });
        }
    });

    main_win.on_open_settings({
        let cfg_ref = cfg.clone();
        move || open_settings(cfg_ref.clone(), None)
    });
    main_win.on_join_voice(|| { /* Phase 2 */ });
    main_win.on_leave_voice(|| { /* Phase 2 */ });
    main_win.on_toggle_mute(|| { /* Phase 2 */ });
    main_win.on_toggle_deafen(|| { /* Phase 2 */ });

    // Backend task
    let login_h_bg = login_win.as_weak();
    let main_h_bg = main_win.as_weak();
    let vault_bg = Arc::clone(&vault);
    let cfg_bg = cfg.clone();

    rt.spawn(async move {
        let mut api: Option<api::ApiClient> = None;
        let mut socket: Option<rust_socketio::asynchronous::Client> = None;
        let mut session = Session::default();

        loop {
            tokio::select! {
                Some(cmd) = cmd_rx.recv() => {
                    match cmd {
                        Cmd::Login { server_url, username, password } => {
                            let client = api::ApiClient::new(&server_url);
                            match api::auth::login(&client, &username, &password).await {
                                Ok(data) => {
                                    let client = client.with_token(&data.token);
                                    session.token = data.token.clone();
                                    session.user_id = data.user_id.clone();
                                    session.username = data.username.clone();
                                    session.device_id = data.device_id.clone();

                                    if let Ok(v) = vault_bg.lock() {
                                        let _ = v.save_session(&data.token, &data.user_id, &data.username, &data.device_id);
                                    }

                                    let mut new_cfg = cfg_bg.clone();
                                    new_cfg.server.url = server_url.clone();
                                    let _ = settings::save(&new_cfg);

                                    match realtime::connect(&server_url, &data.token, event_tx.clone()).await {
                                        Ok(sock) => { socket = Some(sock); }
                                        Err(e) => tracing::warn!("Socket.IO: {e}"),
                                    }

                                    api = Some(client);

                                    match api::servers::list(api.as_ref().unwrap()).await {
                                        Ok(servers) => {
                                            let username_clone = data.username.clone();
                                            let server_items: Vec<ServerItem> = servers.iter().map(|s| ServerItem {
                                                id: s.server_id.clone().into(),
                                                name: s.name.clone().into(),
                                                initial: initial(&s.name).into(),
                                            }).collect();
                                            let first_id = servers.first().map(|s| s.server_id.clone());

                                            let lh = login_h_bg.clone();
                                            let mh = main_h_bg.clone();
                                            slint::invoke_from_event_loop(move || {
                                                // Mostrar main PRIMER, després amagar login
                                                // (si s'amaga sense cap finestra visible, l'event loop acaba)
                                                let win = mh.unwrap();
                                                win.set_current_username(username_clone.clone().into());
                                                win.set_current_username_initial(initial(&username_clone).into());
                                                win.set_servers(ModelRc::new(VecModel::from(server_items)));
                                                win.set_status_text("Connectat".into());
                                                win.show().ok();
                                                lh.unwrap().hide().ok();
                                            }).ok();

                                            if let Some(sid) = first_id {
                                                let _ = cmd_tx.try_send(Cmd::SelectServer { server_id: sid });
                                            }
                                        }
                                        Err(e) => {
                                            let msg = format!("Error carregant servidors: {e}");
                                            let lh = login_h_bg.clone();
                                            slint::invoke_from_event_loop(move || {
                                                let win = lh.unwrap();
                                                win.set_loading(false);
                                                win.set_error_message(msg.into());
                                            }).ok();
                                        }
                                    }
                                }
                                Err(e) => {
                                    let msg = match e {
                                        api::ApiError::Unauthorized => "Usuari o contrasenya incorrectes".to_string(),
                                        other => format!("Error: {other}"),
                                    };
                                    let lh = login_h_bg.clone();
                                    slint::invoke_from_event_loop(move || {
                                        let win = lh.unwrap();
                                        win.set_loading(false);
                                        win.set_error_message(msg.into());
                                    }).ok();
                                }
                            }
                        }

                        Cmd::SelectServer { server_id } => {
                            if let Some(client) = &api {
                                session.active_server_id = server_id.clone();
                                match api::channels::list(client, &server_id).await {
                                    Ok(channels) => {
                                        let items: Vec<ChannelItem> = channels.iter().map(|c| ChannelItem {
                                            id: c.id.clone().into(),
                                            name: c.name.clone().into(),
                                            channel_type: c.channel_type.as_str().into(),
                                            unread: c.unread_count.map(|n| n > 0).unwrap_or(false),
                                        }).collect();
                                        let mh = main_h_bg.clone();
                                        slint::invoke_from_event_loop(move || {
                                            let win = mh.unwrap();
                                            win.set_channels(ModelRc::new(VecModel::from(items)));
                                            win.set_active_server_id(server_id.into());
                                            win.set_active_channel_id("".into());
                                            win.set_active_channel_name("".into());
                                            win.set_messages(ModelRc::new(VecModel::default()));
                                        }).ok();
                                    }
                                    Err(e) => tracing::warn!("Error loading channels: {e}"),
                                }
                            }
                        }

                        Cmd::SelectChannel { channel_id, channel_type } => {
                            if let Some(client) = &api {
                                if !session.active_channel_id.is_empty() {
                                    if let Some(sock) = &socket {
                                        let _ = realtime::leave_channel(sock, &session.active_channel_id).await;
                                    }
                                }
                                session.active_channel_id = channel_id.clone();
                                session.active_channel_type = channel_type.clone();

                                if let Some(sock) = &socket {
                                    let _ = realtime::join_channel(sock, &channel_id).await;
                                }

                                let ch_id = channel_id.clone();
                                let ch_type = channel_type.clone();
                                let mh = main_h_bg.clone();
                                slint::invoke_from_event_loop(move || {
                                    let win = mh.unwrap();
                                    let channels = win.get_channels();
                                    for i in 0..channels.row_count() {
                                        if let Some(ch) = channels.row_data(i) {
                                            if ch.id == ch_id.as_str() {
                                                win.set_active_channel_name(ch.name);
                                                break;
                                            }
                                        }
                                    }
                                    win.set_active_channel_id(ch_id.into());
                                    win.set_active_channel_type(ch_type.into());
                                    win.set_messages(ModelRc::new(VecModel::default()));
                                }).ok();

                                if channel_type == "text" {
                                    match api::messages::list(client, &channel_id, 50).await {
                                        Ok(msgs) => {
                                            let items: Vec<MessageItem> = msgs.into_iter().map(|m| {
                                                let encrypted = !m.iv.is_empty();
                                                // For none-encryption channels, payload is plaintext
                                                let content = m.encrypted_payload.clone();
                                                MessageItem {
                                                    id: m.message_id.into(),
                                                    author: m.sender_username.clone().into(),
                                                    author_initial: initial(&m.sender_username).into(),
                                                    content: content.into(),
                                                    timestamp: format_timestamp(&m.timestamp).into(),
                                                    encrypted,
                                                }
                                            }).collect();
                                            let mh = main_h_bg.clone();
                                            slint::invoke_from_event_loop(move || {
                                                mh.unwrap().set_messages(ModelRc::new(VecModel::from(items)));
                                            }).ok();
                                        }
                                        Err(e) => tracing::warn!("Error loading messages: {e}"),
                                    }
                                }
                            }
                        }

                        Cmd::SendMessage { content } => {
                            if let Some(client) = &api {
                                if !session.active_channel_id.is_empty() {
                                    if let Err(e) = api::messages::send_plain(client, &session.active_channel_id, &content).await {
                                        tracing::warn!("Error sending message: {e}");
                                    }
                                }
                            }
                        }
                    }
                }

                Some(event) = event_rx.recv() => {
                    match event {
                        realtime::RealtimeEvent::Message {
                            channel_id, sender_username, encrypted_payload, iv,
                            message_id, timestamp, ..
                        } => {
                            if channel_id == session.active_channel_id {
                                let encrypted = !iv.is_empty();
                                let content = encrypted_payload; // plaintext per canals none
                                let item = MessageItem {
                                    id: message_id.into(),
                                    author: sender_username.clone().into(),
                                    author_initial: initial(&sender_username).into(),
                                    content: content.into(),
                                    timestamp: format_timestamp(&timestamp).into(),
                                    encrypted,
                                };
                                let mh = main_h_bg.clone();
                                slint::invoke_from_event_loop(move || {
                                    let win = mh.unwrap();
                                    let model = win.get_messages();
                                    if let Some(vm) = model.as_any().downcast_ref::<VecModel<MessageItem>>() {
                                        vm.push(item);
                                    }
                                }).ok();
                            }
                        }

                        realtime::RealtimeEvent::ChannelsUpdated { server_id } => {
                            if server_id == session.active_server_id {
                                let _ = cmd_tx.try_send(Cmd::SelectServer { server_id });
                            }
                        }

                        realtime::RealtimeEvent::Connected => {
                            let mh = main_h_bg.clone();
                            slint::invoke_from_event_loop(move || {
                                mh.unwrap().set_status_text("Connectat".into());
                            }).ok();
                        }

                        realtime::RealtimeEvent::Disconnected => {
                            let mh = main_h_bg.clone();
                            slint::invoke_from_event_loop(move || {
                                mh.unwrap().set_status_text("Desconnectat — reconnectant...".into());
                            }).ok();
                        }

                        _ => {}
                    }
                }
            }
        }
    });

    // show() no bloqueja — run_event_loop() manté el loop actiu fins que
    // totes les finestres es tanquen o es crida quit_event_loop()
    login_win.show().unwrap();
    slint::run_event_loop().unwrap();
}

// `on_saved`: callback opcional per notificar el caller quan es desa
fn open_settings(cfg: settings::Settings, on_saved: Option<Box<dyn Fn(settings::Settings) + 'static>>) {
    let win = SettingsWindow::new().unwrap();
    win.set_server_url(cfg.server.url.clone().into());
    win.set_vault_path(cfg.vault.path.to_string_lossy().to_string().into());
    win.set_notifications_enabled(cfg.notifications.enabled);
    win.set_notifications_sound(cfg.notifications.sound);

    let win_close = win.as_weak();
    win.on_close(move || {
        win_close.unwrap().hide().ok();
    });

    let win_save = win.as_weak();
    win.on_save(move || {
        let w = win_save.unwrap();
        let mut new_cfg = settings::Settings {
            server: settings::ServerSettings {
                url: w.get_server_url().to_string(),
            },
            vault: settings::VaultSettings {
                path: w.get_vault_path().to_string().into(),
            },
            notifications: settings::NotificationSettings {
                enabled: w.get_notifications_enabled(),
                sound: w.get_notifications_sound(),
                mention_only: false,
            },
        };
        if let Err(e) = settings::save(&new_cfg) {
            tracing::warn!("Error desant configuració: {e}");
        }
        if let Some(cb) = &on_saved {
            cb(new_cfg);
        }
        w.hide().ok();
    });

    win.show().ok();
}

fn format_timestamp(ts: &str) -> String {
    if let Some(t) = ts.split('T').nth(1) {
        t.chars().take(5).collect()
    } else {
        ts.chars().take(5).collect()
    }
}
