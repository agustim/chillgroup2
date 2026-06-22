mod api;
mod crypto;
mod realtime;
mod settings;
mod storage;
mod voice;

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
    active_channel_key_version: Option<i32>,
    active_channel_key_version_id: Option<String>,
    // Voice
    voice_cmd_tx: Option<mpsc::Sender<voice::VoiceCmd>>,
    voice_session_gen: u64,
    voice_muted: bool,
    voice_deafened: bool,
    voice_camera_on: bool,
    voice_screen_sharing: bool,
    voice_presence: HashMap<String, Vec<realtime::VoicePresenceUser>>,
}

struct ChannelMeta {
    name: String,
    encryption_type: api::channels::EncryptionType,
    permission_level: i32,
    message_ttl: Option<i32>,
    key_version: Option<i32>,
    key_version_id: Option<String>,
}

#[derive(Debug)]
enum Cmd {
    OpenVault { passphrase: String, is_new: bool },
    Login { server_url: String, username: String, password: String },
    Logout,
    SelectServer { server_id: String },
    SelectChannel { channel_id: String, channel_type: String },
    SendMessage { content: String },
    RepairChannel { channel_id: String },
    RotateChannelKey { channel_id: String },
    // Voice
    JoinVoice,
    LeaveVoice,
    ToggleMute,
    ToggleDeafen,
    ToggleCamera,
    ToggleScreenShare,
    StartScreenShare(u64, bool),
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
    let (voice_event_tx, mut voice_event_rx) = mpsc::channel::<voice::VoiceEvent>(32);

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

    app.on_join_voice({
        let cmd_tx = cmd_tx.clone();
        move || { let _ = cmd_tx.try_send(Cmd::JoinVoice); }
    });
    app.on_leave_voice({
        let cmd_tx = cmd_tx.clone();
        move || { let _ = cmd_tx.try_send(Cmd::LeaveVoice); }
    });
    app.on_toggle_mute({
        let cmd_tx = cmd_tx.clone();
        move || { let _ = cmd_tx.try_send(Cmd::ToggleMute); }
    });
    app.on_toggle_deafen({
        let cmd_tx = cmd_tx.clone();
        move || { let _ = cmd_tx.try_send(Cmd::ToggleDeafen); }
    });
    app.on_toggle_camera({
        let cmd_tx = cmd_tx.clone();
        move || { let _ = cmd_tx.try_send(Cmd::ToggleCamera); }
    });
    app.on_toggle_screen_share({
        let cmd_tx = cmd_tx.clone();
        move || { let _ = cmd_tx.try_send(Cmd::ToggleScreenShare); }
    });
    app.on_start_screen_share({
        let cmd_tx = cmd_tx.clone();
        move |id_str: slint::SharedString, is_window: bool| {
            if let Ok(source_id) = id_str.parse::<u64>() {
                let _ = cmd_tx.try_send(Cmd::StartScreenShare(source_id, is_window));
            }
        }
    });
    app.on_cancel_screen_share_picker({
        let aw = app.as_weak();
        move || { if let Some(h) = aw.upgrade() { h.set_screen_sources(Default::default()); } }
    });

    app.on_repair_channel({
        let cmd_tx = cmd_tx.clone();
        let app_weak = app.as_weak();
        move || {
            if let Some(win) = app_weak.upgrade() {
                let ch_id = win.get_active_channel_id().to_string();
                if !ch_id.is_empty() {
                    let _ = cmd_tx.try_send(Cmd::RepairChannel { channel_id: ch_id });
                }
            }
        }
    });

    app.on_rotate_channel_key({
        let cmd_tx = cmd_tx.clone();
        let app_weak = app.as_weak();
        move || {
            if let Some(win) = app_weak.upgrade() {
                let ch_id = win.get_active_channel_id().to_string();
                if !ch_id.is_empty() {
                    let _ = cmd_tx.try_send(Cmd::RotateChannelKey { channel_id: ch_id });
                }
            }
        }
    });

    let app_bg = app.as_weak();
    let vault_bg = Arc::clone(&vault);
    let cfg_bg = cfg.clone();
    let cmd_tx_close = cmd_tx.clone();

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
                                                    ensure_keypairs(&vault_bg, api.as_ref().unwrap()).await;

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
                                    ensure_keypairs(&vault_bg, api.as_ref().unwrap()).await;

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
                                                message_ttl: c.message_ttl,
                                                key_version: c.key_version,
                                                key_version_id: c.key_version_id.clone(),
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
                                session.active_channel_key_version = None;
                                session.active_channel_key_version_id = None;

                                if let Some(sock) = &socket {
                                    let _ = realtime::join_channel(sock, &channel_id).await;
                                }

                                let enc_type = channel_meta.get(&channel_id)
                                    .map(|m| m.encryption_type.clone())
                                    .unwrap_or_default();
                                let ch_name = channel_meta.get(&channel_id)
                                    .map(|m| m.name.clone())
                                    .unwrap_or_default();
                                let ch_ttl = channel_meta.get(&channel_id)
                                    .and_then(|m| m.message_ttl);
                                let enc_type_str = match enc_type {
                                    api::channels::EncryptionType::Symmetric => "symmetric",
                                    api::channels::EncryptionType::Asymmetric => "asymmetric",
                                    api::channels::EncryptionType::None => "none",
                                }.to_string();

                                let needs_key = matches!(enc_type,
                                    api::channels::EncryptionType::Symmetric |
                                    api::channels::EncryptionType::Asymmetric
                                );
                                if needs_key {
                                    let dk = vault_bg.lock().unwrap()
                                        .as_ref()
                                        .and_then(|v| v.load_kem_keypair().ok().flatten())
                                        .map(|(dk, _)| dk);

                                    if matches!(enc_type, api::channels::EncryptionType::Asymmetric) {
                                        // Sync ALL versions from /keys/all
                                        sync_all_key_bundles(client, &vault_bg, &channel_id, dk.as_deref()).await;
                                    }

                                    // Try to get latest key (from vault cache or server)
                                    let cached = vault_bg.lock().unwrap()
                                        .as_ref()
                                        .and_then(|v| v.load_channel_key(&channel_id).ok().flatten());

                                    let (maybe_key, key_version, key_version_id) = if let Some(bytes) = cached {
                                        if bytes.len() == 32 {
                                            let mut k = [0u8; 32];
                                            k.copy_from_slice(&bytes);
                                            // Restore version from vault (authoritative, updated on every key save/rotate)
                                            let (cur_ver, cur_ver_id) = vault_bg.lock().unwrap().as_ref()
                                                .and_then(|v| v.load_channel_key_current_version(&channel_id).ok().flatten())
                                                .map(|(v, vid)| (Some(v), Some(vid)))
                                                .unwrap_or_else(|| {
                                                    // Fallback: channel_meta from last SelectServer (may be stale after rotation)
                                                    let kv = channel_meta.get(&channel_id).and_then(|m| m.key_version);
                                                    let kvid = channel_meta.get(&channel_id).and_then(|m| m.key_version_id.clone());
                                                    (kv, kvid)
                                                });
                                            tracing::debug!("SelectChannel {}: flat cache hit, key_version={:?} key_version_id={:?}", channel_id, cur_ver, cur_ver_id);
                                            (Some(k), cur_ver, cur_ver_id)
                                        } else {
                                            tracing::warn!("SelectChannel {}: flat cache bad len={}", channel_id, bytes.len());
                                            (None, None, None)
                                        }
                                    } else {
                                        tracing::debug!("SelectChannel {}: flat cache miss, fetching from server", channel_id);
                                        // Fetch key bundle from server
                                        match api::keys::get_channel_key(client, &channel_id).await {
                                            Ok(bundle) => {
                                                let key_ver = bundle.key_version;
                                                let key_ver_id = bundle.key_version_id.clone();
                                                if let Some(dk_bytes) = &dk {
                                                    match crypto::unwrap_channel_key(dk_bytes, &bundle.encrypted_key, &bundle.kem_ciphertext) {
                                                        Ok(key) => {
                                                            if let Some(vault) = vault_bg.lock().unwrap().as_ref() {
                                                                let _ = vault.save_channel_key(&channel_id, &key);
                                                                if let Some(v) = key_ver {
                                                                    let _ = vault.save_channel_key_version(
                                                                        &channel_id, v, &key,
                                                                        key_ver_id.as_deref()
                                                                    );
                                                                    if let Some(ref vid) = key_ver_id {
                                                                        let _ = vault.save_channel_key_current_version(&channel_id, v, vid);
                                                                    }
                                                                }
                                                            }
                                                            (Some(key), key_ver, key_ver_id)
                                                        }
                                                        Err(e) => { tracing::warn!("KEM unwrap failed: {e}"); (None, None, None) }
                                                    }
                                                } else {
                                                    tracing::warn!("No KEM keypair in vault");
                                                    (None, None, None)
                                                }
                                            }
                                            Err(e) => {
                                                if matches!(enc_type, api::channels::EncryptionType::Asymmetric) {
                                                    tracing::info!("Asymmetric: no bundle yet ({e})");
                                                } else {
                                                    tracing::warn!("get_channel_key failed: {e}");
                                                }
                                                (None, None, None)
                                            }
                                        }
                                    };
                                    session.active_channel_key = maybe_key;
                                    session.active_channel_key_version = key_version;
                                    session.active_channel_key_version_id = key_version_id;

                                    // Asymmetric: if we have key + version_id, distribute to devices without bundles
                                    if matches!(enc_type, api::channels::EncryptionType::Asymmetric) {
                                        if let (Some(channel_key), Some(kvid)) = (session.active_channel_key, session.active_channel_key_version_id.clone()) {
                                            let client2 = client.clone();
                                            let vault2 = vault_bg.clone();
                                            let ch_id2 = channel_id.clone();
                                            let device_id2 = session.device_id.clone();
                                            let kv = session.active_channel_key_version;
                                            tokio::spawn(async move {
                                                distribute_channel_key(&client2, &vault2, &ch_id2, &device_id2, channel_key, kv, Some(kvid)).await;
                                            });
                                        }
                                    }
                                }

                                let is_blocked = match enc_type {
                                    api::channels::EncryptionType::None => false,
                                    api::channels::EncryptionType::Symmetric => session.active_channel_key.is_none(),
                                    api::channels::EncryptionType::Asymmetric => session.active_channel_key.is_none(),
                                };

                                let ch_id = channel_id.clone();
                                let ch_type = channel_type.clone();
                                let ch_ttl_i = ch_ttl.unwrap_or(0);
                                let ch_key_ver = session.active_channel_key_version.unwrap_or(0);
                                let ah = app_bg.clone();
                                slint::invoke_from_event_loop(move || {
                                    let win = ah.unwrap();
                                    win.set_active_channel_id(ch_id.into());
                                    win.set_active_channel_type(ch_type.into());
                                    win.set_active_channel_name(ch_name.into());
                                    win.set_active_channel_encrypted(is_blocked);
                                    win.set_active_channel_encryption_type(enc_type_str.into());
                                    win.set_active_channel_ttl(ch_ttl_i);
                                    win.set_active_channel_key_version(ch_key_ver);
                                    win.set_messages(ModelRc::new(VecModel::default()));
                                }).ok();

                                if channel_type == "text" && !is_blocked {
                                    match api::messages::list(client, &channel_id, 50).await {
                                        Ok(msgs) => {
                                            let vault_lock = vault_bg.lock().unwrap();
                                            let vault_ref = vault_lock.as_ref();
                                            let items: Vec<MessageItem> = msgs.into_iter().map(|m| {
                                                let content = if !m.iv.is_empty() {
                                                    let key_opt = if let Some(v) = m.key_version {
                                                        // Try version-specific key first
                                                        let from_vault = vault_ref.and_then(|v2| v2.load_channel_key_version(&channel_id, v).ok().flatten());
                                                        tracing::debug!("msg {} kv={} vault={} fallback={}", m.id, v, from_vault.is_some(), session.active_channel_key.is_some());
                                                        from_vault.or_else(|| session.active_channel_key.map(|k| k.to_vec()))
                                                    } else {
                                                        session.active_channel_key.map(|k| k.to_vec())
                                                    };
                                                    if let Some(key_bytes) = key_opt {
                                                        if key_bytes.len() == 32 {
                                                            let mut key = [0u8; 32];
                                                            key.copy_from_slice(&key_bytes);
                                                            crypto::decrypt_message(&key, &m.encrypted_payload, &m.iv)
                                                                .unwrap_or_else(|_| "[no s'ha pogut desxifrar]".into())
                                                        } else { "[clau invàlida]".into() }
                                                    } else {
                                                        "[xifrat — falta clau v{}]".replace("{}", &m.key_version.map(|v| v.to_string()).unwrap_or_default())
                                                    }
                                                } else {
                                                    m.encrypted_payload.clone()
                                                };
                                                let author = m.sender_username.clone().unwrap_or_default();
                                                let expires = m.expires_at.as_deref().map(format_expires).unwrap_or_default();
                                                MessageItem {
                                                    id: m.id.into(),
                                                    author: author.clone().into(),
                                                    author_initial: initial(&author).into(),
                                                    content: content.into(),
                                                    timestamp: format_timestamp(&m.timestamp).into(),
                                                    encrypted: false,
                                                    key_version: m.key_version.unwrap_or(0),
                                                    expires_at: expires.into(),
                                                }
                                            }).collect();
                                            drop(vault_lock);
                                            let ah = app_bg.clone();
                                            slint::invoke_from_event_loop(move || {
                                                let win = ah.unwrap();
                                                win.set_messages(ModelRc::new(VecModel::from(items)));
                                                win.set_scroll_to_bottom(true);
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
                                    // Optimistic: show immediately before API call
                                    let optimistic = MessageItem {
                                        id: "pending".into(),
                                        author: session.username.clone().into(),
                                        author_initial: initial(&session.username).into(),
                                        content: content.clone().into(),
                                        timestamp: "Ara".into(),
                                        encrypted: false,
                                        key_version: session.active_channel_key_version.unwrap_or(0),
                                        expires_at: "".into(),
                                    };
                                    let ah = app_bg.clone();
                                    slint::invoke_from_event_loop(move || {
                                        let win = ah.unwrap();
                                        let model = win.get_messages();
                                        if let Some(vm) = model.as_any().downcast_ref::<VecModel<MessageItem>>() {
                                            vm.push(optimistic);
                                        }
                                        win.set_scroll_to_bottom(true);
                                    }).ok();

                                    let (payload, iv) = if let Some(key) = &session.active_channel_key {
                                        crypto::encrypt_message(key, &content)
                                    } else {
                                        (content.clone(), String::new())
                                    };
                                    if let Err(e) = api::messages::send(client, &session.active_channel_id, &payload, &iv, session.active_channel_key_version).await {
                                        tracing::warn!("Error sending message: {e}");
                                    }
                                    // Real message arrives via Socket.IO; remove optimistic item
                                    let ah = app_bg.clone();
                                    slint::invoke_from_event_loop(move || {
                                        let win = ah.unwrap();
                                        let model = win.get_messages();
                                        if let Some(vm) = model.as_any().downcast_ref::<VecModel<MessageItem>>() {
                                            for i in (0..vm.row_count()).rev() {
                                                if let Some(item) = vm.row_data(i) {
                                                    if item.id.as_str() == "pending" {
                                                        vm.remove(i);
                                                        break;
                                                    }
                                                }
                                            }
                                        }
                                    }).ok();
                                }
                            }
                        }

                        Cmd::RepairChannel { channel_id } => {
                            if let Some(client) = &api {
                                let channel_key = if channel_id == session.active_channel_id {
                                    session.active_channel_key
                                } else { None };
                                if let Some(key) = channel_key {
                                    let client2 = client.clone();
                                    let vault2 = vault_bg.clone();
                                    let dev_id = session.device_id.clone();
                                    let kv = session.active_channel_key_version;
                                    let kvid = session.active_channel_key_version_id.clone();
                                    tokio::spawn(async move {
                                        distribute_channel_key(&client2, &vault2, &channel_id, &dev_id, key, kv, kvid).await;
                                    });
                                } else {
                                    tracing::warn!("RepairChannel: no key available for {channel_id}");
                                }
                            }
                        }

                        Cmd::RotateChannelKey { channel_id } => {
                            if let Some(client) = &api {
                                let enc_type = channel_meta.get(&channel_id)
                                    .map(|m| m.encryption_type.clone())
                                    .unwrap_or_default();
                                match api::keys::rotate_channel_key(client, &channel_id).await {
                                    Ok(result) => {
                                        tracing::info!("Rotated key to v{} for {channel_id}", result.key_version);
                                        if matches!(enc_type, api::channels::EncryptionType::Asymmetric) {
                                            // Generate new channel key and distribute
                                            let new_key: [u8; 32] = rand::random();
                                            if let Some(vault) = vault_bg.lock().unwrap().as_ref() {
                                                let _ = vault.save_channel_key(&channel_id, &new_key);
                                                let _ = vault.save_channel_key_version(
                                                    &channel_id, result.key_version, &new_key,
                                                    Some(&result.key_version_id)
                                                );
                                                let _ = vault.save_channel_key_current_version(
                                                    &channel_id, result.key_version, &result.key_version_id
                                                );
                                            }
                                            if channel_id == session.active_channel_id {
                                                session.active_channel_key = Some(new_key);
                                                session.active_channel_key_version = Some(result.key_version);
                                                session.active_channel_key_version_id = Some(result.key_version_id.clone());
                                                let kv = result.key_version;
                                                let ah = app_bg.clone();
                                                slint::invoke_from_event_loop(move || {
                                                    ah.unwrap().set_active_channel_key_version(kv);
                                                }).ok();
                                            }
                                            let client2 = client.clone();
                                            let vault2 = vault_bg.clone();
                                            let dev_id = session.device_id.clone();
                                            let kv = Some(result.key_version);
                                            let kvid = Some(result.key_version_id);
                                            tokio::spawn(async move {
                                                distribute_channel_key(&client2, &vault2, &channel_id, &dev_id, new_key, kv, kvid).await;
                                            });
                                        } else {
                                            // Symmetric: server generated new key; refresh
                                            let dk = vault_bg.lock().unwrap()
                                                .as_ref()
                                                .and_then(|v| v.load_kem_keypair().ok().flatten())
                                                .map(|(dk, _)| dk);
                                            if let Some(dk_bytes) = dk {
                                                if let Ok(bundle) = api::keys::get_channel_key(client, &channel_id).await {
                                                    if let Ok(key) = crypto::unwrap_channel_key(&dk_bytes, &bundle.encrypted_key, &bundle.kem_ciphertext) {
                                                        if let Some(vault) = vault_bg.lock().unwrap().as_ref() {
                                                            let _ = vault.save_channel_key(&channel_id, &key);
                                                            if let Some(v) = bundle.key_version {
                                                                let _ = vault.save_channel_key_version(&channel_id, v, &key, bundle.key_version_id.as_deref());
                                                                if let Some(ref vid) = bundle.key_version_id {
                                                                    let _ = vault.save_channel_key_current_version(&channel_id, v, vid);
                                                                }
                                                            }
                                                        }
                                                        if channel_id == session.active_channel_id {
                                                            session.active_channel_key = Some(key);
                                                            session.active_channel_key_version = bundle.key_version;
                                                            session.active_channel_key_version_id = bundle.key_version_id;
                                                            let kv = session.active_channel_key_version.unwrap_or(0);
                                                            let ah = app_bg.clone();
                                                            slint::invoke_from_event_loop(move || {
                                                                ah.unwrap().set_active_channel_key_version(kv);
                                                            }).ok();
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => tracing::warn!("Rotate key failed: {e}"),
                                }
                            }
                        }

                        Cmd::JoinVoice => {
                            // Auto-disconnect from any existing voice session first
                            if let Some(tx) = session.voice_cmd_tx.take() {
                                let _ = tx.try_send(voice::VoiceCmd::Disconnect);
                            }
                            if let Some(client) = &api {
                                let channel_id = session.active_channel_id.clone();
                                let client = client.clone();
                                let e2ee_key = session.active_channel_key;
                                let vtx = voice_event_tx.clone();
                                let (vcmd_tx, vcmd_rx) = mpsc::channel::<voice::VoiceCmd>(8);
                                session.voice_session_gen += 1;
                                let gen = session.voice_session_gen;
                                session.voice_cmd_tx = Some(vcmd_tx);
                                let weak_mtx = Arc::new(Mutex::new(app_bg.clone()));
                        let frame_cb: Arc<dyn Fn(Vec<u8>, u32, u32) + Send + Sync> = {
                            let wm = Arc::clone(&weak_mtx);
                            Arc::new(move |bytes: Vec<u8>, w: u32, h: u32| {
                                let weak = wm.lock().unwrap().clone();
                                slint::invoke_from_event_loop(move || {
                                    if let Some(a) = weak.upgrade() {
                                        let buf = slint::SharedPixelBuffer::<slint::Rgb8Pixel>::clone_from_slice(&bytes, w, h);
                                        a.set_camera_preview(slint::Image::from_rgb8(buf));
                                    }
                                }).ok();
                            })
                        };
                        tokio::spawn(async move {
                                    match api::livekit::get_token(&client, &channel_id).await {
                                        Ok(resp) => {
                                            voice::run(resp.url, resp.token, e2ee_key, gen, vcmd_rx, vtx, Some(frame_cb)).await;
                                        }
                                        Err(e) => {
                                            let _ = vtx.send(voice::VoiceEvent::Error { session_gen: gen, msg: format!("Token LiveKit: {e}") }).await;
                                        }
                                    }
                                });
                            } else {
                                tracing::warn!("JoinVoice: no API client (not logged in)");
                            }
                        }

                        Cmd::LeaveVoice => {
                            if let Some(tx) = session.voice_cmd_tx.take() {
                                let _ = tx.try_send(voice::VoiceCmd::Disconnect);
                            }
                        }

                        Cmd::ToggleMute => {
                            if let Some(tx) = &session.voice_cmd_tx {
                                let _ = tx.try_send(voice::VoiceCmd::ToggleMute);
                            }
                        }

                        Cmd::ToggleDeafen => {
                            if let Some(tx) = &session.voice_cmd_tx {
                                let _ = tx.try_send(voice::VoiceCmd::ToggleDeafen);
                            }
                        }

                        Cmd::ToggleCamera => {
                            if let Some(tx) = &session.voice_cmd_tx {
                                let _ = tx.try_send(voice::VoiceCmd::ToggleCamera);
                            }
                        }

                        Cmd::ToggleScreenShare => {
                            if let Some(tx) = &session.voice_cmd_tx {
                                let _ = tx.try_send(voice::VoiceCmd::ToggleScreenShare);
                            }
                        }

                        Cmd::StartScreenShare(source_id, is_window) => {
                            if let Some(tx) = &session.voice_cmd_tx {
                                let _ = tx.try_send(voice::VoiceCmd::StartScreenShare { source_id, is_window });
                            }
                            let ah = app_bg.clone();
                            slint::invoke_from_event_loop(move || {
                                ah.unwrap().set_screen_sources(Default::default());
                            }).ok();
                        }
                    }
                }

                Some(ve) = voice_event_rx.recv() => {
                    match ve {
                        voice::VoiceEvent::Connected { session_gen } => {
                            if session_gen != session.voice_session_gen { continue; }
                            let channel_id = session.active_channel_id.clone();
                            // Notify server we're in this voice channel
                            if let Some(sock) = &socket {
                                let _ = realtime::join_voice_channel(sock, &channel_id).await;
                            }
                            // Add ourselves to sidebar presence locally
                            let self_user = realtime::VoicePresenceUser {
                                user_id: session.user_id.clone(),
                                username: session.username.clone(),
                            };
                            let entry = session.voice_presence.entry(channel_id).or_default();
                            entry.retain(|u| u.user_id != session.user_id);
                            entry.push(self_user);
                            push_voice_sidebar(&session.voice_presence, &app_bg);

                            let ah = app_bg.clone();
                            slint::invoke_from_event_loop(move || {
                                ah.unwrap().set_in_voice_channel(true);
                                ah.unwrap().set_voice_participants(Default::default());
                            }).ok();
                        }
                        voice::VoiceEvent::Disconnected { session_gen } => {
                            if session_gen != session.voice_session_gen { continue; }
                            let channel_id = session.active_channel_id.clone();
                            // Notify server we've left this voice channel
                            if let Some(sock) = &socket {
                                let _ = realtime::leave_voice_channel(sock, &channel_id).await;
                            }
                            // Remove ourselves from sidebar presence locally
                            if let Some(users) = session.voice_presence.get_mut(&channel_id) {
                                users.retain(|u| u.user_id != session.user_id);
                                if users.is_empty() { session.voice_presence.remove(&channel_id); }
                            }
                            push_voice_sidebar(&session.voice_presence, &app_bg);

                            session.voice_cmd_tx = None;
                            session.voice_muted = false;
                            session.voice_deafened = false;
                            session.voice_camera_on = false;
                            session.voice_screen_sharing = false;
                            let ah = app_bg.clone();
                            slint::invoke_from_event_loop(move || {
                                let h = ah.unwrap();
                                h.set_in_voice_channel(false);
                                h.set_mic_muted(false);
                                h.set_deafened(false);
                                h.set_camera_on(false);
                                h.set_camera_preview(Default::default());
                                h.set_screen_sharing(false);
                                h.set_screen_sources(Default::default());
                                h.set_voice_participants(Default::default());
                            }).ok();
                        }
                        voice::VoiceEvent::ParticipantsUpdated(parts) => {
                            let ah = app_bg.clone();
                            slint::invoke_from_event_loop(move || {
                                let h = ah.unwrap();
                                let existing = h.get_voice_participants();
                                let new_parts: Vec<VoiceParticipant> = parts.into_iter().map(|p| {
                                    let (video_preview, has_video) = if p.has_video {
                                        let img = (0..existing.row_count())
                                            .find_map(|i| existing.row_data(i).filter(|ep| ep.user_id == p.user_id.as_str()))
                                            .map(|ep| ep.video_preview)
                                            .unwrap_or_default();
                                        (img, true)
                                    } else {
                                        (Default::default(), false)
                                    };
                                    VoiceParticipant {
                                        user_id: p.user_id.into(),
                                        username: p.username.into(),
                                        initial: p.initial.into(),
                                        is_speaking: p.is_speaking,
                                        is_suppressed: p.is_suppressed,
                                        has_video,
                                        video_preview,
                                        is_screen: p.is_screen,
                                    }
                                }).collect();
                                let model = std::rc::Rc::new(slint::VecModel::from(new_parts));
                                h.set_voice_participants(slint::ModelRc::from(model));
                            }).ok();
                        }
                        voice::VoiceEvent::RemoteVideoFrame { participant_id, bytes, w, h: fh } => {
                            let ah = app_bg.clone();
                            slint::invoke_from_event_loop(move || {
                                let h = ah.unwrap();
                                let model = h.get_voice_participants();
                                for i in 0..model.row_count() {
                                    if let Some(mut p) = model.row_data(i) {
                                        if p.user_id.as_str() == participant_id {
                                            let buf = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(&bytes, w, fh);
                                            p.video_preview = slint::Image::from_rgba8(buf);
                                            p.has_video = true;
                                            model.set_row_data(i, p);
                                            break;
                                        }
                                    }
                                }
                            }).ok();
                        }
                        voice::VoiceEvent::MuteChanged(muted) => {
                            session.voice_muted = muted;
                            let ah = app_bg.clone();
                            slint::invoke_from_event_loop(move || {
                                ah.unwrap().set_mic_muted(muted);
                            }).ok();
                        }
                        voice::VoiceEvent::DeafenChanged(deafened) => {
                            session.voice_deafened = deafened;
                            let ah = app_bg.clone();
                            slint::invoke_from_event_loop(move || {
                                ah.unwrap().set_deafened(deafened);
                            }).ok();
                        }
                        voice::VoiceEvent::CameraChanged(on) => {
                            session.voice_camera_on = on;
                            let ah = app_bg.clone();
                            slint::invoke_from_event_loop(move || {
                                let h = ah.unwrap();
                                h.set_camera_on(on);
                                if !on { h.set_camera_preview(Default::default()); }
                            }).ok();
                        }
                        voice::VoiceEvent::ScreenSources(sources) => {
                            let ah = app_bg.clone();
                            slint::invoke_from_event_loop(move || {
                                let h = ah.unwrap();
                                let items: Vec<ScreenSource> = sources.iter().map(|(id, title, is_window)| {
                                    ScreenSource {
                                        id: id.to_string().into(),
                                        title: title.as_str().into(),
                                        is_window: *is_window,
                                    }
                                }).collect();
                                let model = std::rc::Rc::new(slint::VecModel::from(items));
                                h.set_screen_sources(slint::ModelRc::from(model));
                            }).ok();
                        }
                        voice::VoiceEvent::ScreenShareChanged(on) => {
                            session.voice_screen_sharing = on;
                            let ah = app_bg.clone();
                            slint::invoke_from_event_loop(move || {
                                let h = ah.unwrap();
                                h.set_screen_sharing(on);
                                if !on { h.set_screen_sources(Default::default()); }
                            }).ok();
                        }
                        voice::VoiceEvent::Error { session_gen, msg } => {
                            tracing::error!("Voice error (gen={session_gen}): {msg}");
                            if session_gen == session.voice_session_gen {
                                session.voice_cmd_tx = None;
                                let ah = app_bg.clone();
                                let smsg: slint::SharedString = format!("Error de veu: {msg}").into();
                                slint::invoke_from_event_loop(move || {
                                    let h = ah.unwrap();
                                    h.set_in_voice_channel(false);
                                    h.set_status_text(smsg);
                                }).ok();
                            }
                        }
                    }
                }

                Some(event) = event_rx.recv() => {
                    match event {
                        realtime::RealtimeEvent::Message {
                            channel_id, sender_username, encrypted_payload, iv,
                            message_id, timestamp, key_version: msg_key_version, expires_at, ..
                        } => {
                            let is_active = channel_id == session.active_channel_id;

                            // Decrypt using version-specific key when available
                            let content = if !iv.is_empty() {
                                let key_opt: Option<[u8; 32]> = if let Some(v) = msg_key_version {
                                    let from_vault = vault_bg.lock().unwrap()
                                        .as_ref()
                                        .and_then(|vlt| vlt.load_channel_key_version(&channel_id, v).ok().flatten());
                                    tracing::debug!("realtime msg {} ch={} kv={} vault={} active_key={}", message_id, channel_id, v, from_vault.is_some(), session.active_channel_key.is_some());
                                    if let Some(bytes) = from_vault {
                                        if bytes.len() == 32 {
                                            let mut k = [0u8; 32];
                                            k.copy_from_slice(&bytes);
                                            Some(k)
                                        } else { session.active_channel_key }
                                    } else { session.active_channel_key }
                                } else {
                                    tracing::debug!("realtime msg {} ch={} kv=None active_key={}", message_id, channel_id, session.active_channel_key.is_some());
                                    session.active_channel_key
                                };

                                if let Some(key) = key_opt {
                                    crypto::decrypt_message(&key, &encrypted_payload, &iv)
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
                                let expires_str = expires_at.as_deref().map(format_expires).unwrap_or_default();
                                let item = MessageItem {
                                    id: message_id.into(),
                                    author: sender_username.clone().into(),
                                    author_initial: initial(&sender_username).into(),
                                    content: content.into(),
                                    timestamp: format_timestamp(&timestamp).into(),
                                    encrypted: false,
                                    key_version: msg_key_version.unwrap_or(0),
                                    expires_at: expires_str.into(),
                                };
                                let ah = app_bg.clone();
                                slint::invoke_from_event_loop(move || {
                                    let win = ah.unwrap();
                                    let model = win.get_messages();
                                    if let Some(vm) = model.as_any().downcast_ref::<VecModel<MessageItem>>() {
                                        vm.push(item);
                                    }
                                    win.set_scroll_to_bottom(true);
                                }).ok();
                            }
                        }

                        realtime::RealtimeEvent::ChannelsUpdated { server_id } => {
                            tracing::debug!("ChannelsUpdated server={} active={}", server_id, session.active_server_id);
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

                        realtime::RealtimeEvent::VoicePresenceSnapshot { server_id, channels } => {
                            if server_id == session.active_server_id {
                                session.voice_presence.clear();
                                for (channel_id, users) in channels {
                                    session.voice_presence.insert(channel_id, users);
                                }
                                // Re-add self if in voice (server snapshot never includes local user)
                                ensure_self_in_presence(&mut session);
                                push_voice_sidebar(&session.voice_presence, &app_bg);
                            }
                        }

                        realtime::RealtimeEvent::VoicePresenceUpdated { channel_id, users } => {
                            if users.is_empty() {
                                session.voice_presence.remove(&channel_id);
                            } else {
                                session.voice_presence.insert(channel_id, users);
                            }
                            // Re-add self to updated channel if in voice
                            ensure_self_in_presence(&mut session);
                            push_voice_sidebar(&session.voice_presence, &app_bg);
                        }
                    }
                }
            }
        }
    });

    // Leave voice channel cleanly on window close
    app.window().on_close_requested(move || {
        let _ = cmd_tx_close.try_send(Cmd::LeaveVoice);
        slint::CloseRequestResponse::HideWindow
    });

    app.run().unwrap();
}

fn ensure_self_in_presence(session: &mut Session) {
    if session.voice_cmd_tx.is_none() { return; }
    let channel_id = session.active_channel_id.clone();
    if channel_id.is_empty() { return; }
    let self_user = realtime::VoicePresenceUser {
        user_id: session.user_id.clone(),
        username: session.username.clone(),
    };
    let entry = session.voice_presence.entry(channel_id).or_default();
    if !entry.iter().any(|u| u.user_id == self_user.user_id) {
        entry.push(self_user);
    }
}

fn push_voice_sidebar(
    presence: &HashMap<String, Vec<realtime::VoicePresenceUser>>,
    app_bg: &slint::Weak<AppWindow>,
) {
    let entries: Vec<VoiceSidebarEntry> = presence
        .iter()
        .flat_map(|(channel_id, users)| {
            users.iter().map(move |u| {
                let initial = u.username.chars().next()
                    .map(|c| c.to_uppercase().to_string())
                    .unwrap_or_else(|| "?".into());
                VoiceSidebarEntry {
                    channel_id: channel_id.clone().into(),
                    username: u.username.clone().into(),
                    initial: initial.into(),
                    is_speaking: false,
                    is_suppressed: false,
                }
            })
        })
        .collect();
    let ah = app_bg.clone();
    slint::invoke_from_event_loop(move || {
        let model = std::rc::Rc::new(slint::VecModel::from(entries));
        ah.unwrap().set_voice_sidebar_presences(slint::ModelRc::from(model));
    }).ok();
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

/// Sync all key bundle versions for an asymmetric channel.
async fn sync_all_key_bundles(
    client: &api::ApiClient,
    vault: &Arc<Mutex<Option<storage::Vault>>>,
    channel_id: &str,
    dk_bytes: Option<&[u8]>,
) {
    let Some(dk) = dk_bytes else { return; };
    let bundles = match api::keys::get_all_key_bundles(client, channel_id).await {
        Ok(b) => b,
        Err(e) => { tracing::warn!("get_all_key_bundles failed: {e}"); return; }
    };
    for bundle in bundles {
        let Some(version) = bundle.key_version else { continue; };
        // Skip if already cached
        let already_cached = vault.lock().unwrap()
            .as_ref()
            .and_then(|v| v.load_channel_key_version(channel_id, version).ok().flatten())
            .is_some();
        if already_cached { continue; }

        match crypto::unwrap_channel_key(dk, &bundle.encrypted_key, &bundle.kem_ciphertext) {
            Ok(key) => {
                if let Some(v) = vault.lock().unwrap().as_ref() {
                    let _ = v.save_channel_key_version(
                        channel_id, version, &key,
                        bundle.key_version_id.as_deref()
                    );
                }
                tracing::debug!("Synced key v{version} for channel {channel_id}");
            }
            Err(e) => {
                tracing::debug!("Bundle v{version} not for us or invalid: {e}");
            }
        }
    }
}

/// Distribute an asymmetric channel key to all member devices that don't have a bundle yet.
async fn distribute_channel_key(
    client: &api::ApiClient,
    vault: &Arc<Mutex<Option<storage::Vault>>>,
    channel_id: &str,
    our_device_id: &str,
    channel_key: [u8; 32],
    key_version: Option<i32>,
    key_version_id: Option<String>,
) {
    let devices = match api::keys::get_member_devices(client, channel_id).await {
        Ok(d) => d,
        Err(e) => { tracing::warn!("get_member_devices failed: {e}"); return; }
    };

    let dsa_sk = vault.lock().unwrap()
        .as_ref()
        .and_then(|v| v.load_dsa_keypair().ok().flatten())
        .map(|(sk, _)| sk);
    let Some(dsa_sk_bytes) = dsa_sk else {
        tracing::warn!("No DSA keypair in vault, cannot distribute");
        return;
    };

    // Find which devices already have a bundle for this key_version — skip them
    let devices_with_bundle: std::collections::HashSet<String> = if key_version_id.is_some() {
        match api::keys::get_all_key_bundles(client, channel_id).await {
            Ok(all) => all.into_iter()
                .filter(|b| b.key_version_id == key_version_id)
                .map(|b| b.device_id)
                .collect(),
            Err(_) => std::collections::HashSet::new(),
        }
    } else {
        std::collections::HashSet::new()
    };

    let mut bundles = Vec::new();
    for device in &devices {
        if device.device_id == our_device_id { continue; }
        if device.kem_public_key.is_empty() { continue; }
        if devices_with_bundle.contains(&device.device_id) {
            tracing::debug!("Device {} already has bundle for v{:?}, skipping", device.device_id, key_version);
            continue;
        }

        let ek_bytes = match STANDARD.decode(&device.kem_public_key) {
            Ok(b) => b,
            Err(e) => { tracing::warn!("Bad KEM key for device {}: {e}", device.device_id); continue; }
        };

        match crypto::wrap_channel_key_for_device(&ek_bytes, &channel_key) {
            Ok((enc_key, kem_ct)) => {
                // Signature payload: "${keyVersionId}:${deviceId}:${kemCiphertext}:${encryptedKey}"
                // Matches frontend format in channel-crypto.ts buildSignaturePayload()
                let signature = if let Some(ref vid) = key_version_id {
                    let msg = format!("{}:{}:{}:{}", vid, device.device_id, kem_ct, enc_key);
                    crypto::dsa_sign(&dsa_sk_bytes, msg.as_bytes()).ok()
                } else { None };
                bundles.push(api::keys::KeyBundle {
                    device_id: device.device_id.clone(),
                    encrypted_key: enc_key,
                    kem_ciphertext: kem_ct,
                    key_version,
                    signature,
                    signed_by_device_id: Some(our_device_id.to_string()),
                });
            }
            Err(e) => { tracing::warn!("wrap_channel_key_for_device failed for {}: {e}", device.device_id); }
        }
    }

    let mut distributed = 0usize;
    for bundle in bundles {
        let device_id = bundle.device_id.clone();
        match api::keys::upload_key_bundles(client, channel_id, &[bundle]).await {
            Ok(()) => { distributed += 1; }
            Err(e) if e.to_string().contains("409") || e.to_string().contains("Conflict") => {
                tracing::debug!("Device {device_id} already has key bundle, skipping");
            }
            Err(e) => { tracing::warn!("upload_key_bundles for {device_id} failed: {e}"); }
        }
    }
    if distributed > 0 {
        tracing::info!("Distributed key to {distributed} devices for channel {channel_id}");
    }
}

async fn ensure_keypairs(vault: &Arc<Mutex<Option<storage::Vault>>>, client: &api::ApiClient) {
    // KEM keypair
    let kem_pair = vault.lock().unwrap()
        .as_ref()
        .and_then(|v| v.load_kem_keypair().ok().flatten());
    let ek_bytes = match kem_pair {
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

    // DSA keypair
    let dsa_pair = vault.lock().unwrap()
        .as_ref()
        .and_then(|v| v.load_dsa_keypair().ok().flatten());
    let dsa_vk_bytes = match dsa_pair {
        Some((_, vk)) => vk,
        None => {
            let (sk, vk) = crypto::generate_dsa_keypair();
            if let Some(vault) = vault.lock().unwrap().as_ref() {
                if let Err(e) = vault.save_dsa_keypair(&sk, &vk) {
                    tracing::warn!("Failed to save DSA keypair: {e}");
                    return;
                }
            }
            tracing::info!("Generated new ML-DSA-87 keypair");
            vk
        }
    };

    let ek_b64 = STANDARD.encode(&ek_bytes);
    let dsa_vk_b64 = STANDARD.encode(&dsa_vk_bytes);
    match api::keys::update_device_public_key(client, &ek_b64, &dsa_vk_b64).await {
        Ok(()) => tracing::info!("KEM + DSA public keys registered"),
        Err(e) => tracing::warn!("Key registration failed: {e}"),
    }
}

fn format_timestamp(ts: &str) -> String {
    if let Some(t) = ts.split('T').nth(1) {
        t.chars().take(5).collect()
    } else {
        ts.chars().take(5).collect()
    }
}

fn format_expires(expires_at: &str) -> String {
    // Returns "⏱ HH:MM" or "⏱ Dd" depending on remaining time
    use chrono::{DateTime, Utc};
    if let Ok(exp) = expires_at.parse::<DateTime<Utc>>() {
        let now = Utc::now();
        let diff = exp.signed_duration_since(now);
        if diff.num_seconds() <= 0 {
            return "⏱ Caducat".into();
        }
        let secs = diff.num_seconds();
        if secs < 3600 {
            format!("⏱ {}m", secs / 60)
        } else if secs < 86400 {
            format!("⏱ {}h", secs / 3600)
        } else {
            format!("⏱ {}d", secs / 86400)
        }
    } else {
        String::new()
    }
}
