use std::env;

use chrono::{Duration, Utc};
use chillgroup_server::{
    db::DatabasePool,
    models::{ChannelType, EncryptionType},
};
use shared::types::ServerRole;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use uuid::Uuid;

async fn test_pool() -> Pool<Postgres> {
    init_test_pool().await
}

async fn init_test_pool() -> Pool<Postgres> {
    let base_url = env::var("TEST_DATABASE_URL").unwrap_or_else(|_| {
        "postgres://chillgroup:chillgroup@localhost:5432/chillgroup?sslmode=disable".to_string()
    });
    let admin_url = replace_database_name(&base_url, "postgres");
    let test_database_name = format!("chillgroup_test_{}", Uuid::new_v4().simple());
    let test_url = replace_database_name(&base_url, &test_database_name);

    let admin_pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&admin_url)
        .await
        .expect("connect admin postgres");

    sqlx::query(&format!("DROP DATABASE IF EXISTS {} WITH (FORCE)", test_database_name))
        .execute(&admin_pool)
        .await
        .expect("drop test database");
    sqlx::query(&format!("CREATE DATABASE {}", test_database_name))
        .execute(&admin_pool)
        .await
        .expect("create test database");

    let test_pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&test_url)
        .await
        .expect("connect test postgres");

    sqlx::migrate!("./migrations")
        .run(&test_pool)
        .await
        .expect("run postgres migrations");

    let db = DatabasePool::Postgres(test_pool.clone());
    db.ensure_default_plans().await.expect("ensure default plans");

    test_pool
}

fn replace_database_name(url: &str, database_name: &str) -> String {
    let (without_query, query) = url.split_once('?').unwrap_or((url, ""));
    let (prefix, _) = without_query
        .rsplit_once('/')
        .expect("postgres url must include a database name");

    if query.is_empty() {
        format!("{}/{}", prefix, database_name)
    } else {
        format!("{}/{}?{}", prefix, database_name, query)
    }
}

#[tokio::test]
async fn postgres_server_and_channel_roundtrip() {
    let pool = test_pool().await;
    let db = DatabasePool::Postgres(pool);
    let suffix = Uuid::new_v4().simple().to_string();

    let owner_id = db
        .create_user_with_role(&format!("owner-{suffix}"), "password-hash", "user")
        .await
        .expect("create owner user");

    let server_id = Uuid::new_v4();
    db.create_server_with_owner(server_id, &format!("server-{suffix}"), None, owner_id)
        .await
        .expect("create server");

    let server = db
        .get_server_full_info(server_id, owner_id, false)
        .await
        .expect("load server full info")
        .expect("server exists");

    assert_eq!(server.owner_id, owner_id);
    assert_eq!(server.my_role, ServerRole::Owner);
    assert_eq!(server.members.len(), 1);
    assert!(!server.created_at.is_empty());

    let channel_id = Uuid::new_v4();
    db.create_channel(
        channel_id,
        server_id,
        "general",
        "text",
        "none",
        Some(60),
        false,
    )
    .await
    .expect("create channel");

    let channel = db
        .get_channel(channel_id)
        .await
        .expect("load channel")
        .expect("channel exists");

    assert_eq!(channel.channel_type, ChannelType::Text);
    assert_eq!(channel.encryption_type, EncryptionType::None);
    assert!(!channel.is_private);

    let channels = db
        .list_channels_for_server(server_id, owner_id)
        .await
        .expect("list channels");

    assert_eq!(channels.len(), 1);
    assert_eq!(channels[0].channel_type, ChannelType::Text);
}

#[tokio::test]
async fn postgres_message_read_flow_roundtrip() {
    let pool = test_pool().await;
    let db = DatabasePool::Postgres(pool);
    let suffix = Uuid::new_v4().simple().to_string();

    let owner_id = db
        .create_user_with_role(&format!("owner-msg-{suffix}"), "password-hash", "user")
        .await
        .expect("create owner user");
    let sender_id = db
        .create_user_with_role(&format!("sender-msg-{suffix}"), "password-hash", "user")
        .await
        .expect("create sender user");

    let sender_device_id = db
        .upsert_device_for_user(sender_id, "sender-device", None)
        .await
        .expect("create sender device");

    let server_id = Uuid::new_v4();
    db.create_server_with_owner(server_id, &format!("server-msg-{suffix}"), None, owner_id)
        .await
        .expect("create server");
    db.add_server_member(server_id, sender_id, "member")
        .await
        .expect("add sender to server");

    let channel_id = Uuid::new_v4();
    db.create_channel(
        channel_id,
        server_id,
        "general",
        "text",
        "none",
        Some(15),
        false,
    )
    .await
    .expect("create channel");

    let message_id = Uuid::new_v4();
    let timestamp = Utc::now();
    let expires_at = timestamp + Duration::hours(1);

    db.create_message(
        message_id,
        channel_id,
        sender_id,
        &format!("sender-{suffix}"),
        sender_device_id,
        "encrypted-payload",
        "nonce",
        Some(1),
        Some(expires_at),
        timestamp,
        None,
    )
    .await
    .expect("create message");

    let messages = db
        .list_messages(channel_id, 50, None, None, None)
        .await
        .expect("list messages");

    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].id, message_id);

    let unread_before = db
        .count_unread_messages_for_user(channel_id, owner_id)
        .await
        .expect("count unread before read");
    assert_eq!(unread_before, 1);

    db.mark_channel_read(owner_id, channel_id, Some(message_id))
        .await
        .expect("mark channel read");

    let unread_after = db
        .count_unread_messages_for_user(channel_id, owner_id)
        .await
        .expect("count unread after read");
    assert_eq!(unread_after, 0);
}

#[tokio::test]
async fn postgres_expired_message_cleanup_roundtrip() {
    let pool = test_pool().await;
    let db = DatabasePool::Postgres(pool);
    let suffix = Uuid::new_v4().simple().to_string();

    let owner_id = db
        .create_user_with_role(&format!("owner-exp-{suffix}"), "password-hash", "user")
        .await
        .expect("create owner user");
    let sender_id = db
        .create_user_with_role(&format!("sender-exp-{suffix}"), "password-hash", "user")
        .await
        .expect("create sender user");

    let sender_device_id = db
        .upsert_device_for_user(sender_id, "sender-device", None)
        .await
        .expect("create sender device");

    let server_id = Uuid::new_v4();
    db.create_server_with_owner(server_id, &format!("server-exp-{suffix}"), None, owner_id)
        .await
        .expect("create server");

    let channel_id = Uuid::new_v4();
    db.create_channel(
        channel_id,
        server_id,
        "ttl",
        "text",
        "none",
        Some(1),
        false,
    )
    .await
    .expect("create channel");

    let message_id = Uuid::new_v4();
    let timestamp = Utc::now() - Duration::hours(2);
    let expires_at = Utc::now() - Duration::minutes(1);

    db.create_message(
        message_id,
        channel_id,
        sender_id,
        &format!("sender-exp-{suffix}"),
        sender_device_id,
        "expired-payload",
        "nonce",
        Some(1),
        Some(expires_at),
        timestamp,
        None,
    )
    .await
    .expect("create expired message");

    let (deleted, _) = db.delete_expired_messages().await.expect("delete expired messages");
    assert!(deleted.iter().any(|(id, _)| *id == message_id));
}

#[tokio::test]
async fn postgres_dm_channel_creation_roundtrip() {
    let pool = test_pool().await;
    let db = DatabasePool::Postgres(pool);
    let suffix = Uuid::new_v4().simple().to_string();

    let user_a = db
        .create_user_with_role(&format!("dm-a-{suffix}"), "password-hash", "user")
        .await
        .expect("create first dm user");
    let user_b = db
        .create_user_with_role(&format!("dm-b-{suffix}"), "password-hash", "user")
        .await
        .expect("create second dm user");

    let dm_channel_id = Uuid::new_v4();
    db.create_dm_channel(dm_channel_id, user_a, user_b, Some(120))
        .await
        .expect("create dm channel");

    let found_channel = db
        .find_dm_channel_by_users(user_a, user_b)
        .await
        .expect("find dm channel by users");

    assert_eq!(found_channel, Some(dm_channel_id));

    let channel = db
        .get_channel(dm_channel_id)
        .await
        .expect("load dm channel")
        .expect("dm channel should exist");

    assert_eq!(channel.channel_type, ChannelType::Text);
    assert_eq!(channel.encryption_type, EncryptionType::Asymmetric);
    assert_eq!(channel.message_ttl, Some(120));
    assert!(channel.is_private);
}

// ── TTL cleanup edge-case tests ───────────────────────────────────────────────

async fn setup_channel(db: &DatabasePool, suffix: &str) -> (Uuid, Uuid, Uuid) {
    let user_id = db
        .create_user_with_role(&format!("user-{suffix}"), "password-hash", "user")
        .await
        .expect("create user");
    let device_id = db
        .upsert_device_for_user(user_id, &format!("dev-{suffix}"), None)
        .await
        .expect("create device");
    let server_id = Uuid::new_v4();
    db.create_server_with_owner(server_id, &format!("srv-{suffix}"), None, user_id)
        .await
        .expect("create server");
    let channel_id = Uuid::new_v4();
    db.create_channel(channel_id, server_id, "ch", "text", "none", None, false)
        .await
        .expect("create channel");
    (user_id, device_id, channel_id)
}

#[tokio::test]
async fn postgres_ttl_cleanup_with_thumbnail_attachment_no_fk_violation() {
    let pool = test_pool().await;
    let db = DatabasePool::Postgres(pool.clone());
    let suffix = Uuid::new_v4().simple().to_string();
    let (user_id, device_id, channel_id) = setup_channel(&db, &suffix).await;

    let message_id = Uuid::new_v4();
    db.create_message(
        message_id, channel_id, user_id, "sender", device_id,
        "payload", "iv", None,
        Some(Utc::now() - Duration::minutes(1)),
        Utc::now() - Duration::hours(2),
        None,
    ).await.expect("create expired message");

    let thumb_id = Uuid::new_v4();
    let main_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO attachments \
         (id, channel_id, uploader_user_id, uploader_device_id, file_name, mime_type, \
          size_bytes, created_at, object_key, status, upload_id, chunk_size_bytes, chunk_count) \
         VALUES ($1,$2,$3,$4,'thumb.jpg','image/jpeg',100,NOW(),'thumb-key-tst','linked','u1',1024,1)",
    )
    .bind(thumb_id).bind(channel_id).bind(user_id).bind(device_id)
    .execute(&pool).await.expect("insert thumbnail attachment");

    sqlx::query(
        "INSERT INTO attachments \
         (id, channel_id, uploader_user_id, uploader_device_id, file_name, mime_type, \
          size_bytes, created_at, object_key, status, upload_id, chunk_size_bytes, chunk_count, \
          thumbnail_attachment_id) \
         VALUES ($1,$2,$3,$4,'main.jpg','image/jpeg',1000,NOW(),'main-key-tst','linked','u2',1024,1,$5)",
    )
    .bind(main_id).bind(channel_id).bind(user_id).bind(device_id).bind(thumb_id)
    .execute(&pool).await.expect("insert main attachment with thumbnail ref");

    sqlx::query("INSERT INTO message_attachments (message_id, attachment_id) VALUES ($1,$2)")
        .bind(message_id).bind(main_id)
        .execute(&pool).await.expect("link attachment to message");

    let (deleted, _) = db.delete_expired_messages().await
        .expect("TTL cleanup must not fail with FK violation on thumbnail_attachment_id");

    assert!(deleted.iter().any(|(id, _)| *id == message_id), "expired message deleted");

    let (main_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM attachments WHERE id=$1")
        .bind(main_id).fetch_one(&pool).await.unwrap();
    assert_eq!(main_count, 0, "main attachment deleted");

    let (thumb_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM attachments WHERE id=$1")
        .bind(thumb_id).fetch_one(&pool).await.unwrap();
    assert_eq!(thumb_count, 0, "thumbnail attachment deleted");
}

#[tokio::test]
async fn postgres_ttl_cleanup_cascades_reactions() {
    let pool = test_pool().await;
    let db = DatabasePool::Postgres(pool.clone());
    let suffix = Uuid::new_v4().simple().to_string();
    let (user_id, device_id, channel_id) = setup_channel(&db, &suffix).await;

    let message_id = Uuid::new_v4();
    db.create_message(
        message_id, channel_id, user_id, "sender", device_id,
        "payload", "iv", None,
        Some(Utc::now() - Duration::minutes(1)),
        Utc::now() - Duration::hours(2),
        None,
    ).await.expect("create expired message");

    db.add_reaction(message_id, user_id, &format!("user-{suffix}"), "👍")
        .await.expect("add reaction");

    let reactions = db.get_reactions_for_message(message_id).await.unwrap();
    assert!(!reactions.is_empty(), "reaction must exist before cleanup");

    let (deleted, _) = db.delete_expired_messages().await
        .expect("TTL cleanup must succeed");
    assert!(deleted.iter().any(|(id, _)| *id == message_id));

    let (count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM message_reactions WHERE message_id=$1",
    )
    .bind(message_id).fetch_one(&pool).await.unwrap();
    assert_eq!(count, 0, "reactions deleted via ON DELETE CASCADE");
}

#[tokio::test]
async fn postgres_ttl_cleanup_nulls_reply_to_on_surviving_messages() {
    let pool = test_pool().await;
    let db = DatabasePool::Postgres(pool.clone());
    let suffix = Uuid::new_v4().simple().to_string();
    let (user_id, device_id, channel_id) = setup_channel(&db, &suffix).await;

    let msg_a = Uuid::new_v4();
    db.create_message(
        msg_a, channel_id, user_id, "sender", device_id,
        "payload-a", "iv", None,
        Some(Utc::now() - Duration::minutes(1)),
        Utc::now() - Duration::hours(2),
        None,
    ).await.expect("create expired message A");

    let msg_b = Uuid::new_v4();
    db.create_message(
        msg_b, channel_id, user_id, "sender", device_id,
        "payload-b", "iv", None,
        None,
        Utc::now() - Duration::hours(1),
        Some(msg_a),
    ).await.expect("create non-expired message B replying to A");

    let (deleted, _) = db.delete_expired_messages().await
        .expect("TTL cleanup must succeed");

    assert!(deleted.iter().any(|(id, _)| *id == msg_a), "message A deleted");
    assert!(!deleted.iter().any(|(id, _)| *id == msg_b), "message B NOT deleted");

    let (reply_to,): (Option<Uuid>,) = sqlx::query_as(
        "SELECT reply_to_message_id FROM messages WHERE id=$1",
    )
    .bind(msg_b).fetch_one(&pool).await.unwrap();
    assert_eq!(reply_to, None, "reply_to_message_id nulled via ON DELETE SET NULL");
}