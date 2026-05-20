//! Tipus compartits entre el servidor i el frontend.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Tipus de canal ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    Text,
    Voice,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EncryptionType {
    None,
    Symmetric,
    Asymmetric,
}

// ── Rols de servidor ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ServerRole {
    Owner,
    Admin,
    Member,
}

// ── Respostes d'autenticació ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub user_id: Uuid,
    pub username: String,
    pub token: String,
    pub device_id: Uuid,
    pub device_label: Option<String>,
    pub is_admin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshResponse {
    pub token: String,
}

// ── Models de base de dades ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub user_id: Uuid,
    pub username: String,
    pub is_admin: bool,
    pub devices: Vec<DeviceInfo>,
    pub quotas: Quotas,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_id: Uuid,
    pub label: Option<String>,
    pub public_key: Option<String>,
    pub last_seen: Option<String>,
    pub revoked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quotas {
    pub max_servers: u32,
    pub max_channels_per_server: u32,
    pub max_messages_per_minute: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelInfo {
    pub channel_id: Uuid,
    pub name: String,
    pub channel_type: ChannelType,
    pub encryption_type: EncryptionType,
    pub message_ttl: Option<i32>,
    pub is_private: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub server_id: Uuid,
    pub name: String,
    pub icon_url: Option<String>,
    pub owner_id: Uuid,
    pub member_count: u32,
    pub my_role: ServerRole,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerFullInfo {
    pub server_id: Uuid,
    pub name: String,
    pub icon_url: Option<String>,
    pub owner_id: Uuid,
    pub my_role: ServerRole,
    pub members: Vec<ServerMember>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMember {
    pub user_id: Uuid,
    pub username: String,
    pub role: ServerRole,
    pub joined_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageInfo {
    pub message_id: Uuid,
    pub channel_id: Uuid,
    pub sender_user_id: Uuid,
    pub sender_username: String,
    pub sender_device_id: Uuid,
    pub encrypted_payload: String,
    pub iv: String,
    pub timestamp: String,
    pub expires_at: Option<String>,
    pub edited_at: Option<String>,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse {
    pub data: Vec<MessageInfo>,
    pub pagination: PaginationMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationMeta {
    pub has_more: bool,
    pub next_cursor: Uuid,
}

// ── LiveKit ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveKitTokenResponse {
    pub token: String,
    pub room: String,
    pub livekit_host: String,
    pub e2ee_enabled: bool,
}

// ── Health Check ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub database: String,
    pub uptime_seconds: u64,
}