use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use aws_credential_types::{provider::SharedCredentialsProvider, Credentials};
use aws_sdk_s3::{
    config::{Builder as S3ConfigBuilder, Region},
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

fn build_s3_client() -> S3Client {
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
        .endpoint_url(s3_endpoint())
        .force_path_style(s3_force_path_style())
        .build();

    S3Client::from_conf(conf)
}

fn presigning_config() -> Result<PresigningConfig, AppError> {
    PresigningConfig::expires_in(Duration::from_secs(900)).map_err(|_| AppError::InternalError)
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
pub struct CompleteAttachmentRequest {
    #[serde(alias = "uploadId")]
    pub upload_id: String,
    pub parts: Vec<CompletePartItem>,
    pub crypto: CompleteCrypto,
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

    let s3 = build_s3_client();
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

    let upload_url = presigned.uri().to_string();

    Ok(Json(SignPartResponse {
        part_number: req.part_number,
        upload_url,
    }))
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
        )
        .await
        .map_err(AppError::DatabaseError)?;

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

    if attachment.status != "ready" {
        return Err(AppError::BadRequest);
    }

    let s3 = build_s3_client();
    let cfg = presigning_config()?;
    let presigned = s3
        .get_object()
        .bucket(s3_bucket())
        .key(&attachment.object_key)
        .presigned(cfg)
        .await
        .map_err(|_| AppError::InternalError)?;

    let download_url = presigned.uri().to_string();

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
    }))
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
            "/api/channels/{channel_id}/attachments/{attachment_id}/download",
            get(download_attachment),
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
}
