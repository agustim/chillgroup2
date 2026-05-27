//! Endpoints d'autenticació: register, login.

use axum::{
    extract::State,
    http::StatusCode,
    Json,
    routing,
    Router,
};
use rand::{distributions::Alphanumeric, Rng};
use shared::types::{AuthResponse, RefreshResponse};
use serde::Deserialize;
use uuid::Uuid;
use tracing::{info, error, warn};
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
}

#[derive(Debug, Deserialize)]
pub struct CreateInvitationRequest {
    pub max_uses: Option<i32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvitationListItem {
    pub invitation_id: Uuid,
    pub code: String,
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

// ── Register ─────────────────────────────────────────────────────

#[axum::debug_handler]
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    info!("📝 Endpoint de register cridat per username: {}", req.username);

    if !state.config.open_register {
        warn!("❌ Register rebutjat: OPEN_REGISTER=false");
        return Err(AppError::RegistrationClosed);
    }

    // Validar username
    if req.username.len() < MIN_USERNAME_LENGTH || req.username.len() > MAX_USERNAME_LENGTH {
        error!("❌ Register fallat: username amb longitud invàlida ({})", req.username);
        return Err(AppError::UsernameExists);
    }
    if !req.username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        error!("❌ Register fallat: username amb caràcters invàlids");
        return Err(AppError::UsernameExists);
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

    info!("✅ Usuari no existeix, creant nou usuari: {}", req.username);

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
    info!("✅ Usuari creat a DB amb user_id={}", user_id);

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
    let claims = generate_claims(user_id, &req.username, device_id, false, &state.config);
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
            is_admin: false,
        }),
    ))
}

// ── Login ────────────────────────────────────────────────────────

#[axum::debug_handler]
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    info!("🔑 Endpoint de login cridat per username: {}", req.username);

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
                warn!("❌ Login fallat: password incorrecte per username: {}", req.username);
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

            info!("✅ Login exitós: user_id={}, device_id={}, username={}", user_id, device_id, _username);

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
            warn!("❌ Login fallat: usuari no trobat: {}", req.username);
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
    if req.username.len() < MIN_USERNAME_LENGTH || req.username.len() > MAX_USERNAME_LENGTH {
        return Err(AppError::UsernameExists);
    }
    if !req.username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(AppError::UsernameExists);
    }
    if req.password.len() < MIN_PASSWORD_LENGTH {
        return Err(AppError::WeakPassword { min: MIN_PASSWORD_LENGTH });
    }

    let invitation = state
        .db
        .find_active_invitation_by_code(req.code.trim())
        .await
        .map_err(|_| AppError::DatabaseUnavailable)?;

    let Some((invitation_id, max_uses, uses_count, is_active)) = invitation else {
        return Err(AppError::InvitationInvalid);
    };

    if !is_active {
        return Err(AppError::InvitationInvalid);
    }

    if max_uses != -1 && uses_count >= max_uses {
        return Err(AppError::InvitationExhausted);
    }

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

    let device_label = "Dispositiu principal".to_string();
    let device_id = state
        .db
        .upsert_device_for_user(user_id, &device_label, req.device_id)
        .await
        .map_err(|_| AppError::InternalError)?;

    state
        .db
        .increment_invitation_uses(invitation_id)
        .await
        .map_err(|_| AppError::InternalError)?;

    let claims = generate_claims(user_id, &req.username, device_id, false, &state.config);
    let token = generate_token(&claims, &state.config).map_err(|_| AppError::InternalError)?;

    Ok((
        StatusCode::CREATED,
        Json(AuthResponse {
            user_id,
            username: req.username,
            token,
            device_id,
            device_label: Some(device_label),
            is_admin: false,
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
        .create_invitation(&code, claims.user_id, max_uses)
        .await
        .map_err(|_| AppError::InternalError)?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "success": true,
            "data": {
                "invitationId": invitation_id,
                "code": code,
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
        .map(|(invitation_id, code, max_uses, uses_count, is_active, created_by)| {
            let remaining_uses = if max_uses < 0 {
                None
            } else {
                Some((max_uses - uses_count).max(0))
            };

            InvitationListItem {
                invitation_id,
                code,
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
    State(_state): State<AppState>,
) -> Result<Json<RefreshResponse>, AppError> {
    let user_id = Uuid::new_v4();
    let device_id = Uuid::new_v4();
    let claims = generate_claims(user_id, "user", device_id, false, &_state.config);
    let token = generate_token(&claims, &_state.config)?;
    Ok(Json(RefreshResponse { token }))
}

// ── Router ───────────────────────────────────────────────────────

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/auth/register", routing::post(register))
        .route("/api/auth/register-with-invitation", routing::post(register_with_invitation))
        .route("/api/auth/login", routing::post(login))
        .route("/api/auth/refresh", routing::post(refresh))
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
    use std::{
        collections::{HashMap, HashSet},
        sync::Arc,
    };
    use tokio::sync::RwLock;
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
        };

        let db = connect_db(&config).await.expect("sqlite test db should initialize");
        let (_layer, io) = socketioxide::SocketIo::new_layer();

        AppState {
            db,
            config,
            io,
            user_presence: Arc::new(RwLock::new(UserPresenceState {
                online_sockets: HashMap::<Uuid, HashSet<String>>::new(),
            })),
        }
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
            .create_invitation("INVITE-CODE-123", admin_id, 1)
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
            .create_invitation("INVITE-ONCE", admin_id, 1)
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
}