//! Endpoints d'autenticació: register, login.

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
    routing,
    Router,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use crate::middleware::AuthClaims;
use rand::{distributions::Alphanumeric, Rng};
use shared::types::{AuthResponse, RefreshResponse};
use serde::Deserialize;
use uuid::Uuid;
use tracing::{debug, info, error, warn};
use serde::Serialize;

use crate::{
    crypto::hash,
    error::AppError,
    middleware::{AppState, generate_token, generate_claims},
};
use shared::constants::{MIN_USERNAME_LENGTH, MAX_USERNAME_LENGTH, MIN_PASSWORD_LENGTH};

// ── Requests ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub device_id: Option<Uuid>,
    pub admin_invitation_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub device_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterWithInvitationRequest {
    pub code: String,
    pub username: String,
    pub password: String,
    pub device_id: Option<Uuid>,
    pub admin_invitation_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateInvitationRequest {
    pub max_uses: Option<i32>,
    pub server_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvitationListItem {
    pub invitation_id: Uuid,
    pub code: String,
    pub server_id: Option<Uuid>,
    pub max_uses: i32,
    pub uses_count: i32,
    pub remaining_uses: Option<i32>,
    pub is_active: bool,
    pub created_by: String,
}

fn generate_invitation_code() -> String {
    let raw: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(24)
        .map(char::from)
        .collect();

    format!(
        "{}-{}-{}-{}",
        &raw[0..6],
        &raw[6..12],
        &raw[12..18],
        &raw[18..24],
    )
    .to_uppercase()
}

fn parse_admin_invitation_hash(code: Option<&str>) -> Option<String> {
    let trimmed = code?.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(hash::hash_admin_invitation_code(trimmed))
}

// ── Register ─────────────────────────────────────────────────────

#[axum::debug_handler]
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    debug!("📝 Endpoint de register cridat");

    let admin_invitation_hash = parse_admin_invitation_hash(req.admin_invitation_code.as_deref());

    if !state.config.open_register && admin_invitation_hash.is_none() {
        warn!("❌ Register rebutjat: OPEN_REGISTER=false");
        return Err(AppError::RegistrationClosed);
    }

    // Validar username
    if req.username.len() < MIN_USERNAME_LENGTH || req.username.len() > MAX_USERNAME_LENGTH {
        error!("❌ Register fallat: username amb longitud invàlida");
        return Err(AppError::InvalidUsername);
    }
    if !req.username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        error!("❌ Register fallat: username amb caràcters invàlids");
        return Err(AppError::InvalidUsername);
    }

    // Validar password
    if req.password.len() < MIN_PASSWORD_LENGTH {
        error!("❌ Register fallat: password massa curt ({} < {})", req.password.len(), MIN_PASSWORD_LENGTH);
        return Err(AppError::WeakPassword { min: MIN_PASSWORD_LENGTH });
    }

    // Comprovar si l'usuari ja existeix
    let exists = state.db.user_exists(&req.username).await;
    match exists {
        Ok(true) => {
            error!("❌ Register fallat: usuari ja existeix: {}", req.username);
            return Err(AppError::UsernameExists);
        }
        Err(e) => {
            error!("❌ Error consultant DB: {}", e);
            return Err(AppError::DatabaseUnavailable);
        }
        Ok(false) => {}
    }

    debug!("✅ Usuari no existeix, creant nou usuari");

    // Generar hash de password
    let password_hash = match hash::hash_password(&req.password) {
        Ok(h) => h,
        Err(e) => {
            error!("❌ Error generant hash: {}", e);
            return Err(AppError::InternalError);
        }
    };
    info!("✅ Password hash generat");

    // Crear usuari a DB
    let user_id = match state
        .db
        .create_user_with_role(&req.username, &password_hash, "user")
        .await
    {
        Ok(id) => id,
        Err(e) => {
            error!("❌ Error creant usuari a DB: {}", e);
            return Err(AppError::InternalError);
        }
    };
    debug!("✅ Usuari creat a DB");

    let mut is_admin = false;
    if let Some(invitation_hash) = admin_invitation_hash {
        let consumed = state
            .db
            .consume_one_admin_invitation_hash(&invitation_hash, user_id)
            .await
            .map_err(|_| AppError::InternalError)?;

        if !consumed {
            let _ = state.db.delete_user_by_id(user_id).await;
            return Err(AppError::InvitationInvalid);
        }

        let updated = state
            .db
            .update_user_role_by_id(user_id, "admin")
            .await
            .map_err(|_| AppError::InternalError)?;
        if !updated {
            return Err(AppError::InternalError);
        }

        is_admin = true;
    }

    // Crear dispositiu persistent per a l'usuari
    let device_label = format!("Dispositiu principal");
    let device_id = match state
        .db
        .upsert_device_for_user(user_id, &device_label, req.device_id)
        .await
    {
        Ok(id) => id,
        Err(e) => {
            error!("❌ Error creant dispositiu a DB: {}", e);
            return Err(AppError::InternalError);
        }
    };

    // Generar token JWT
    let claims = generate_claims(user_id, &req.username, device_id, is_admin, &state.config);
    let token = match generate_token(&claims, &state.config) {
        Ok(t) => t,
        Err(e) => {
            error!("❌ Error generant token: {}", e);
            return Err(AppError::InternalError);
        }
    };
    info!("✅ Token JWT generat");

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            user_id,
            username: req.username,
            token,
            device_id,
            device_label: Some("Dispositiu principal".to_string()),
            is_admin,
        }),
    ))
}

// ── Login ────────────────────────────────────────────────────────

#[axum::debug_handler]
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    debug!("🔑 Endpoint de login cridat");

    // Buscar usuari a DB
    let user = state.db.find_user_auth_by_username(&req.username).await;
    match user {
        Ok(Some((_user_id, _username, password_hash, is_admin))) => {
            info!("✅ Usuari trobat a DB, verificant password...");

            // Verificar password amb hash
            let is_valid = match hash::verify_password(&req.password, &password_hash) {
                Ok(v) => v,
                Err(e) => {
                    error!("❌ Error verificant password: {}", e);
                    return Err(AppError::InternalError);
                }
            };

            if !is_valid {
                debug!("❌ Login fallat: password incorrecte");
                return Err(AppError::UnauthorizedCredentials);
            }

            info!("✅ Password correcte, generant token...");

            let user_id = _user_id;

            // Recuperar o crear dispositiu persistent per a l'usuari
            let device_id = match state
                .db
                .upsert_device_for_user(user_id, "Dispositiu principal", req.device_id)
                .await
            {
                Ok(id) => id,
                Err(e) => {
                    error!("❌ Error obtenint dispositiu a DB: {}", e);
                    return Err(AppError::InternalError);
                }
            };

            // Generar token JWT
            let claims = generate_claims(user_id, &_username, device_id, is_admin, &state.config);
            let token = match generate_token(&claims, &state.config) {
                Ok(t) => t,
                Err(e) => {
                    error!("❌ Error generant token: {}", e);
                    return Err(AppError::InternalError);
                }
            };

            debug!("✅ Login exitós");

            Ok((
                StatusCode::OK,
                Json(AuthResponse {
                    user_id,
                    username: _username,
                    token,
                    device_id,
                    device_label: Some("Dispositiu principal".to_string()),
                    is_admin,
                }),
            ))
        }
        Ok(None) => {
            debug!("❌ Login fallat: usuari no trobat");
            Err(AppError::UnauthorizedCredentials)
        }
        Err(e) => {
            error!("❌ Error consultant DB: {}", e);
            Err(AppError::DatabaseUnavailable)
        }
    }
}

#[axum::debug_handler]
pub async fn register_with_invitation(
    State(state): State<AppState>,
    Json(req): Json<RegisterWithInvitationRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    let explicit_admin_invitation_hash = parse_admin_invitation_hash(req.admin_invitation_code.as_deref());

    if req.username.len() < MIN_USERNAME_LENGTH || req.username.len() > MAX_USERNAME_LENGTH {
        return Err(AppError::InvalidUsername);
    }
    if !req.username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(AppError::InvalidUsername);
    }
    if req.password.len() < MIN_PASSWORD_LENGTH {
        return Err(AppError::WeakPassword { min: MIN_PASSWORD_LENGTH });
    }

    let invitation = state
        .db
        .find_active_invitation_by_code(req.code.trim())
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    let (invitation_id, invitation_server_id, admin_invitation_hash) = match invitation {
        Some((invitation_id, invitation_server_id, max_uses, uses_count, is_active)) => {
            if !is_active {
                return Err(AppError::InvitationInvalid);
            }

            if max_uses != -1 && uses_count >= max_uses {
                return Err(AppError::InvitationExhausted);
            }

            (Some(invitation_id), invitation_server_id, explicit_admin_invitation_hash)
        }
        None => {
            let fallback_admin_invitation_hash = explicit_admin_invitation_hash
                .or_else(|| parse_admin_invitation_hash(Some(req.code.as_str())));

            let Some(admin_invitation_hash) = fallback_admin_invitation_hash else {
                return Err(AppError::InvitationInvalid);
            };

            (None, None, Some(admin_invitation_hash))
        }
    };

    let exists = state
        .db
        .user_exists(&req.username)
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    if exists {
        return Err(AppError::UsernameExists);
    }

    let password_hash = hash::hash_password(&req.password).map_err(|_| AppError::InternalError)?;
    let user_id = state
        .db
        .create_user_with_role(&req.username, &password_hash, "user")
        .await
        .map_err(|_| AppError::InternalError)?;

    let mut is_admin = false;
    if let Some(invitation_hash) = admin_invitation_hash {
        let consumed = state
            .db
            .consume_one_admin_invitation_hash(&invitation_hash, user_id)
            .await
            .map_err(|_| AppError::InternalError)?;

        if !consumed {
            let _ = state.db.delete_user_by_id(user_id).await;
            return Err(AppError::InvitationInvalid);
        }

        let updated = state
            .db
            .update_user_role_by_id(user_id, "admin")
            .await
            .map_err(|_| AppError::InternalError)?;
        if !updated {
            return Err(AppError::InternalError);
        }

        is_admin = true;
    }

    if let Some(server_id) = invitation_server_id {
        let already_member = state
            .db
            .is_server_member(server_id, user_id)
            .await
            .map_err(|_| AppError::InternalError)?
            .is_some();

        if !already_member {
            state
                .db
                .add_server_member(server_id, user_id, "member")
                .await
                .map_err(|_| AppError::InternalError)?;

            let user_servers_updated_event = serde_json::json!({
                "serverId": server_id,
                "reason": "server-joined-via-invitation",
            });
            let user_room = format!("user:{}", user_id);
            if let Err(e) = state
                .io
                .to(user_room)
                .emit("user-servers-updated", &user_servers_updated_event)
                .await
            {
                tracing::warn!("Error enviant user-servers-updated: {:?}", e);
            }

            let server_members_updated_event = serde_json::json!({
                "serverId": server_id,
                "reason": "member-added",
                "userId": user_id,
            });
            let server_room = format!("server:{}", server_id);
            if let Err(e) = state
                .io
                .to(server_room)
                .emit("server-members-updated", &server_members_updated_event)
                .await
            {
                tracing::warn!("Error enviant server-members-updated: {:?}", e);
            }
        }
    }

    let device_label = "Dispositiu principal".to_string();
    let device_id = state
        .db
        .upsert_device_for_user(user_id, &device_label, req.device_id)
        .await
        .map_err(|_| AppError::InternalError)?;

    if let Some(invitation_id) = invitation_id {
        state
            .db
            .increment_invitation_uses(invitation_id)
            .await
            .map_err(|_| AppError::InternalError)?;
    }

    let claims = generate_claims(user_id, &req.username, device_id, is_admin, &state.config);
    let token = generate_token(&claims, &state.config).map_err(|_| AppError::InternalError)?;

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            user_id,
            username: req.username,
            token,
            device_id,
            device_label: Some(device_label),
            is_admin,
        }),
    ))
}

#[axum::debug_handler]
pub async fn create_invitation(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<crate::middleware::AuthClaims>,
    Json(req): Json<CreateInvitationRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    if !claims.is_admin {
        return Err(AppError::Forbidden);
    }

    let max_uses = req.max_uses.unwrap_or(1);
    let code = generate_invitation_code();
    let invitation_id = state
        .db
        .create_invitation(&code, claims.user_id, req.server_id, max_uses)
        .await
        .map_err(|_| AppError::InternalError)?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "success": true,
            "data": {
                "invitationId": invitation_id,
                "code": code,
                "serverId": req.server_id,
                "maxUses": max_uses,
                "usesCount": 0,
                "isActive": true,
                "createdBy": claims.username,
            }
        })),
    ))
}

#[axum::debug_handler]
pub async fn list_invitations(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<crate::middleware::AuthClaims>,
) -> Result<Json<serde_json::Value>, AppError> {
    if !claims.is_admin {
        return Err(AppError::Forbidden);
    }

    let invitations = state
        .db
        .list_invitations_admin()
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    let data: Vec<InvitationListItem> = invitations
        .into_iter()
        .map(|(invitation_id, code, server_id, max_uses, uses_count, is_active, created_by)| {
            let remaining_uses = if max_uses < 0 {
                None
            } else {
                Some((max_uses - uses_count).max(0))
            };

            InvitationListItem {
                invitation_id,
                code,
                server_id,
                max_uses,
                uses_count,
                remaining_uses,
                is_active,
                created_by,
            }
        })
        .collect();

    Ok(Json(serde_json::json!({
        "success": true,
        "data": data,
    })))
}

// ── Refresh ──────────────────────────────────────────────────────

#[axum::debug_handler]
pub async fn refresh(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<RefreshResponse>, AppError> {
    let token_str = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or(AppError::TokenMissing)?;

    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.validate_exp = false;

    let old_claims = decode::<AuthClaims>(
        token_str,
        &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|_| AppError::TokenInvalid)?
    .claims;

    let active = state.db
        .is_device_active(old_claims.device_id, old_claims.user_id)
        .await
        .map_err(|_| AppError::InternalError)?;

    if !active {
        warn!("Refresh rebutjat: device {} revocat o inexistent", old_claims.device_id);
        return Err(AppError::TokenInvalid);
    }

    let new_claims = generate_claims(
        old_claims.user_id,
        &old_claims.username,
        old_claims.device_id,
        old_claims.is_admin,
        &state.config,
    );
    let token = generate_token(&new_claims, &state.config)?;
    debug!("Token renovat");
    Ok(Json(RefreshResponse { token }))
}

pub fn login_router(state: AppState) -> Router {
    Router::new()
        .route("/api/auth/login", routing::post(login))
        .route("/api/auth/refresh", routing::post(refresh))
        .with_state(state)
}

pub fn register_router(state: AppState) -> Router {
    Router::new()
        .route("/api/auth/register", routing::post(register))
        .route("/api/auth/register-with-invitation", routing::post(register_with_invitation))
        .with_state(state)
}

pub fn protected_router(state: AppState) -> Router {
    Router::new()
    .route("/api/invitations", routing::get(list_invitations).post(create_invitation))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{Config, LogLevel},
        db::connect_db,
        middleware::auth::UserPresenceState,
    };
    use axum_test::TestServer;
    use socketioxide::extract::{Data, SocketRef};
    use std::{
        collections::{HashMap, HashSet},
        sync::Arc,
    };
    use tokio::sync::{Mutex, RwLock};
    use uuid::Uuid;

    async fn make_state(open_register: bool) -> AppState {
        let config = Config {
            server_host: "127.0.0.1".to_string(),
            server_port: 8080,
            database_url: "sqlite::memory:".to_string(),
            open_register,
            admin_user: Some("admin".to_string()),
            admin_password: Some("admin-pass".to_string()),
            ttl_cleanup_interval_minutes: 5,
            livekit_host: "http://localhost:7880".to_string(),
            livekit_api_key: "test-key".to_string(),
            livekit_api_secret: "test-secret".to_string(),
            jwt_secret: "test-secret".to_string(),
            jwt_expiration_days: 7,
            backend_debug: LogLevel::Info,
            server_master_key: [7u8; 32],
            static_dir: None,
            max_file_size_bytes: 100 * 1024 * 1024,
            allowed_origins: vec![],
        };

        let db = connect_db(&config).await.expect("sqlite test db should initialize");
        let (_layer, io) = socketioxide::SocketIo::new_layer();
        io.ns("/", |_socket: SocketRef, Data(_auth): Data<serde_json::Value>| async move {});

        AppState {
            db,
            config,
            io,
            user_presence: Arc::new(RwLock::new(UserPresenceState {
                online_sockets: HashMap::<Uuid, HashSet<String>>::new(),
            })),
            livekit_token_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    #[tokio::test]
    async fn register_open_and_login_full_flow() {
        let state = make_state(true).await;
        let server = TestServer::new(router(state)).expect("router should build");

        let register_response = server
            .post("/api/auth/register")
            .json(&serde_json::json!({
                "username": "flow_user",
                "password": "password123"
            }))
            .await;

        assert_eq!(register_response.status_code(), 201);
        let body: serde_json::Value = register_response.json();
        assert_eq!(body["username"], "flow_user");
        assert!(!body["token"].as_str().unwrap_or("").is_empty(), "token should be present");

        let login_response = server
            .post("/api/auth/login")
            .json(&serde_json::json!({
                "username": "flow_user",
                "password": "password123"
            }))
            .await;

        assert_eq!(login_response.status_code(), 200);
        let login_body: serde_json::Value = login_response.json();
        assert_eq!(login_body["username"], "flow_user");
        assert!(!login_body["token"].as_str().unwrap_or("").is_empty(), "login token should be present");
    }

    #[tokio::test]
    async fn login_with_wrong_password_returns_401() {
        let state = make_state(true).await;
        let server = TestServer::new(router(state)).expect("router should build");

        server
            .post("/api/auth/register")
            .json(&serde_json::json!({
                "username": "wrong_pass_user",
                "password": "correct-password"
            }))
            .await;

        let response = server
            .post("/api/auth/login")
            .json(&serde_json::json!({
                "username": "wrong_pass_user",
                "password": "wrong-password"
            }))
            .await;

        assert_eq!(response.status_code(), 401);
    }

    #[tokio::test]
    async fn register_duplicate_username_returns_conflict() {
        let state = make_state(true).await;
        let server = TestServer::new(router(state)).expect("router should build");

        server
            .post("/api/auth/register")
            .json(&serde_json::json!({
                "username": "dup_user",
                "password": "password123"
            }))
            .await;

        let second = server
            .post("/api/auth/register")
            .json(&serde_json::json!({
                "username": "dup_user",
                "password": "password456"
            }))
            .await;

        assert_eq!(second.status_code(), 409, "duplicate username should return 409");
    }

    #[tokio::test]
    async fn register_returns_403_when_open_register_is_false() {
        let state = make_state(false).await;
        let server = TestServer::new(router(state)).expect("router should build");

        let response = server
            .post("/api/auth/register")
            .json(&serde_json::json!({
                "username": "closed_user",
                "password": "password123"
            }))
            .await;

        assert_eq!(response.status_code(), 403);
        let body: serde_json::Value = response.json();
        assert_eq!(body["error"]["code"], 1010);
    }

    #[tokio::test]
    async fn register_with_invitation_works_when_register_is_closed() {
        let state = make_state(false).await;

        let admin_hash = crate::crypto::hash::hash_password("admin-pass").expect("hash");
        let admin_id = state
            .db
            .create_user_with_role("admin", &admin_hash, "admin")
            .await
            .expect("create admin");
        state
            .db
            .create_invitation("INVITE-CODE-123", admin_id, None, 1)
            .await
            .expect("create invitation");

        let server = TestServer::new(router(state)).expect("router should build");
        let response = server
            .post("/api/auth/register-with-invitation")
            .json(&serde_json::json!({
                "code": "INVITE-CODE-123",
                "username": "invited_user",
                "password": "password123"
            }))
            .await;

        assert_eq!(response.status_code(), 201);
        let body: serde_json::Value = response.json();
        assert_eq!(body["username"], "invited_user");
        assert_eq!(body["is_admin"], false);
    }

    #[tokio::test]
    async fn register_with_exhausted_invitation_returns_410() {
        let state = make_state(false).await;

        let admin_hash = crate::crypto::hash::hash_password("admin-pass").expect("hash");
        let admin_id = state
            .db
            .create_user_with_role("admin", &admin_hash, "admin")
            .await
            .expect("create admin");
        let invitation_id = state
            .db
            .create_invitation("INVITE-ONCE", admin_id, None, 1)
            .await
            .expect("create invitation");
        state
            .db
            .increment_invitation_uses(invitation_id)
            .await
            .expect("consume invitation");

        let server = TestServer::new(router(state)).expect("router should build");
        let response = server
            .post("/api/auth/register-with-invitation")
            .json(&serde_json::json!({
                "code": "INVITE-ONCE",
                "username": "blocked_user",
                "password": "password123"
            }))
            .await;

        assert_eq!(response.status_code(), 410);
        let body: serde_json::Value = response.json();
        assert_eq!(body["error"]["code"], 1012);
    }

    #[tokio::test]
    async fn register_with_server_invitation_auto_joins_server() {
        let state = make_state(false).await;

        let admin_hash = crate::crypto::hash::hash_password("admin-pass").expect("hash");
        let admin_id = state
            .db
            .create_user_with_role("admin_owner", &admin_hash, "admin")
            .await
            .expect("create admin");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, "Server Invitacio", None, admin_id)
            .await
            .expect("create server");

        state
            .db
            .create_invitation("INVITE-SERVER-123", admin_id, Some(server_id), 1)
            .await
            .expect("create invitation");

        let server = TestServer::new(router(state.clone())).expect("router should build");
        let response = server
            .post("/api/auth/register-with-invitation")
            .json(&serde_json::json!({
                "code": "INVITE-SERVER-123",
                "username": "invited_server_user",
                "password": "password123"
            }))
            .await;

        assert_eq!(response.status_code(), 201);

        let member = state
            .db
            .find_user_by_username("invited_server_user")
            .await
            .expect("find invited user")
            .expect("invited user exists");

        let role = state
            .db
            .is_server_member(server_id, member.0)
            .await
            .expect("check server membership");

        assert_eq!(role.as_deref(), Some("member"));
    }

    #[tokio::test]
    async fn register_with_one_admin_invitation_promotes_to_admin_once() {
        let state = make_state(false).await;
        let invitation_hash = crate::crypto::hash::hash_admin_invitation_code("ONE-ADMIN-CODE");
        state
            .db
            .sync_one_admin_invitation_hash(&invitation_hash)
            .await
            .expect("seed one admin invitation");

        let server = TestServer::new(router(state.clone())).expect("router should build");

        let first = server
            .post("/api/auth/register")
            .json(&serde_json::json!({
                "username": "bootstrap_admin",
                "password": "password123",
                "admin_invitation_code": "ONE-ADMIN-CODE"
            }))
            .await;

        assert_eq!(first.status_code(), 201);
        let first_body: serde_json::Value = first.json();
        assert_eq!(first_body["is_admin"], true);

        let second = server
            .post("/api/auth/register")
            .json(&serde_json::json!({
                "username": "second_user",
                "password": "password123",
                "admin_invitation_code": "ONE-ADMIN-CODE"
            }))
            .await;

        assert_eq!(second.status_code(), 404);
        let second_body: serde_json::Value = second.json();
        assert_eq!(second_body["error"]["code"], 1011);
    }

    #[tokio::test]
    async fn register_with_invalid_admin_invitation_when_closed_does_not_create_user() {
        let state = make_state(false).await;
        let server = TestServer::new(router(state.clone())).expect("router should build");

        let response = server
            .post("/api/auth/register")
            .json(&serde_json::json!({
                "username": "should_not_exist",
                "password": "password123",
                "admin_invitation_code": "INVALID-CODE"
            }))
            .await;

        assert_eq!(response.status_code(), 404);

        let exists = state
            .db
            .user_exists("should_not_exist")
            .await
            .expect("query user");
        assert!(!exists);
    }

    #[tokio::test]
    async fn register_with_invitation_accepts_one_admin_invitation_in_code_field() {
        let state = make_state(false).await;
        let invitation_hash = crate::crypto::hash::hash_admin_invitation_code("CODI-UNIC-ADMIN");
        state
            .db
            .sync_one_admin_invitation_hash(&invitation_hash)
            .await
            .expect("seed one admin invitation");

        let server = TestServer::new(router(state)).expect("router should build");
        let response = server
            .post("/api/auth/register-with-invitation")
            .json(&serde_json::json!({
                "code": "CODI-UNIC-ADMIN",
                "username": "agus",
                "password": "12345678"
            }))
            .await;

        assert_eq!(response.status_code(), 201);
        let body: serde_json::Value = response.json();
        assert_eq!(body["username"], "agus");
        assert_eq!(body["is_admin"], true);
    }

    #[tokio::test]
    async fn register_username_too_short_returns_422() {
        let state = make_state(true).await;
        let server = TestServer::new(router(state)).expect("router should build");

        let response = server
            .post("/api/auth/register")
            .json(&serde_json::json!({
                "username": "ab",
                "password": "password123"
            }))
            .await;

        assert_eq!(response.status_code(), 422, "username too short should return 422, not 409");
        let body: serde_json::Value = response.json();
        assert_eq!(body["error"]["code"], 1013);
    }

    #[tokio::test]
    async fn register_username_too_long_returns_422() {
        let state = make_state(true).await;
        let server = TestServer::new(router(state)).expect("router should build");

        let response = server
            .post("/api/auth/register")
            .json(&serde_json::json!({
                "username": "a".repeat(100),
                "password": "password123"
            }))
            .await;

        assert_eq!(response.status_code(), 422, "username too long should return 422, not 409");
        let body: serde_json::Value = response.json();
        assert_eq!(body["error"]["code"], 1013);
    }

    #[tokio::test]
    async fn register_username_invalid_chars_returns_422() {
        let state = make_state(true).await;
        let server = TestServer::new(router(state)).expect("router should build");

        let response = server
            .post("/api/auth/register")
            .json(&serde_json::json!({
                "username": "user name!",
                "password": "password123"
            }))
            .await;

        assert_eq!(response.status_code(), 422, "username with invalid chars should return 422, not 409");
        let body: serde_json::Value = response.json();
        assert_eq!(body["error"]["code"], 1013);
    }
}