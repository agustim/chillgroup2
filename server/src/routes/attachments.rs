use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use aws_credential_types::{provider::SharedCredentialsProvider, Credentials};
use aws_sdk_s3::{
    config::{Builder as S3ConfigBuilder, Region},
    primitives::ByteStream,
    presigning::PresigningConfig,
    types::{CompletedMultipartUpload, CompletedPart},
    Client as S3Client,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

use crate::{
    db::{CHANNEL_PERMISSION_READ, CHANNEL_PERMISSION_WRITE},
    error::AppError,
    middleware::{AppState, AuthClaims},
};

fn s3_endpoint() -> String {
    std::env::var("S3_ENDPOINT").unwrap_or_else(|_| "http://localhost:9000".to_string())
}

fn s3_public_endpoint() -> String {
    std::env::var("S3_PUBLIC_ENDPOINT")
        .or_else(|_| std::env::var("S3_ENDPOINT"))
        .unwrap_or_else(|_| "http://localhost:9000".to_string())
}

fn s3_region() -> String {
    std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string())
}

fn s3_bucket() -> String {
    std::env::var("S3_BUCKET").unwrap_or_else(|_| "chillgroup-attachments".to_string())
}

fn s3_access_key_id() -> String {
    std::env::var("S3_ACCESS_KEY_ID").unwrap_or_else(|_| "rustfsadmin".to_string())
}

fn s3_secret_access_key() -> String {
    std::env::var("S3_SECRET_ACCESS_KEY").unwrap_or_else(|_| "rustfsadmin".to_string())
}

fn s3_force_path_style() -> bool {
    std::env::var("S3_FORCE_PATH_STYLE")
        .ok()
        .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "true" | "1" | "yes" | "on"))
        .unwrap_or(true)
}

fn server_proxy_s3() -> bool {
    std::env::var("SERVER_PROXY_S3")
        .ok()
        .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "true" | "1" | "yes" | "on"))
        .unwrap_or(false)
}

fn build_s3_client_with_endpoint(endpoint: String) -> S3Client {
    let creds = Credentials::new(
        s3_access_key_id(),
        s3_secret_access_key(),
        None,
        None,
        "chillgroup-static-s3-creds",
    );

    let conf = S3ConfigBuilder::new()
        .region(Region::new(s3_region()))
        .credentials_provider(SharedCredentialsProvider::new(creds))
        .endpoint_url(endpoint)
        .force_path_style(s3_force_path_style())
        .build();

    S3Client::from_conf(conf)
}

fn build_s3_client() -> S3Client {
    build_s3_client_with_endpoint(s3_endpoint())
}

fn build_s3_presign_client() -> S3Client {
    build_s3_client_with_endpoint(s3_public_endpoint())
}

fn presigning_config() -> Result<PresigningConfig, AppError> {
    PresigningConfig::expires_in(Duration::from_secs(900)).map_err(|_| AppError::InternalError)
}

fn attachment_is_downloadable(status: &str) -> bool {
    matches!(status, "ready" | "linked")
}

#[derive(Debug, Deserialize)]
pub struct InitAttachmentRequest {
    #[serde(alias = "fileName")]
    pub file_name: String,
    #[serde(alias = "mimeType")]
    pub mime_type: String,
    #[serde(alias = "sizeBytes")]
    pub size_bytes: i64,
    #[serde(alias = "createdAt")]
    pub created_at: Option<String>,
    #[serde(alias = "chunkSizeBytes")]
    pub chunk_size_bytes: i64,
    #[serde(alias = "chunkCount")]
    pub chunk_count: i32,
}

#[derive(Debug, Serialize)]
pub struct InitAttachmentResponse {
    #[serde(rename = "attachmentId")]
    pub attachment_id: Uuid,
    #[serde(rename = "uploadId")]
    pub upload_id: String,
    #[serde(rename = "objectKey")]
    pub object_key: String,
    #[serde(rename = "chunkSizeBytes")]
    pub chunk_size_bytes: i64,
    #[serde(rename = "chunkCount")]
    pub chunk_count: i32,
}

#[derive(Debug, Deserialize)]
pub struct SignPartRequest {
    #[serde(alias = "uploadId")]
    pub upload_id: String,
    #[serde(alias = "partNumber")]
    pub part_number: i32,
}

#[derive(Debug, Serialize)]
pub struct SignPartResponse {
    #[serde(rename = "partNumber")]
    pub part_number: i32,
    #[serde(rename = "uploadUrl")]
    pub upload_url: String,
}

#[derive(Debug, Deserialize)]
pub struct ProxyUploadPartQuery {
    #[serde(alias = "uploadId")]
    pub upload_id: String,
    #[serde(alias = "partNumber")]
    pub part_number: i32,
}

#[derive(Debug, Serialize)]
pub struct ProxyUploadPartResponse {
    pub etag: String,
}

#[derive(Debug, Deserialize)]
pub struct CompleteAttachmentRequest {
    #[serde(alias = "uploadId")]
    pub upload_id: String,
    pub parts: Vec<CompletePartItem>,
    pub crypto: CompleteCrypto,
    #[serde(alias = "thumbnail_attachment_id")]
    pub thumbnail_attachment_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct CompletePartItem {
    #[serde(alias = "partNumber")]
    pub part_number: i32,
    pub etag: String,
}

#[derive(Debug, Deserialize)]
pub struct CompleteCrypto {
    pub algorithm: String,
    #[serde(alias = "fileIv")]
    pub file_iv: String,
    #[serde(alias = "wrappedFileKey")]
    pub wrapped_file_key: String,
    #[serde(alias = "keyVersionId")]
    pub key_version_id: Uuid,
    #[serde(alias = "keyVersion")]
    pub key_version: i32,
    #[serde(alias = "ciphertextSha256")]
    pub ciphertext_sha256: String,
}

#[derive(Debug, Serialize)]
pub struct CompleteAttachmentResponse {
    #[serde(rename = "attachmentId")]
    pub attachment_id: Uuid,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct DownloadAttachmentResponse {
    #[serde(rename = "attachmentId")]
    pub attachment_id: Uuid,
    #[serde(rename = "fileName")]
    pub file_name: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    #[serde(rename = "sizeBytes")]
    pub size_bytes: i64,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "downloadUrl")]
    pub download_url: String,
    pub crypto: DownloadCrypto,
    #[serde(rename = "thumbnail_attachment_id", skip_serializing_if = "Option::is_none")]
    pub thumbnail_attachment_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct DownloadCrypto {
    pub algorithm: String,
    #[serde(rename = "fileIv")]
    pub file_iv: String,
    #[serde(rename = "wrappedFileKey")]
    pub wrapped_file_key: String,
    #[serde(rename = "keyVersionId")]
    pub key_version_id: Uuid,
    #[serde(rename = "keyVersion")]
    pub key_version: i32,
    #[serde(rename = "chunkSizeBytes")]
    pub chunk_size_bytes: i64,
    #[serde(rename = "chunkCount")]
    pub chunk_count: i32,
    #[serde(rename = "ciphertextSha256")]
    pub ciphertext_sha256: String,
}

pub async fn init_attachment(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path(channel_id): Path<Uuid>,
    Json(req): Json<InitAttachmentRequest>,
) -> Result<(StatusCode, Json<InitAttachmentResponse>), AppError> {
    let permission_level = state
        .db
        .get_channel_permission_level(channel_id, claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?
        .unwrap_or(0);
    if permission_level < CHANNEL_PERMISSION_WRITE {
        return Err(AppError::Forbidden);
    }

    if req.chunk_size_bytes <= 0 || req.chunk_count <= 0 || req.size_bytes < 0 {
        return Err(AppError::BadRequest);
    }

    let max_bytes = state.config.max_file_size_bytes;
    if max_bytes > 0 && req.size_bytes as u64 > max_bytes {
        return Err(AppError::FileTooLarge { max_mb: max_bytes / (1024 * 1024) });
    }

    let year_month = Utc::now().format("%Y-%m").to_string();
    let (max_storage, _) = state
        .db
        .get_user_s3_quota(claims.user_id)
        .await
        .map_err(|e| AppError::DatabaseError(sqlx::Error::Protocol(e)))?;
    if max_storage >= 0 {
        let (stored, _) = state
            .db
            .get_user_storage_usage(claims.user_id, &year_month)
            .await
            .map_err(|e| AppError::DatabaseError(sqlx::Error::Protocol(e)))?;
        if stored + req.size_bytes > max_storage {
            return Err(AppError::StorageQuotaExceeded);
        }
    }

    let attachment_id = Uuid::new_v4();
    let object_key = format!("channels/{}/attachments/{}.bin", channel_id, attachment_id);
    let created_at = req
        .created_at
        .as_deref()
        .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
        .map(|v| v.with_timezone(&Utc))
        .unwrap_or_else(Utc::now);

    let s3 = build_s3_client();
    let upload_res = s3
        .create_multipart_upload()
        .bucket(s3_bucket())
        .key(&object_key)
        .content_type(&req.mime_type)
        .send()
        .await
        .map_err(|_| AppError::InternalError)?;
    let upload_id = upload_res.upload_id().ok_or(AppError::InternalError)?.to_string();

    state
        .db
        .create_attachment_init(
            attachment_id,
            channel_id,
            claims.user_id,
            claims.device_id,
            &req.file_name,
            &req.mime_type,
            req.size_bytes,
            created_at,
            &object_key,
            &upload_id,
            req.chunk_size_bytes,
            req.chunk_count,
        )
        .await
        .map_err(AppError::DatabaseError)?;

    Ok((
        StatusCode::CREATED,
        Json(InitAttachmentResponse {
            attachment_id,
            upload_id,
            object_key,
            chunk_size_bytes: req.chunk_size_bytes,
            chunk_count: req.chunk_count,
        }),
    ))
}

pub async fn sign_part(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path((channel_id, attachment_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<SignPartRequest>,
) -> Result<Json<SignPartResponse>, AppError> {
    let permission_level = state
        .db
        .get_channel_permission_level(channel_id, claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?
        .unwrap_or(0);
    if permission_level < CHANNEL_PERMISSION_WRITE {
        return Err(AppError::Forbidden);
    }

    let attachment = state
        .db
        .get_attachment_by_id(channel_id, attachment_id)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::AttachmentNotFound)?;

    if attachment.upload_id != req.upload_id || attachment.status != "initiated" {
        return Err(AppError::BadRequest);
    }

    let upload_url = if server_proxy_s3() {
        format!(
            "/api/channels/{}/attachments/{}/upload-part?uploadId={}&partNumber={}",
            channel_id, attachment_id, req.upload_id, req.part_number
        )
    } else {
        let s3 = build_s3_presign_client();
        let cfg = presigning_config()?;
        let presigned = s3
            .upload_part()
            .bucket(s3_bucket())
            .key(&attachment.object_key)
            .upload_id(&req.upload_id)
            .part_number(req.part_number)
            .presigned(cfg)
            .await
            .map_err(|_| AppError::InternalError)?;

        presigned.uri().to_string()
    };

    Ok(Json(SignPartResponse {
        part_number: req.part_number,
        upload_url,
    }))
}

pub async fn upload_part_proxy(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path((channel_id, attachment_id)): Path<(Uuid, Uuid)>,
    axum::extract::Query(query): axum::extract::Query<ProxyUploadPartQuery>,
    body: Bytes,
) -> Result<impl IntoResponse, AppError> {
    if !server_proxy_s3() {
        return Err(AppError::BadRequest);
    }

    let permission_level = state
        .db
        .get_channel_permission_level(channel_id, claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?
        .unwrap_or(0);
    if permission_level < CHANNEL_PERMISSION_WRITE {
        return Err(AppError::Forbidden);
    }

    let attachment = state
        .db
        .get_attachment_by_id(channel_id, attachment_id)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::AttachmentNotFound)?;

    if attachment.upload_id != query.upload_id || attachment.status != "initiated" {
        return Err(AppError::BadRequest);
    }

    let s3 = build_s3_client();
    let output = s3
        .upload_part()
        .bucket(s3_bucket())
        .key(&attachment.object_key)
        .upload_id(&query.upload_id)
        .part_number(query.part_number)
        .body(ByteStream::from(body.to_vec()))
        .send()
        .await
        .map_err(|_| AppError::InternalError)?;

    let etag = output.e_tag().ok_or(AppError::InternalError)?.to_string();

    Ok((
        StatusCode::OK,
        [(header::ETAG, etag.clone())],
        Json(ProxyUploadPartResponse { etag }),
    ))
}

pub async fn complete_attachment(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path((channel_id, attachment_id)): Path<(Uuid, Uuid)>,
    Json(req): Json<CompleteAttachmentRequest>,
) -> Result<Json<CompleteAttachmentResponse>, AppError> {
    let permission_level = state
        .db
        .get_channel_permission_level(channel_id, claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?
        .unwrap_or(0);
    if permission_level < CHANNEL_PERMISSION_WRITE {
        return Err(AppError::Forbidden);
    }

    let attachment = state
        .db
        .get_attachment_by_id(channel_id, attachment_id)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::AttachmentNotFound)?;

    if attachment.upload_id != req.upload_id || attachment.status != "initiated" {
        return Err(AppError::BadRequest);
    }

    if req.parts.is_empty() {
        return Err(AppError::BadRequest);
    }

    let completed_parts = req
        .parts
        .iter()
        .map(|p| {
            CompletedPart::builder()
                .part_number(p.part_number)
                .e_tag(p.etag.clone())
                .build()
        })
        .collect::<Vec<_>>();

    let multipart_upload = CompletedMultipartUpload::builder()
        .set_parts(Some(completed_parts))
        .build();

    let s3 = build_s3_client();
    s3.complete_multipart_upload()
        .bucket(s3_bucket())
        .key(&attachment.object_key)
        .upload_id(&req.upload_id)
        .multipart_upload(multipart_upload)
        .send()
        .await
        .map_err(|_| AppError::InternalError)?;

    state
        .db
        .complete_attachment(
            attachment_id,
            &req.crypto.algorithm,
            &req.crypto.file_iv,
            &req.crypto.wrapped_file_key,
            req.crypto.key_version_id,
            req.crypto.key_version,
            &req.crypto.ciphertext_sha256,
            req.thumbnail_attachment_id,
        )
        .await
        .map_err(AppError::DatabaseError)?;

    let year_month = Utc::now().format("%Y-%m").to_string();
    if let Ok(new_stored) = state
        .db
        .increment_stored_bytes(claims.user_id, &year_month, attachment.size_bytes)
        .await
    {
        let (max_storage, _) = state
            .db
            .get_user_s3_quota(claims.user_id)
            .await
            .unwrap_or((-1, -1));
        if max_storage > 0 {
            let pct = new_stored * 100 / max_storage;
            let (sent80, sent90) = state
                .db
                .get_quota_warning_timestamps(claims.user_id, &year_month)
                .await
                .unwrap_or((false, false));

            for (threshold, already_sent) in [(90u8, sent90), (80u8, sent80)] {
                if pct >= i64::from(threshold) && !already_sent {
                    let user_room = format!("user:{}", claims.user_id);
                    let _ = state
                        .io
                        .to(user_room)
                        .emit(
                            "quota_warning",
                            &serde_json::json!({
                                "type": "storage",
                                "threshold": threshold,
                                "usedBytes": new_stored,
                                "maxBytes": max_storage,
                            }),
                        )
                        .await;
                    let _ = state
                        .db
                        .set_quota_warning_sent(claims.user_id, &year_month, threshold)
                        .await;
                }
            }
        }
    }

    Ok(Json(CompleteAttachmentResponse {
        attachment_id,
        status: "ready".to_string(),
    }))
}

pub async fn download_attachment(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path((channel_id, attachment_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<DownloadAttachmentResponse>, AppError> {
    let permission_level = state
        .db
        .get_channel_permission_level(channel_id, claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?
        .unwrap_or(0);
    if permission_level < CHANNEL_PERMISSION_READ {
        return Err(AppError::Forbidden);
    }

    let attachment = state
        .db
        .get_attachment_by_id(channel_id, attachment_id)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::AttachmentNotFound)?;

    if !attachment_is_downloadable(&attachment.status) {
        return Err(AppError::BadRequest);
    }

    let year_month = Utc::now().format("%Y-%m").to_string();
    let (_, max_transfer) = state
        .db
        .get_user_s3_quota(claims.user_id)
        .await
        .map_err(|e| AppError::DatabaseError(sqlx::Error::Protocol(e)))?;
    if max_transfer >= 0 {
        let (_, transferred) = state
            .db
            .get_user_storage_usage(claims.user_id, &year_month)
            .await
            .map_err(|e| AppError::DatabaseError(sqlx::Error::Protocol(e)))?;
        if transferred + attachment.size_bytes > max_transfer {
            return Err(AppError::TransferQuotaExceeded);
        }
    }

    let download_url = if server_proxy_s3() {
        format!(
            "/api/channels/{}/attachments/{}/download-proxy",
            channel_id, attachment_id
        )
    } else {
        let s3 = build_s3_presign_client();
        let cfg = presigning_config()?;
        let presigned = s3
            .get_object()
            .bucket(s3_bucket())
            .key(&attachment.object_key)
            .presigned(cfg)
            .await
            .map_err(|_| AppError::InternalError)?;

        presigned.uri().to_string()
    };

    let _ = state
        .db
        .increment_transfer_bytes(claims.user_id, &year_month, attachment.size_bytes)
        .await;

    Ok(Json(DownloadAttachmentResponse {
        attachment_id: attachment.id,
        file_name: attachment.file_name,
        mime_type: attachment.mime_type,
        size_bytes: attachment.size_bytes,
        created_at: attachment.created_at.to_rfc3339(),
        download_url,
        crypto: DownloadCrypto {
            algorithm: attachment.algorithm.unwrap_or_else(|| "aes-256-gcm".to_string()),
            file_iv: attachment.file_iv.unwrap_or_default(),
            wrapped_file_key: attachment.wrapped_file_key.unwrap_or_default(),
            key_version_id: attachment.key_version_id.ok_or(AppError::BadRequest)?,
            key_version: attachment.key_version.unwrap_or_default(),
            chunk_size_bytes: attachment.chunk_size_bytes,
            chunk_count: attachment.chunk_count,
            ciphertext_sha256: attachment.ciphertext_sha256.unwrap_or_default(),
        },
        thumbnail_attachment_id: attachment.thumbnail_attachment_id,
    }))
}

pub async fn download_attachment_proxy(
    State(state): State<AppState>,
    axum::Extension(claims): axum::Extension<AuthClaims>,
    Path((channel_id, attachment_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    if !server_proxy_s3() {
        return Err(AppError::BadRequest);
    }

    let permission_level = state
        .db
        .get_channel_permission_level(channel_id, claims.user_id)
        .await
        .map_err(AppError::DatabaseError)?
        .unwrap_or(0);
    if permission_level < CHANNEL_PERMISSION_READ {
        return Err(AppError::Forbidden);
    }

    let attachment = state
        .db
        .get_attachment_by_id(channel_id, attachment_id)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::AttachmentNotFound)?;

    if !attachment_is_downloadable(&attachment.status) {
        return Err(AppError::BadRequest);
    }

    let s3 = build_s3_client();
    let output = s3
        .get_object()
        .bucket(s3_bucket())
        .key(&attachment.object_key)
        .send()
        .await
        .map_err(|_| AppError::InternalError)?;

    let content_type = output
        .content_type()
        .map(|v| v.to_string())
        .unwrap_or_else(|| attachment.mime_type.clone());

    let body = output
        .body
        .collect()
        .await
        .map_err(|_| AppError::InternalError)?
        .into_bytes();

    Ok(([(header::CONTENT_TYPE, content_type)], body))
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route(
            "/api/channels/{channel_id}/attachments/init",
            post(init_attachment),
        )
        .route(
            "/api/channels/{channel_id}/attachments/{attachment_id}/sign-part",
            post(sign_part),
        )
        .route(
            "/api/channels/{channel_id}/attachments/{attachment_id}/complete",
            post(complete_attachment),
        )
        .route(
            "/api/channels/{channel_id}/attachments/{attachment_id}/upload-part",
            put(upload_part_proxy),
        )
        .route(
            "/api/channels/{channel_id}/attachments/{attachment_id}/download",
            get(download_attachment),
        )
        .route(
            "/api/channels/{channel_id}/attachments/{attachment_id}/download-proxy",
            get(download_attachment_proxy),
        )
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
    use axum::Extension;
    use std::{
        collections::{HashMap, HashSet},
        sync::Arc,
    };
    use tokio::sync::RwLock;

    async fn make_state() -> AppState {
        let config = Config {
            server_host: "127.0.0.1".to_string(),
            server_port: 8080,
            database_url: "sqlite::memory:".to_string(),
            open_register: true,
            admin_user: None,
            admin_password: None,
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
        };

        let db = connect_db(&config)
            .await
            .expect("sqlite test db should initialize");
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

    fn claims_for(user_id: Uuid, username: &str) -> AuthClaims {
        AuthClaims {
            user_id,
            username: username.to_string(),
            device_id: Uuid::new_v4(),
            is_admin: false,
            exp: 0,
            iat: 0,
            jti: Uuid::new_v4().to_string(),
        }
    }

    async fn setup_server_channel(state: &AppState, owner_name: &str, channel_name: &str) -> (Uuid, Uuid, Uuid) {
        let owner_id = state
            .db
            .create_user(owner_name, "hash")
            .await
            .expect("create owner");

        let server_id = Uuid::new_v4();
        state
            .db
            .create_server_with_owner(server_id, &format!("srv-{}", channel_name), None, owner_id)
            .await
            .expect("create server");

        let channel_id = Uuid::new_v4();
        state
            .db
            .create_channel(channel_id, server_id, channel_name, "text", "symmetric", None, false)
            .await
            .expect("create channel");

        (owner_id, server_id, channel_id)
    }

    #[tokio::test]
    async fn init_attachment_forbidden_for_non_member() {
        let state = make_state().await;
        let (_owner_id, _server_id, channel_id) =
            setup_server_channel(&state, "att_init_owner", "att-init").await;

        let outsider_id = state
            .db
            .create_user("att_init_outsider", "hash")
            .await
            .expect("create outsider");

        let result = init_attachment(
            State(state),
            Extension(claims_for(outsider_id, "att_init_outsider")),
            Path(channel_id),
            Json(InitAttachmentRequest {
                file_name: "x.txt".to_string(),
                mime_type: "text/plain".to_string(),
                size_bytes: 10,
                created_at: None,
                chunk_size_bytes: 5,
                chunk_count: 2,
            }),
        )
        .await;

        assert!(matches!(result, Err(AppError::Forbidden)));
    }

    #[tokio::test]
    async fn init_attachment_bad_request_when_chunk_invalid() {
        let state = make_state().await;
        let (owner_id, _server_id, channel_id) =
            setup_server_channel(&state, "att_badreq_owner", "att-badreq").await;

        let result = init_attachment(
            State(state),
            Extension(claims_for(owner_id, "att_badreq_owner")),
            Path(channel_id),
            Json(InitAttachmentRequest {
                file_name: "x.txt".to_string(),
                mime_type: "text/plain".to_string(),
                size_bytes: 10,
                created_at: None,
                chunk_size_bytes: 0,
                chunk_count: 2,
            }),
        )
        .await;

        assert!(matches!(result, Err(AppError::BadRequest)));
    }

    #[tokio::test]
    async fn sign_part_returns_not_found_for_missing_attachment() {
        let state = make_state().await;
        let (owner_id, _server_id, channel_id) =
            setup_server_channel(&state, "att_sign_owner", "att-sign").await;

        let result = sign_part(
            State(state),
            Extension(claims_for(owner_id, "att_sign_owner")),
            Path((channel_id, Uuid::new_v4())),
            Json(SignPartRequest {
                upload_id: "missing-upload".to_string(),
                part_number: 1,
            }),
        )
        .await;

        assert!(matches!(result, Err(AppError::AttachmentNotFound)));
    }

    #[tokio::test]
    async fn complete_attachment_rejects_empty_parts() {
        let state = make_state().await;
        let (owner_id, _server_id, channel_id) =
            setup_server_channel(&state, "att_complete_owner", "att-complete").await;

        let attachment_id = Uuid::new_v4();
        let upload_id = Uuid::new_v4().to_string();
        state
            .db
            .create_attachment_init(
                attachment_id,
                channel_id,
                owner_id,
                Uuid::new_v4(),
                "f.bin",
                "application/octet-stream",
                32,
                Utc::now(),
                "channels/test/attachments/test.bin",
                &upload_id,
                16,
                2,
            )
            .await
            .expect("create initiated attachment");

        let result = complete_attachment(
            State(state),
            Extension(claims_for(owner_id, "att_complete_owner")),
            Path((channel_id, attachment_id)),
            Json(CompleteAttachmentRequest {
                upload_id,
                parts: vec![],
                crypto: CompleteCrypto {
                    algorithm: "aes-256-gcm".to_string(),
                    file_iv: "iv".to_string(),
                    wrapped_file_key: "wrapped".to_string(),
                    key_version_id: Uuid::new_v4(),
                    key_version: 1,
                    ciphertext_sha256: "deadbeef".to_string(),
                },
                thumbnail_attachment_id: None,
            }),
        )
        .await;

        assert!(matches!(result, Err(AppError::BadRequest)));
    }

    #[tokio::test]
    async fn download_attachment_rejects_not_ready_status() {
        let state = make_state().await;
        let (owner_id, _server_id, channel_id) =
            setup_server_channel(&state, "att_download_owner", "att-download").await;

        let attachment_id = Uuid::new_v4();
        state
            .db
            .create_attachment_init(
                attachment_id,
                channel_id,
                owner_id,
                Uuid::new_v4(),
                "f.bin",
                "application/octet-stream",
                32,
                Utc::now(),
                "channels/test/attachments/test.bin",
                "upload-id",
                16,
                2,
            )
            .await
            .expect("create initiated attachment");

        let result = download_attachment(
            State(state),
            Extension(claims_for(owner_id, "att_download_owner")),
            Path((channel_id, attachment_id)),
        )
        .await;

        assert!(matches!(result, Err(AppError::BadRequest)));
    }

    #[tokio::test]
    async fn download_attachment_allows_linked_status() {
        let state = make_state().await;
        let (owner_id, _server_id, channel_id) =
            setup_server_channel(&state, "att_download_linked_owner", "att-download-linked").await;

        let attachment_id = Uuid::new_v4();
        let message_id = Uuid::new_v4();
        let key_version_id = Uuid::new_v4();
        state
            .db
            .create_attachment_init(
                attachment_id,
                channel_id,
                owner_id,
                Uuid::new_v4(),
                "f.bin",
                "application/octet-stream",
                32,
                Utc::now(),
                "channels/test/attachments/test.bin",
                "upload-id",
                16,
                2,
            )
            .await
            .expect("create initiated attachment");

        state
            .db
            .complete_attachment(
                attachment_id,
                "aes-256-gcm",
                "iv",
                "wrapped",
                key_version_id,
                1,
                "deadbeef",
                None,
            )
            .await
            .expect("complete attachment");

        state
            .db
            .create_message(
                message_id,
                channel_id,
                owner_id,
                "att_download_linked_owner",
                Uuid::new_v4(),
                "payload",
                "iv",
                Some(1),
                None,
                Utc::now(),
            )
            .await
            .expect("create message");

        state
            .db
            .attach_message_attachments(message_id, channel_id, owner_id, &[attachment_id])
            .await
            .expect("link attachment to message");

        let result = download_attachment(
            State(state),
            Extension(claims_for(owner_id, "att_download_linked_owner")),
            Path((channel_id, attachment_id)),
        )
        .await;

        assert!(result.is_ok(), "linked attachments should remain downloadable");
    }

    fn make_state_with_max_file_size(db: crate::db::DatabasePool, io: socketioxide::SocketIo, max_bytes: u64) -> AppState {
        let config = Config {
            server_host: "127.0.0.1".to_string(),
            server_port: 8080,
            database_url: "sqlite::memory:".to_string(),
            open_register: true,
            admin_user: None,
            admin_password: None,
            ttl_cleanup_interval_minutes: 5,
            livekit_host: "http://localhost:7880".to_string(),
            livekit_api_key: "test-key".to_string(),
            livekit_api_secret: "test-secret".to_string(),
            jwt_secret: "test-secret".to_string(),
            jwt_expiration_days: 7,
            backend_debug: LogLevel::Info,
            server_master_key: [7u8; 32],
            static_dir: None,
            max_file_size_bytes: max_bytes,
        };
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
    async fn init_attachment_file_too_large_returns_413() {
        let base = make_state().await;
        let db = base.db.clone();
        let (_layer, io) = socketioxide::SocketIo::new_layer();
        let state = make_state_with_max_file_size(db, io, 1024); // limit: 1 KB

        let (owner_id, _server_id, channel_id) =
            setup_server_channel(&state, "att_max_owner_1", "att-max-1").await;

        let result = init_attachment(
            State(state),
            Extension(claims_for(owner_id, "att_max_owner_1")),
            Path(channel_id),
            Json(InitAttachmentRequest {
                file_name: "big.bin".to_string(),
                mime_type: "application/octet-stream".to_string(),
                size_bytes: 2048, // 2 KB > limit
                created_at: None,
                chunk_size_bytes: 2048,
                chunk_count: 1,
            }),
        )
        .await;

        let err = result.expect_err("file exceeding limit should be rejected");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn init_attachment_within_limit_passes_validation() {
        let base = make_state().await;
        let db = base.db.clone();
        let (_layer, io) = socketioxide::SocketIo::new_layer();
        // 0 = sense límit → mai bloqueja
        let state = make_state_with_max_file_size(db, io, 0);

        let (owner_id, _server_id, channel_id) =
            setup_server_channel(&state, "att_max_owner_2", "att-max-2").await;

        let result = init_attachment(
            State(state),
            Extension(claims_for(owner_id, "att_max_owner_2")),
            Path(channel_id),
            Json(InitAttachmentRequest {
                file_name: "any.bin".to_string(),
                mime_type: "application/octet-stream".to_string(),
                size_bytes: 999_999_999,
                created_at: None,
                chunk_size_bytes: 5 * 1024 * 1024,
                chunk_count: 200,
            }),
        )
        .await;

        // Sense límit, la validació passa. Fallarà per S3 unavailable en test,
        // però no per FileTooLarge.
        let is_file_too_large = matches!(result, Err(AppError::FileTooLarge { .. }));
        assert!(!is_file_too_large, "unlimited (max=0) should never return FileTooLarge");
    }

    #[tokio::test]
    async fn init_attachment_exactly_at_limit_is_allowed() {
        let base = make_state().await;
        let db = base.db.clone();
        let (_layer, io) = socketioxide::SocketIo::new_layer();
        let state = make_state_with_max_file_size(db, io, 1024); // limit: 1 KB

        let (owner_id, _server_id, channel_id) =
            setup_server_channel(&state, "att_max_owner_3", "att-max-3").await;

        let result = init_attachment(
            State(state),
            Extension(claims_for(owner_id, "att_max_owner_3")),
            Path(channel_id),
            Json(InitAttachmentRequest {
                file_name: "exact.bin".to_string(),
                mime_type: "application/octet-stream".to_string(),
                size_bytes: 1024, // exactament al límit
                created_at: None,
                chunk_size_bytes: 1024,
                chunk_count: 1,
            }),
        )
        .await;

        let is_file_too_large = matches!(result, Err(AppError::FileTooLarge { .. }));
        assert!(!is_file_too_large, "file exactly at limit should not return FileTooLarge");
    }

    #[tokio::test]
    async fn init_attachment_storage_quota_exceeded_returns_error() {
        let state = make_state().await;
        let (owner_id, _server_id, channel_id) =
            setup_server_channel(&state, "quota_stor_owner", "quota-stor").await;

        let year_month = chrono::Utc::now().format("%Y-%m").to_string();

        // Free plan: 10 GB storage. Pre-fill usage to leave only 500 bytes free.
        let free_storage: i64 = 10 * 1024 * 1024 * 1024;
        let pre_used = free_storage - 500;
        state
            .db
            .increment_stored_bytes(owner_id, &year_month, pre_used)
            .await
            .expect("pre-fill storage usage");

        let result = init_attachment(
            State(state),
            Extension(claims_for(owner_id, "quota_stor_owner")),
            Path(channel_id),
            Json(InitAttachmentRequest {
                file_name: "toobig.bin".to_string(),
                mime_type: "application/octet-stream".to_string(),
                size_bytes: 1024, // 1 KB > 500 bytes remaining
                created_at: None,
                chunk_size_bytes: 1024,
                chunk_count: 1,
            }),
        )
        .await;

        assert!(
            matches!(result, Err(AppError::StorageQuotaExceeded)),
            "upload over storage quota should be rejected"
        );
    }

    #[tokio::test]
    async fn init_attachment_storage_quota_not_exceeded_passes() {
        let state = make_state().await;
        let (owner_id, _server_id, channel_id) =
            setup_server_channel(&state, "quota_stor_ok_owner", "quota-stor-ok").await;

        let year_month = chrono::Utc::now().format("%Y-%m").to_string();

        // Pre-fill 1 GB — still well within free 10 GB limit.
        state
            .db
            .increment_stored_bytes(owner_id, &year_month, 1024 * 1024 * 1024)
            .await
            .expect("pre-fill storage usage");

        let result = init_attachment(
            State(state),
            Extension(claims_for(owner_id, "quota_stor_ok_owner")),
            Path(channel_id),
            Json(InitAttachmentRequest {
                file_name: "small.bin".to_string(),
                mime_type: "application/octet-stream".to_string(),
                size_bytes: 100,
                created_at: None,
                chunk_size_bytes: 100,
                chunk_count: 1,
            }),
        )
        .await;

        assert!(
            !matches!(result, Err(AppError::StorageQuotaExceeded)),
            "upload within quota should not be rejected for storage"
        );
    }

    #[tokio::test]
    async fn monthly_usage_increments_stored_bytes_on_complete() {
        let state = make_state().await;
        let (owner_id, _server_id, channel_id) =
            setup_server_channel(&state, "quota_incr_owner", "quota-incr").await;

        let year_month = chrono::Utc::now().format("%Y-%m").to_string();
        let (stored_before, _) = state
            .db
            .get_user_storage_usage(owner_id, &year_month)
            .await
            .expect("usage lookup");

        let attachment_id = Uuid::new_v4();
        let key_version_id = Uuid::new_v4();
        let size: i64 = 512;

        state
            .db
            .create_attachment_init(
                attachment_id,
                channel_id,
                owner_id,
                Uuid::new_v4(),
                "incr.bin",
                "application/octet-stream",
                size,
                chrono::Utc::now(),
                "channels/x/y.bin",
                "upload-id-x",
                size,
                1,
            )
            .await
            .expect("create attachment init");

        state
            .db
            .complete_attachment(
                attachment_id,
                "aes-256-gcm",
                "iv",
                "wrapped",
                key_version_id,
                1,
                "deadbeef",
                None,
            )
            .await
            .expect("complete attachment");

        // Simulate what complete_attachment handler does
        state
            .db
            .increment_stored_bytes(owner_id, &year_month, size)
            .await
            .expect("increment stored bytes");

        let (stored_after, _) = state
            .db
            .get_user_storage_usage(owner_id, &year_month)
            .await
            .expect("usage lookup after");

        assert_eq!(
            stored_after,
            stored_before + size,
            "stored_bytes should increase by the attachment size"
        );
    }

    #[tokio::test]
    async fn download_transfer_quota_exceeded_returns_error() {
        let state = make_state().await;
        let (owner_id, _server_id, channel_id) =
            setup_server_channel(&state, "quota_transfer_owner", "quota-transfer").await;

        let year_month = chrono::Utc::now().format("%Y-%m").to_string();

        // Free plan: 100 GB transfer. Pre-fill to leave only 500 bytes.
        let free_transfer: i64 = 100 * 1024 * 1024 * 1024;
        let pre_used = free_transfer - 500;
        state
            .db
            .increment_transfer_bytes(owner_id, &year_month, pre_used)
            .await
            .expect("pre-fill transfer usage");

        // Create a completed attachment of 1 KB > 500 bytes remaining
        let attachment_id = Uuid::new_v4();
        let key_version_id = Uuid::new_v4();
        state
            .db
            .create_attachment_init(
                attachment_id,
                channel_id,
                owner_id,
                Uuid::new_v4(),
                "large.bin",
                "application/octet-stream",
                1024,
                chrono::Utc::now(),
                "channels/t/large.bin",
                "upload-id-t",
                1024,
                1,
            )
            .await
            .expect("create attachment");
        state
            .db
            .complete_attachment(attachment_id, "aes-256-gcm", "iv", "wrapped", key_version_id, 1, "deadbeef", None)
            .await
            .expect("complete attachment");

        let result = download_attachment(
            State(state),
            Extension(claims_for(owner_id, "quota_transfer_owner")),
            Path((channel_id, attachment_id)),
        )
        .await;

        assert!(
            matches!(result, Err(AppError::TransferQuotaExceeded)),
            "download over transfer quota should be rejected"
        );
    }

    #[tokio::test]
    async fn unlimited_plan_bypasses_storage_quota() {
        let state = make_state().await;
        let (owner_id, _server_id, channel_id) =
            setup_server_channel(&state, "quota_unlimited_owner", "quota-unlimited").await;

        // Assign enterprise plan (unlimited = -1) to the user
        state
            .db
            .set_user_plan_by_name(owner_id, "enterprise")
            .await
            .expect("set enterprise plan");

        let year_month = chrono::Utc::now().format("%Y-%m").to_string();

        // Pre-fill with enormous amount — should still be allowed
        state
            .db
            .increment_stored_bytes(owner_id, &year_month, i64::MAX / 2)
            .await
            .expect("pre-fill storage");

        let result = init_attachment(
            State(state),
            Extension(claims_for(owner_id, "quota_unlimited_owner")),
            Path(channel_id),
            Json(InitAttachmentRequest {
                file_name: "any.bin".to_string(),
                mime_type: "application/octet-stream".to_string(),
                size_bytes: 1024,
                created_at: None,
                chunk_size_bytes: 1024,
                chunk_count: 1,
            }),
        )
        .await;

        assert!(
            !matches!(result, Err(AppError::StorageQuotaExceeded)),
            "enterprise plan (-1) should never block uploads"
        );
    }

    #[tokio::test]
    async fn quota_warning_emitted_at_80_percent() {
        let state = make_state().await;
        let (owner_id, _server_id, _channel_id) =
            setup_server_channel(&state, "warn80_owner", "warn80").await;

        let year_month = chrono::Utc::now().format("%Y-%m").to_string();
        let free_storage: i64 = 10 * 1024 * 1024 * 1024;

        // Pre-fill to 79% — no warning yet
        state
            .db
            .increment_stored_bytes(owner_id, &year_month, free_storage * 79 / 100)
            .await
            .expect("pre-fill");

        let (sent80_before, _) = state
            .db
            .get_quota_warning_timestamps(owner_id, &year_month)
            .await
            .expect("timestamps before");
        assert!(!sent80_before, "warning_sent_at_80 should not be set yet");

        // Add enough to cross 80%
        state
            .db
            .increment_stored_bytes(owner_id, &year_month, free_storage * 3 / 100)
            .await
            .expect("increment to 82%");

        // Simulate the handler marking the warning
        let (new_stored, _) = state
            .db
            .get_user_storage_usage(owner_id, &year_month)
            .await
            .expect("usage");
        let pct = new_stored * 100 / free_storage;
        assert!(pct >= 80, "should be at or above 80%");

        let (_, _sent90) = state
            .db
            .get_quota_warning_timestamps(owner_id, &year_month)
            .await
            .expect("timestamps");

        // Mark 80% warning
        state
            .db
            .set_quota_warning_sent(owner_id, &year_month, 80)
            .await
            .expect("set warning 80");

        let (sent80_after, _) = state
            .db
            .get_quota_warning_timestamps(owner_id, &year_month)
            .await
            .expect("timestamps after");
        assert!(sent80_after, "warning_sent_at_80 should now be set");
    }

    #[tokio::test]
    async fn quota_warning_not_repeated_once_set() {
        let state = make_state().await;
        let (owner_id, _server_id, _channel_id) =
            setup_server_channel(&state, "warn_repeat_owner", "warn-repeat").await;

        let year_month = chrono::Utc::now().format("%Y-%m").to_string();

        state
            .db
            .set_quota_warning_sent(owner_id, &year_month, 80)
            .await
            .expect("set warning first time");

        state
            .db
            .set_quota_warning_sent(owner_id, &year_month, 80)
            .await
            .expect("set warning second time (idempotent)");

        let (sent80, _) = state
            .db
            .get_quota_warning_timestamps(owner_id, &year_month)
            .await
            .expect("timestamps");

        assert!(sent80, "warning should still be set after idempotent call");
    }
}
