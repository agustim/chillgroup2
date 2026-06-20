mod api;
mod crypto;
mod realtime;
mod settings;
mod storage;

use base64::{engine::general_purpose::STANDARD, Engine};
use slint::{Model, ModelRc, VecModel};
use std::collections::HashMap;
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
    active_channel_key: Option<[u8; 32]>,
}

struct ChannelMeta {
    name: String,
    encryption_type: api::channels::EncryptionType,
    permission_level: i32,
}

#[derive(Debug)]
enum Cmd {
    OpenVault { passphrase: String, is_new: bool },
    Login { server_url: String, username: String, password: String },
    Logout,
    SelectServer { server_id: String },
    SelectChannel { channel_id: String, channel_type: String },
    SendMessage { content: String },
}

fn theme_to_index(theme: &str) -> i32 {
    match theme { "dark" => 1, "light" => 2, _ => 0 }
}

fn index_to_theme(idx: i32) -> &'static str {
    match idx { 1 => "dark", 2 => "light", _ => "system" }
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

    // Vault starts as None — only opened after passphrase entry
    let vault: Arc<Mutex<Option<storage::Vault>>> = Arc::new(Mutex::new(None));

    let (cmd_tx, mut cmd_rx) = mpsc::channel::<Cmd>(32);
    let (event_tx, mut event_rx) = mpsc::channel::<realtime::RealtimeEvent>(64);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let app = AppWindow::new().unwrap();
    app.set_server_url(cfg.server.url.clone().into());
    app.set_servers(ModelRc::new(VecModel::default()));
    app.set_channels(ModelRc::new(VecModel::default()));
    app.set_messages(ModelRc::new(VecModel::default()));
    app.set_app_theme_index(theme_to_index(&cfg.ui.theme));

    // Check vault state at startup
    let vault_path = cfg.vault.path.clone();
    let vault_exists = storage::Vault::exists(&vault_path);
    app.set_vault_exists(vault_exists);

    app.on_open_vault({
        let cmd_tx = cmd_tx.clone();
        let handle = app.as_weak();
        let vault_exists_ref = vault_exists;
        move || {
            let win = handle.unwrap();
            let passphrase = win.get_vault_passphrase().to_string();
            let is_new = !vault_exists_ref;

            if is_new {
                let confirm = win.get_vault_confirm().to_string();
                if passphrase.len() < 8 {
                    win.set_vault_error("La contrasenya ha de tenir com a mínim 8 caràcters".into());
                    return;
                }
                if passphrase != confirm {
                    win.set_vault_error("Les contrasenyes no coincideixen".into());
                    return;
                }
            }

            win.set_vault_error("".into());
            win.set_vault_loading(true);
            let _ = cmd_tx.try_send(Cmd::OpenVault { passphrase, is_new });
        }
    });

    app.on_login({
        let cmd_tx = cmd_tx.clone();
        let handle = app.as_weak();
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

    app.on_open_settings({
        let cfg_ref = cfg.clone();
        let app_weak = app.as_weak();
        move || open_settings(cfg_ref.clone(), app_weak.clone())
    });

    app.on_select_server({
        let cmd_tx = cmd_tx.clone();
        move |server_id| {
            let _ = cmd_tx.try_send(Cmd::SelectServer { server_id: server_id.to_string() });
        }
    });

    app.on_select_channel({
        let cmd_tx = cmd_tx.clone();
        move |channel_id, channel_type| {
            let _ = cmd_tx.try_send(Cmd::SelectChannel {
                channel_id: channel_id.to_string(),
                channel_type: channel_type.to_string(),
            });
        }
    });

    app.on_send_message({
        let cmd_tx = cmd_tx.clone();
        move |content| {
            let _ = cmd_tx.try_send(Cmd::SendMessage { content: content.to_string() });
        }
    });

    app.on_logout({
        let cmd_tx = cmd_tx.clone();
        move || { let _ = cmd_tx.try_send(Cmd::Logout); }
    });

    app.on_join_voice(|| { /* Phase 2 */ });
    app.on_leave_voice(|| { /* Phase 2 */ });
    app.on_toggle_mute(|| { /* Phase 2 */ });
    app.on_toggle_deafen(|| { /* Phase 2 */ });

    let app_bg = app.as_weak();
    let vault_bg = Arc::clone(&vault);
    let cfg_bg = cfg.clone();

    rt.spawn(async move {
        let mut api: Option<api::ApiClient> = None;
        let mut socket: Option<rust_socketio::asynchronous::Client> = None;
        let mut session = Session::default();
        let mut channel_meta: HashMap<String, ChannelMeta> = HashMap::new();

        loop {
            tokio::select! {
                Some(cmd) = cmd_rx.recv() => {
                    match cmd {
                        Cmd::OpenVault { passphrase, is_new } => {
                            let vault_path = cfg_bg.vault.path.clone();
                            let result = tokio::task::spawn_blocking(move || {
                                if is_new {
                                    storage::Vault::create(&vault_path, &passphrase)
                                } else {
                                    storage::Vault::open(&vault_path, &passphrase)
                                }
                            }).await;

                            match result {
                                Ok(Ok(v)) => {
                                    // Try auto-login from saved session
                                    let saved = v.load_session().unwrap_or(None);
                                    *vault_bg.lock().unwrap() = Some(v);

                                    let ah = app_bg.clone();
                                    if let Some((token, user_id, username, device_id)) = saved {
                                        // Have a saved token — try to use it
                                        let server_url = cfg_bg.server.url.clone();
                                        if !server_url.is_empty() && !token.is_empty() {
                                            let client = api::ApiClient::new(&server_url).with_token(&token);
                                            match api::servers::list(&client).await {
                                                Ok(servers) if !servers.is_empty() => {
                                                    // Token valid — auto-login
                                                    session.token = token.clone();
                                                    session.user_id = user_id;
                                                    session.username = username.clone();
                                                    session.device_id = device_id;

                                                    match realtime::connect(&server_url, &token, event_tx.clone()).await {
                                                        Ok(sock) => { socket = Some(sock); }
                                                        Err(e) => tracing::warn!("Socket.IO: {e}"),
                                                    }
                                                    api = Some(client);
                                                    ensure_kem_keypair(&vault_bg, api.as_ref().unwrap()).await;

                                                    let server_items: Vec<ServerItem> = servers.iter().map(|s| ServerItem {
                                                        id: s.server_id.clone().into(),
                                                        name: s.name.clone().into(),
                                                        initial: initial(&s.name).into(),
                                                    }).collect();
                                                    let first_id = servers.first().map(|s| s.server_id.clone());

                                                    slint::invoke_from_event_loop(move || {
                                                        let win = ah.unwrap();
                                                        win.set_current_username(username.clone().into());
                                                        win.set_current_username_initial(initial(&username).into());
                                                        win.set_servers(ModelRc::new(VecModel::from(server_items)));
                                                        win.set_status_text("Connectat".into());
                                                        win.set_vault_loading(false);
                                                        win.set_vault_open(true);
                                                        win.set_logged_in(true);
                                                    }).ok();

                                                    if let Some(sid) = first_id {
                                                        let _ = cmd_tx.try_send(Cmd::SelectServer { server_id: sid });
                                                    }
                                                }
                                                _ => {
                                                    // Token expired or server unreachable — show login
                                                    slint::invoke_from_event_loop(move || {
                                                        let win = ah.unwrap();
                                                        win.set_vault_loading(false);
                                                        win.set_vault_open(true); // shows login screen
                                                    }).ok();
                                                }
                                            }
                                        } else {
                                            slint::invoke_from_event_loop(move || {
                                                let win = ah.unwrap();
                                                win.set_vault_loading(false);
                                                win.set_vault_open(true);
                                            }).ok();
                                        }
                                    } else {
                                        // New vault or no session saved — show login
                                        slint::invoke_from_event_loop(move || {
                                            let win = ah.unwrap();
                                            win.set_vault_loading(false);
                                            win.set_vault_open(true);
                                        }).ok();
                                    }
                                }
                                Ok(Err(e)) => {
                                    let msg = e.to_string();
                                    let ah = app_bg.clone();
                                    slint::invoke_from_event_loop(move || {
                                        let win = ah.unwrap();
                                        win.set_vault_loading(false);
                                        win.set_vault_error(msg.into());
                                    }).ok();
                                }
                                Err(e) => {
                                    tracing::error!("spawn_blocking vault panic: {e}");
                                    let ah = app_bg.clone();
                                    slint::invoke_from_event_loop(move || {
                                        let win = ah.unwrap();
                                        win.set_vault_loading(false);
                                        win.set_vault_error("Error intern obrint el vault".into());
                                    }).ok();
                                }
                            }
                        }

                        Cmd::Login { server_url, username, password } => {
                            let client = api::ApiClient::new(&server_url);
                            match api::auth::login(&client, &username, &password).await {
                                Ok(data) => {
                                    let client = client.with_token(&data.token);
                                    session.token = data.token.clone();
                                    session.user_id = data.user_id.clone();
                                    session.username = data.username.clone();
                                    session.device_id = data.device_id.clone();

                                    if let Ok(mut v) = vault_bg.lock() {
                                        if let Some(vault) = v.as_ref() {
                                            let _ = vault.save_session(&data.token, &data.user_id, &data.username, &data.device_id);
                                        }
                                    }

                                    let mut new_cfg = cfg_bg.clone();
                                    new_cfg.server.url = server_url.clone();
                                    let _ = settings::save(&new_cfg);

                                    match realtime::connect(&server_url, &data.token, event_tx.clone()).await {
                                        Ok(sock) => { socket = Some(sock); }
                                        Err(e) => tracing::warn!("Socket.IO: {e}"),
                                    }

                                    api = Some(client);
                                    ensure_kem_keypair(&vault_bg, api.as_ref().unwrap()).await;

                                    match api::servers::list(api.as_ref().unwrap()).await {
                                        Ok(servers) => {
                                            let username_clone = data.username.clone();
                                            let server_items: Vec<ServerItem> = servers.iter().map(|s| ServerItem {
                                                id: s.server_id.clone().into(),
                                                name: s.name.clone().into(),
                                                initial: initial(&s.name).into(),
                                            }).collect();
                                            let first_id = servers.first().map(|s| s.server_id.clone());

                                            let ah = app_bg.clone();
                                            slint::invoke_from_event_loop(move || {
                                                let win = ah.unwrap();
                                                win.set_current_username(username_clone.clone().into());
                                                win.set_current_username_initial(initial(&username_clone).into());
                                                win.set_servers(ModelRc::new(VecModel::from(server_items)));
                                                win.set_status_text("Connectat".into());
                                                win.set_loading(false);
                                                win.set_logged_in(true);
                                            }).ok();

                                            if let Some(sid) = first_id {
                                                let _ = cmd_tx.try_send(Cmd::SelectServer { server_id: sid });
                                            }
                                        }
                                        Err(e) => {
                                            let msg = format!("Error carregant servidors: {e}");
                                            let ah = app_bg.clone();
                                            slint::invoke_from_event_loop(move || {
                                                let win = ah.unwrap();
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
                                    let ah = app_bg.clone();
                                    slint::invoke_from_event_loop(move || {
                                        let win = ah.unwrap();
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
                                        channel_meta.clear();
                                        let items: Vec<ChannelItem> = channels.iter().map(|c| {
                                            let enc_type_str = match c.encryption_type {
                                                api::channels::EncryptionType::Symmetric => "symmetric",
                                                api::channels::EncryptionType::Asymmetric => "asymmetric",
                                                api::channels::EncryptionType::None => "none",
                                            };
                                            channel_meta.insert(c.id.clone(), ChannelMeta {
                                                name: c.name.clone(),
                                                encryption_type: c.encryption_type.clone(),
                                                permission_level: c.permission_level.unwrap_or(2),
                                            });
                                            ChannelItem {
                                                id: c.id.clone().into(),
                                                name: c.name.clone().into(),
                                                channel_type: c.channel_type.as_str().into(),
                                                unread: c.unread_count.map(|n| n > 0).unwrap_or(false),
                                                encrypted: !matches!(c.encryption_type, api::channels::EncryptionType::None),
                                                encryption_type: enc_type_str.into(),
                                                permission_level: c.permission_level.unwrap_or(2),
                                            }
                                        }).collect();
                                        let ah = app_bg.clone();
                                        slint::invoke_from_event_loop(move || {
                                            let win = ah.unwrap();
                                            win.set_channels(ModelRc::new(VecModel::from(items)));
                                            win.set_active_server_id(server_id.into());
                                            win.set_active_channel_id("".into());
                                            win.set_active_channel_name("".into());
                                            win.set_active_channel_encrypted(false);
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
                                session.active_channel_key = None;

                                if let Some(sock) = &socket {
                                    let _ = realtime::join_channel(sock, &channel_id).await;
                                }

                                // Determine encryption type and fetch key if symmetric
                                let enc_type = channel_meta.get(&channel_id)
                                    .map(|m| m.encryption_type.clone())
                                    .unwrap_or_default();
                                let ch_name = channel_meta.get(&channel_id)
                                    .map(|m| m.name.clone())
                                    .unwrap_or_default();
                                let enc_type_str = match enc_type {
                                    api::channels::EncryptionType::Symmetric => "symmetric",
                                    api::channels::EncryptionType::Asymmetric => "asymmetric",
                                    api::channels::EncryptionType::None => "none",
                                }.to_string();

                                if matches!(enc_type, api::channels::EncryptionType::Symmetric) {
                                    // Check vault cache first
                                    let cached = vault_bg.lock().unwrap()
                                        .as_ref()
                                        .and_then(|v| v.load_channel_key(&channel_id).ok().flatten());

                                    let maybe_key = if let Some(bytes) = cached {
                                        if bytes.len() == 32 {
                                            let mut k = [0u8; 32];
                                            k.copy_from_slice(&bytes);
                                            Some(k)
                                        } else { None }
                                    } else {
                                        // Fetch from server
                                        match api::keys::get_channel_key(client, &channel_id).await {
                                            Ok(bundle) => {
                                                let dk = vault_bg.lock().unwrap()
                                                    .as_ref()
                                                    .and_then(|v| v.load_kem_keypair().ok().flatten())
                                                    .map(|(dk, _)| dk);
                                                if let Some(dk_bytes) = dk {
                                                    match crypto::unwrap_channel_key(&dk_bytes, &bundle.encrypted_key, &bundle.kem_ciphertext) {
                                                        Ok(key) => {
                                                            if let Some(vault) = vault_bg.lock().unwrap().as_ref() {
                                                                let _ = vault.save_channel_key(&channel_id, &key);
                                                            }
                                                            Some(key)
                                                        }
                                                        Err(e) => { tracing::warn!("KEM unwrap failed: {e}"); None }
                                                    }
                                                } else {
                                                    tracing::warn!("No KEM keypair in vault");
                                                    None
                                                }
                                            }
                                            Err(e) => { tracing::warn!("get_channel_key failed: {e}"); None }
                                        }
                                    };
                                    session.active_channel_key = maybe_key;
                                }

                                let is_blocked = match enc_type {
                                    api::channels::EncryptionType::None => false,
                                    api::channels::EncryptionType::Symmetric => session.active_channel_key.is_none(),
                                    api::channels::EncryptionType::Asymmetric => true,
                                };

                                let ch_id = channel_id.clone();
                                let ch_type = channel_type.clone();
                                let ah = app_bg.clone();
                                slint::invoke_from_event_loop(move || {
                                    let win = ah.unwrap();
                                    win.set_active_channel_id(ch_id.into());
                                    win.set_active_channel_type(ch_type.into());
                                    win.set_active_channel_name(ch_name.into());
                                    win.set_active_channel_encrypted(is_blocked);
                                    win.set_active_channel_encryption_type(enc_type_str.into());
                                    win.set_messages(ModelRc::new(VecModel::default()));
                                }).ok();

                                if channel_type == "text" && !is_blocked {
                                    match api::messages::list(client, &channel_id, 50).await {
                                        Ok(msgs) => {
                                            let channel_key = session.active_channel_key;
                                            let items: Vec<MessageItem> = msgs.into_iter().map(|m| {
                                                let content = if !m.iv.is_empty() {
                                                    if let Some(key) = &channel_key {
                                                        crypto::decrypt_message(key, &m.encrypted_payload, &m.iv)
                                                            .unwrap_or_else(|_| "[no s'ha pogut desxifrar]".into())
                                                    } else {
                                                        "[xifrat]".into()
                                                    }
                                                } else {
                                                    m.encrypted_payload.clone()
                                                };
                                                MessageItem {
                                                    id: m.message_id.into(),
                                                    author: m.sender_username.clone().into(),
                                                    author_initial: initial(&m.sender_username).into(),
                                                    content: content.into(),
                                                    timestamp: format_timestamp(&m.timestamp).into(),
                                                    encrypted: false,
                                                }
                                            }).collect();
                                            let ah = app_bg.clone();
                                            slint::invoke_from_event_loop(move || {
                                                ah.unwrap().set_messages(ModelRc::new(VecModel::from(items)));
                                            }).ok();
                                        }
                                        Err(e) => tracing::warn!("Error loading messages: {e}"),
                                    }
                                }
                            }
                        }

                        Cmd::Logout => {
                            // Clear session from vault, disconnect socket, reset UI to login
                            if let Ok(v) = vault_bg.lock() {
                                if let Some(vault) = v.as_ref() {
                                    let _ = vault.clear_session();
                                }
                            }
                            if let Some(sock) = socket.take() {
                                let _ = sock.disconnect().await;
                            }
                            api = None;
                            session = Session::default();
                            let ah = app_bg.clone();
                            slint::invoke_from_event_loop(move || {
                                let win = ah.unwrap();
                                win.set_logged_in(false);
                                win.set_username("".into());
                                win.set_password("".into());
                                win.set_error_message("".into());
                                win.set_servers(ModelRc::new(VecModel::default()));
                                win.set_channels(ModelRc::new(VecModel::default()));
                                win.set_messages(ModelRc::new(VecModel::default()));
                            }).ok();
                        }

                        Cmd::SendMessage { content } => {
                            if let Some(client) = &api {
                                if !session.active_channel_id.is_empty() {
                                    let (payload, iv) = if let Some(key) = &session.active_channel_key {
                                        crypto::encrypt_message(key, &content)
                                    } else {
                                        (content.clone(), String::new())
                                    };
                                    if let Err(e) = api::messages::send(client, &session.active_channel_id, &payload, &iv, None).await {
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
                            let is_active = channel_id == session.active_channel_id;

                            // Decrypt if we have the channel key
                            let content = if !iv.is_empty() {
                                if let Some(key) = &session.active_channel_key {
                                    crypto::decrypt_message(key, &encrypted_payload, &iv)
                                        .unwrap_or_else(|_| "[no s'ha pogut desxifrar]".into())
                                } else {
                                    "[xifrat]".into()
                                }
                            } else {
                                encrypted_payload.clone()
                            };

                            // Native notification (skip encrypted we can't decode)
                            if content != "[xifrat]" && sender_username != session.username {
                                let notif_body: String = content.chars().take(120).collect();
                                let notif_sender = sender_username.clone();
                                tokio::spawn(async move {
                                    notify_rust::Notification::new()
                                        .summary(&format!("ChillGroup — {notif_sender}"))
                                        .body(&notif_body)
                                        .timeout(notify_rust::Timeout::Milliseconds(4000))
                                        .show()
                                        .ok();
                                });
                            }

                            if is_active {
                                let item = MessageItem {
                                    id: message_id.into(),
                                    author: sender_username.clone().into(),
                                    author_initial: initial(&sender_username).into(),
                                    content: content.into(),
                                    timestamp: format_timestamp(&timestamp).into(),
                                    encrypted: false,
                                };
                                let ah = app_bg.clone();
                                slint::invoke_from_event_loop(move || {
                                    let win = ah.unwrap();
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
                            let ah = app_bg.clone();
                            slint::invoke_from_event_loop(move || {
                                ah.unwrap().set_status_text("Connectat".into());
                            }).ok();
                        }

                        realtime::RealtimeEvent::Disconnected => {
                            let ah = app_bg.clone();
                            slint::invoke_from_event_loop(move || {
                                ah.unwrap().set_status_text("Desconnectat — reconnectant...".into());
                            }).ok();
                        }
                    }
                }
            }
        }
    });

    app.run().unwrap();
}

fn open_settings(_cfg: settings::Settings, app_weak: slint::Weak<AppWindow>) {
    // Reload from disk so we always show current values (URL saved on login, etc.)
    let cfg = settings::load();
    let win = SettingsWindow::new().unwrap();
    win.set_server_url(cfg.server.url.clone().into());
    win.set_vault_path(cfg.vault.path.to_string_lossy().to_string().into());
    win.set_notifications_enabled(cfg.notifications.enabled);
    win.set_notifications_sound(cfg.notifications.sound);
    win.set_theme_index(theme_to_index(&cfg.ui.theme));

    // Preview immediat del tema sense necessitat de desar
    win.on_theme_preview({
        let app_weak = app_weak.clone();
        move |idx| {
            if let Some(app) = app_weak.upgrade() {
                app.set_app_theme_index(idx);
            }
        }
    });

    let win_close = win.as_weak();
    win.on_close(move || { win_close.unwrap().hide().ok(); });

    let win_save = win.as_weak();
    win.on_save(move || {
        let w = win_save.unwrap();
        let theme = index_to_theme(w.get_theme_index()).to_string();
        let new_cfg = settings::Settings {
            server: settings::ServerSettings { url: w.get_server_url().to_string() },
            vault: settings::VaultSettings { path: w.get_vault_path().to_string().into() },
            notifications: settings::NotificationSettings {
                enabled: w.get_notifications_enabled(),
                sound: w.get_notifications_sound(),
                mention_only: false,
            },
            ui: settings::UiSettings { theme: theme.clone() },
        };
        if let Err(e) = settings::save(&new_cfg) {
            tracing::warn!("Error desant configuració: {e}");
        }
        // Apply theme immediately
        if let Some(app) = app_weak.upgrade() {
            app.set_app_theme_index(theme_to_index(&theme));
        }
        w.hide().ok();
    });

    win.show().ok();
}

async fn ensure_kem_keypair(vault: &Arc<Mutex<Option<storage::Vault>>>, client: &api::ApiClient) {
    // Load or generate keypair
    let keypair = vault.lock().unwrap()
        .as_ref()
        .and_then(|v| v.load_kem_keypair().ok().flatten());

    let ek_bytes = match keypair {
        Some((_, ek)) => ek,
        None => {
            let (dk, ek) = crypto::generate_kem_keypair();
            if let Some(vault) = vault.lock().unwrap().as_ref() {
                if let Err(e) = vault.save_kem_keypair(&dk, &ek) {
                    tracing::warn!("Failed to save KEM keypair: {e}");
                    return;
                }
            }
            tracing::info!("Generated new ML-KEM-1024 keypair");
            ek
        }
    };

    let ek_b64 = STANDARD.encode(&ek_bytes);
    match api::keys::update_device_public_key(client, &ek_b64).await {
        Ok(()) => tracing::info!("KEM public key registered"),
        Err(e) => tracing::warn!("KEM registration failed: {e}"),
    }
}

fn format_timestamp(ts: &str) -> String {
    if let Some(t) = ts.split('T').nth(1) {
        t.chars().take(5).collect()
    } else {
        ts.chars().take(5).collect()
    }
}
