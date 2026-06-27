//! Assistent de canal: un participant automàtic que entra a un canal de veu
//! LiveKit, captura l'àudio E2EE, el transcriu per segments (endpoint compatible
//! amb OpenAI) i en genera un resum, exportat com a fitxer Markdown a S3.
//!
//! Server-integrat: les sessions actives viuen en un registre global de procés
//! ([`SESSIONS`]); els endpoints a `routes/channel_assistant.rs` les controlen.
//!
//! Seguretat de claus: en canals **asimètrics** el server no custodia la
//! channelKey; el client la passa a l'inici i només viu en memòria mentre dura la
//! connexió (embolicada amb [`Zeroizing`], esborrada en acabar). En canals
//! **simètrics** la clau s'obté de la DB desxifrant-la amb `server_master_key`.

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex as StdMutex};
use std::time::{Duration, Instant};

use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
use base64::{engine::general_purpose::STANDARD, Engine};
use futures_util::StreamExt;
use tokio::sync::{mpsc, oneshot, Mutex as TokioMutex};
use tokio::task::JoinHandle;
use tracing::{info, warn};
use uuid::Uuid;
use zeroize::Zeroizing;

use livekit::e2ee::key_provider::{KeyProvider, KeyProviderOptions};
use livekit::e2ee::{E2eeOptions, EncryptionType as E2eeEncryptionType};
use livekit::prelude::*;
use livekit::webrtc::audio_stream::native::NativeAudioStream;

use crate::config::Config;
use crate::db::DatabasePool;
use crate::error::AppError;
use crate::models::channel::{ChannelType, EncryptionType};
use crate::routes::livekit::mint_livekit_token;

// ── Paràmetres de captura / segmentació ─────────────────────────

/// Freqüència de mostreig objectiu per a STT (Whisper espera 16 kHz mono).
const TARGET_RATE: u32 = 16_000;
/// Energia RMS per sota de la qual una finestra es considera silenci.
const SILENCE_RMS: f64 = 300.0;
/// Silenci continu (ms) que tanca un segment.
const SILENCE_MS: usize = 700;
/// Durada màxima d'un segment abans de forçar-ne el tall (segons).
const MAX_SEGMENT_SECS: usize = 30;
/// Mínim de mostres amb veu perquè un segment valgui la pena transcriure (~0,3 s).
const MIN_VOICED_SAMPLES: usize = (TARGET_RATE as usize) * 3 / 10;

const SUMMARY_SYSTEM_PROMPT: &str = "Ets un assistent que resumeix reunions de veu. \
A partir de la transcripció (amb marques de temps i parlants), redacta un resum clar \
en el mateix idioma de la conversa amb: (1) un paràgraf de visió general, \
(2) punts clau en pics, (3) decisions preses, (4) accions pendents amb responsable si es coneix.";

// ── Registre global de sessions actives ─────────────────────────

struct AssistantHandle {
    stop_tx: oneshot::Sender<()>,
    join: JoinHandle<Option<String>>,
}

static SESSIONS: LazyLock<StdMutex<HashMap<Uuid, AssistantHandle>>> =
    LazyLock::new(|| StdMutex::new(HashMap::new()));

fn is_running(channel_id: Uuid) -> bool {
    SESSIONS.lock().unwrap().contains_key(&channel_id)
}

/// Config mínima necessària per les crides STT/LLM (evita clonar tot `Config`).
#[derive(Clone)]
struct SttConfig {
    base_url: String,
    api_key: Option<String>,
    stt_model: String,
    summary_model: String,
    language: Option<String>,
}

// ── Línia de transcripció ───────────────────────────────────────

struct TranscriptLine {
    offset_secs: u64,
    identity: String,
    text: String,
}

// ── API pública: start / stop ───────────────────────────────────

/// Activa l'assistent en un canal de veu. Resol credencials LiveKit i clau E2EE,
/// es connecta a la sala i engega la captura en una tasca de fons.
///
/// `client_key_b64`: clau de canal en base64, **obligatòria** per canals
/// asimètrics i ignorada per simètrics (s'agafa de la DB).
pub async fn start_session(
    db: DatabasePool,
    config: Config,
    channel_id: Uuid,
    client_key_b64: Option<String>,
) -> Result<(), AppError> {
    if is_running(channel_id) {
        return Err(AppError::AssistantAlreadyRunning);
    }

    let channel = db
        .get_channel(channel_id)
        .await
        .map_err(AppError::DatabaseError)?
        .ok_or(AppError::ChannelNotFound)?;

    if channel.channel_type != ChannelType::Voice {
        return Err(AppError::ChannelNotVoice);
    }

    // Resoldre credencials LiveKit (override per servidor si n'hi ha).
    let mut host = config.livekit_host.clone();
    let mut api_key = config.livekit_api_key.clone();
    let mut api_secret = config.livekit_api_secret.clone();
    if let Some(override_config) = db
        .get_server_livekit_override(channel.server_id)
        .await
        .map_err(AppError::DatabaseError)?
    {
        host = override_config.host;
        api_key = override_config.api_key;
        api_secret = override_config.api_secret;
    }

    // Resoldre la clau E2EE segons el mode del canal.
    let key: Option<Zeroizing<Vec<u8>>> = match channel.encryption_type {
        EncryptionType::None => None,
        EncryptionType::Symmetric => {
            let (_, _, encrypted_key_b64, nonce_b64) = db
                .get_latest_channel_key_version(channel_id)
                .await
                .map_err(AppError::DatabaseError)?
                .ok_or(AppError::ChannelKeyNotFound)?;
            if encrypted_key_b64.is_empty() {
                return Err(AppError::ChannelKeyNotFound);
            }
            let encrypted = STANDARD
                .decode(&encrypted_key_b64)
                .map_err(|_| AppError::DecryptionFailed)?;
            let nonce = STANDARD
                .decode(&nonce_b64)
                .map_err(|_| AppError::DecryptionFailed)?;
            let raw = decrypt_with_master(&config.server_master_key, &encrypted, &nonce)?;
            Some(Zeroizing::new(raw))
        }
        EncryptionType::Asymmetric => {
            let b64 = client_key_b64.ok_or(AppError::AssistantKeyRequired)?;
            Some(decode_channel_key(&b64)?)
        }
    };

    let room_name = format!("chillgroup-{}", channel_id);
    let token = mint_livekit_token(
        &api_key,
        &api_secret,
        &room_name,
        "assistant",
        "Assistent de veu",
        false, // can_publish: l'assistent només escolta
        true,  // can_subscribe
    )?;

    // Opcions de sala + E2EE (RoomOptions és #[non_exhaustive]).
    let mut room_options = RoomOptions::default();
    if let Some(key) = key.as_ref() {
        let key_provider =
            KeyProvider::with_shared_key(KeyProviderOptions::default(), key.to_vec());
        room_options.encryption = Some(E2eeOptions {
            encryption_type: E2eeEncryptionType::Gcm,
            key_provider,
        });
    }

    let ws_url = http_to_ws(&host);
    info!("Assistent connectant-se a {} (room={})", ws_url, room_name);
    let (room, events) = Room::connect(&ws_url, &token, room_options)
        .await
        .map_err(|e| AppError::AssistantError(format!("connexió LiveKit: {e}")))?;
    // A partir d'aquí la clau ja viu dins el KeyProvider; la nostra còpia
    // `key` (Zeroizing) s'esborra en sortir d'aquest abast.

    let stt = SttConfig {
        base_url: config.assistant_openai_base_url.clone(),
        api_key: config.assistant_openai_api_key.clone(),
        stt_model: config.assistant_stt_model.clone(),
        summary_model: config.assistant_summary_model.clone(),
        language: config.assistant_language.clone(),
    };

    let (stop_tx, stop_rx) = oneshot::channel();
    let join = tokio::spawn(run_session(room, events, stop_rx, db, channel_id, stt));

    // Inserció atòmica: si una cursa ha creat la sessió mentrestant, avortem la nostra.
    {
        let mut sessions = SESSIONS.lock().unwrap();
        if sessions.contains_key(&channel_id) {
            drop(sessions);
            join.abort();
            return Err(AppError::AssistantAlreadyRunning);
        }
        sessions.insert(channel_id, AssistantHandle { stop_tx, join });
    }

    Ok(())
}

/// Atura l'assistent del canal, espera l'exportació i retorna l'URL del fitxer.
pub async fn stop_session(channel_id: Uuid) -> Result<Option<String>, AppError> {
    let handle = {
        let mut sessions = SESSIONS.lock().unwrap();
        sessions.remove(&channel_id)
    }
    .ok_or(AppError::AssistantNotRunning)?;

    let _ = handle.stop_tx.send(());
    match handle.join.await {
        Ok(url) => Ok(url),
        Err(e) => Err(AppError::AssistantError(format!("tasca avortada: {e}"))),
    }
}

// ── Bucle principal de la sessió ────────────────────────────────

async fn run_session(
    room: Room,
    mut events: mpsc::UnboundedReceiver<RoomEvent>,
    mut stop_rx: oneshot::Receiver<()>,
    db: DatabasePool,
    channel_id: Uuid,
    stt: SttConfig,
) -> Option<String> {
    let session_start = Instant::now();
    let transcript: std::sync::Arc<TokioMutex<Vec<TranscriptLine>>> =
        std::sync::Arc::new(TokioMutex::new(Vec::new()));
    let http = reqwest::Client::new();
    let stt = std::sync::Arc::new(stt);
    let mut capture_tasks: Vec<JoinHandle<()>> = Vec::new();

    loop {
        tokio::select! {
            _ = &mut stop_rx => {
                info!("Assistent: senyal d'aturada (channel={})", channel_id);
                break;
            }
            maybe_event = events.recv() => {
                match maybe_event {
                    Some(RoomEvent::TrackSubscribed { track, participant, .. }) => {
                        if let RemoteTrack::Audio(audio_track) = track {
                            let identity = participant.identity().to_string();
                            info!("Assistent: subscrit a àudio de {}", identity);
                            let t = transcript.clone();
                            let http = http.clone();
                            let stt = stt.clone();
                            capture_tasks.push(tokio::spawn(capture_track(
                                audio_track, identity, session_start, t, http, stt,
                            )));
                        }
                    }
                    Some(RoomEvent::Disconnected { reason }) => {
                        warn!("Assistent desconnectat de LiveKit: {:?}", reason);
                        break;
                    }
                    Some(_) => {}
                    None => break,
                }
            }
        }
    }

    // Tancar la sala perquè els NativeAudioStream acabin i les tasques de captura
    // finalitzin el seu segment pendent.
    let _ = room.close().await;
    for task in capture_tasks {
        let _ = tokio::time::timeout(Duration::from_secs(20), task).await;
    }

    // Construir transcripció ordenada amb noms d'usuari resolts.
    let lines = {
        let mut guard = transcript.lock().await;
        guard.sort_by_key(|l| l.offset_secs);
        std::mem::take(&mut *guard)
    };
    if lines.is_empty() {
        info!("Assistent: cap transcripció generada (channel={})", channel_id);
        return None;
    }

    let mut name_cache: HashMap<String, String> = HashMap::new();
    let mut body = String::new();
    for line in &lines {
        let name = resolve_name(&db, &mut name_cache, &line.identity).await;
        body.push_str(&format!(
            "[{}] {}: {}\n",
            fmt_offset(line.offset_secs),
            name,
            line.text.trim()
        ));
    }

    let summary = match summarize(&http, &stt, &body).await {
        Ok(s) => s,
        Err(e) => {
            warn!("Assistent: error generant resum: {e}");
            "_(No s'ha pogut generar el resum.)_".to_string()
        }
    };

    let markdown = format!(
        "# Resum de la reunió\n\n{}\n\n---\n\n# Transcripció\n\n{}\n",
        summary, body
    );

    match upload_markdown(channel_id, markdown).await {
        Ok(url) => {
            info!("Assistent: transcripció exportada (channel={})", channel_id);
            Some(url)
        }
        Err(e) => {
            warn!("Assistent: error pujant fitxer: {e}");
            None
        }
    }
}

// ── Captura + segmentació d'una pista d'àudio ───────────────────

async fn capture_track(
    track: RemoteAudioTrack,
    identity: String,
    session_start: Instant,
    transcript: std::sync::Arc<TokioMutex<Vec<TranscriptLine>>>,
    http: reqwest::Client,
    stt: std::sync::Arc<SttConfig>,
) {
    let rtc_track = track.rtc_track();
    let mut stream = NativeAudioStream::new(rtc_track, TARGET_RATE as i32, 1);

    let mut seg: Vec<i16> = Vec::new();
    let mut seg_start_offset: u64 = 0;
    let mut silence_samples: usize = 0;
    let mut voiced_samples: usize = 0;

    while let Some(frame) = stream.next().await {
        let samples: &[i16] = frame.data.as_ref();
        let rms = rms_i16(samples);

        // Saltar el silenci inicial fins que comença la veu.
        if seg.is_empty() {
            if rms < SILENCE_RMS {
                continue;
            }
            seg_start_offset = session_start.elapsed().as_secs();
        }

        seg.extend_from_slice(samples);
        if rms >= SILENCE_RMS {
            silence_samples = 0;
            voiced_samples += samples.len();
        } else {
            silence_samples += samples.len();
        }

        let silence_ms = silence_samples * 1000 / TARGET_RATE as usize;
        let seg_secs = seg.len() / TARGET_RATE as usize;
        let should_close = (silence_ms >= SILENCE_MS && voiced_samples >= MIN_VOICED_SAMPLES)
            || seg_secs >= MAX_SEGMENT_SECS;

        if should_close {
            finalize_segment(&http, &stt, &transcript, &identity, seg_start_offset, &seg).await;
            seg.clear();
            silence_samples = 0;
            voiced_samples = 0;
        }
    }

    // Final de la pista: transcriure el segment restant si té prou veu.
    if voiced_samples >= MIN_VOICED_SAMPLES {
        finalize_segment(&http, &stt, &transcript, &identity, seg_start_offset, &seg).await;
    }
}

async fn finalize_segment(
    http: &reqwest::Client,
    stt: &SttConfig,
    transcript: &std::sync::Arc<TokioMutex<Vec<TranscriptLine>>>,
    identity: &str,
    offset_secs: u64,
    samples: &[i16],
) {
    let wav = pcm16_to_wav(samples, TARGET_RATE);
    match transcribe(http, stt, wav).await {
        Ok(text) if !text.trim().is_empty() => {
            transcript.lock().await.push(TranscriptLine {
                offset_secs,
                identity: identity.to_string(),
                text,
            });
        }
        Ok(_) => {}
        Err(e) => warn!("Assistent: error transcrivint segment: {e}"),
    }
}

// ── Crides a l'endpoint compatible amb OpenAI ───────────────────

async fn transcribe(
    http: &reqwest::Client,
    stt: &SttConfig,
    wav: Vec<u8>,
) -> Result<String, AppError> {
    let part = reqwest::multipart::Part::bytes(wav)
        .file_name("audio.wav")
        .mime_str("audio/wav")
        .map_err(|e| AppError::AssistantError(e.to_string()))?;
    let mut form = reqwest::multipart::Form::new()
        .part("file", part)
        .text("model", stt.stt_model.clone());
    if let Some(lang) = &stt.language {
        form = form.text("language", lang.clone());
    }

    let mut req = http
        .post(format!("{}/audio/transcriptions", stt.base_url))
        .multipart(form);
    if let Some(key) = &stt.api_key {
        req = req.bearer_auth(key);
    }

    let resp = req
        .send()
        .await
        .map_err(|e| AppError::AssistantError(format!("STT: {e}")))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::AssistantError(format!("STT {status}: {body}")));
    }
    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::AssistantError(format!("STT json: {e}")))?;
    Ok(json
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string())
}

async fn summarize(
    http: &reqwest::Client,
    stt: &SttConfig,
    transcript: &str,
) -> Result<String, AppError> {
    let body = serde_json::json!({
        "model": stt.summary_model,
        "temperature": 0.3,
        "messages": [
            { "role": "system", "content": SUMMARY_SYSTEM_PROMPT },
            { "role": "user", "content": transcript },
        ],
    });

    let mut req = http
        .post(format!("{}/chat/completions", stt.base_url))
        .json(&body);
    if let Some(key) = &stt.api_key {
        req = req.bearer_auth(key);
    }

    let resp = req
        .send()
        .await
        .map_err(|e| AppError::AssistantError(format!("resum: {e}")))?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::AssistantError(format!("resum {status}: {text}")));
    }
    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::AssistantError(format!("resum json: {e}")))?;
    Ok(json
        .pointer("/choices/0/message/content")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .trim()
        .to_string())
}

// ── Exportació a S3 ─────────────────────────────────────────────

async fn upload_markdown(channel_id: Uuid, markdown: String) -> Result<String, AppError> {
    use aws_sdk_s3::presigning::PresigningConfig;

    let bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "chillgroup-attachments".to_string());
    let key = format!(
        "meetings/{}/{}.md",
        channel_id,
        chrono::Utc::now().format("%Y%m%dT%H%M%SZ")
    );

    let s3 = build_s3_client(s3_endpoint());
    s3.put_object()
        .bucket(&bucket)
        .key(&key)
        .body(markdown.into_bytes().into())
        .content_type("text/markdown; charset=utf-8")
        .send()
        .await
        .map_err(|e| AppError::AssistantError(format!("S3 put: {e}")))?;

    // URL presignada (7 dies) servida des de l'endpoint públic.
    let presign = build_s3_client(s3_public_endpoint());
    let cfg = PresigningConfig::expires_in(Duration::from_secs(7 * 24 * 3600))
        .map_err(|e| AppError::AssistantError(e.to_string()))?;
    let url = presign
        .get_object()
        .bucket(&bucket)
        .key(&key)
        .presigned(cfg)
        .await
        .map_err(|e| AppError::AssistantError(format!("S3 presign: {e}")))?;
    Ok(url.uri().to_string())
}

fn build_s3_client(endpoint: String) -> aws_sdk_s3::Client {
    use aws_credential_types::{provider::SharedCredentialsProvider, Credentials};
    use aws_sdk_s3::config::{Builder as S3ConfigBuilder, Region};

    let access_key = std::env::var("S3_ACCESS_KEY_ID").unwrap_or_default();
    let secret_key = std::env::var("S3_SECRET_ACCESS_KEY").unwrap_or_default();
    let region = std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".to_string());
    let force_path_style = std::env::var("S3_FORCE_PATH_STYLE")
        .ok()
        .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "true" | "1" | "yes" | "on"))
        .unwrap_or(true);

    let creds = Credentials::new(access_key, secret_key, None, None, "chillgroup-assistant");
    let conf = S3ConfigBuilder::new()
        .region(Region::new(region))
        .credentials_provider(SharedCredentialsProvider::new(creds))
        .endpoint_url(endpoint)
        .force_path_style(force_path_style)
        .build();
    aws_sdk_s3::Client::from_conf(conf)
}

fn s3_endpoint() -> String {
    std::env::var("S3_ENDPOINT").unwrap_or_else(|_| "http://localhost:9000".to_string())
}

fn s3_public_endpoint() -> String {
    std::env::var("S3_PUBLIC_ENDPOINT")
        .or_else(|_| std::env::var("S3_ENDPOINT"))
        .unwrap_or_else(|_| "http://localhost:9000".to_string())
}

// ── Utilitats ───────────────────────────────────────────────────

async fn resolve_name(
    db: &DatabasePool,
    cache: &mut HashMap<String, String>,
    identity: &str,
) -> String {
    if let Some(name) = cache.get(identity) {
        return name.clone();
    }
    let name = match Uuid::parse_str(identity) {
        Ok(uid) => db
            .find_username_by_user_id(uid)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| identity.to_string()),
        Err(_) => identity.to_string(),
    };
    cache.insert(identity.to_string(), name.clone());
    name
}

fn rms_i16(samples: &[i16]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f64 = samples.iter().map(|&s| (s as f64) * (s as f64)).sum();
    (sum / samples.len() as f64).sqrt()
}

fn fmt_offset(secs: u64) -> String {
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

/// Converteix una URL http(s) de LiveKit a ws(s), que és el que espera el SDK.
fn http_to_ws(host: &str) -> String {
    if let Some(rest) = host.strip_prefix("http://") {
        format!("ws://{rest}")
    } else if let Some(rest) = host.strip_prefix("https://") {
        format!("wss://{rest}")
    } else {
        host.to_string()
    }
}

/// Descodifica una clau de canal base64 proporcionada pel client (canals
/// asimètrics). La clau viu embolicada amb [`Zeroizing`] perquè s'esborri de la
/// memòria en deixar-se anar. Ha de tenir exactament 32 bytes.
fn decode_channel_key(b64: &str) -> Result<Zeroizing<Vec<u8>>, AppError> {
    let raw = STANDARD
        .decode(b64.trim())
        .map_err(|_| AppError::AssistantKeyRequired)?;
    if raw.len() != 32 {
        return Err(AppError::AssistantKeyRequired);
    }
    Ok(Zeroizing::new(raw))
}

fn decrypt_with_master(master: &[u8; 32], ciphertext: &[u8], nonce: &[u8]) -> Result<Vec<u8>, AppError> {
    if nonce.len() != 12 {
        return Err(AppError::DecryptionFailed);
    }
    let cipher = Aes256Gcm::new_from_slice(master).map_err(|_| AppError::DecryptionFailed)?;
    cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|_| AppError::DecryptionFailed)
}

fn pcm16_to_wav(samples: &[i16], sample_rate: u32) -> Vec<u8> {
    let data_len = samples.len() * 2;
    let mut buf = Vec::with_capacity(44 + data_len);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&((36 + data_len) as u32).to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes()); // mida del subchunk fmt
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&1u16.to_le_bytes()); // mono
    buf.extend_from_slice(&sample_rate.to_le_bytes());
    buf.extend_from_slice(&(sample_rate * 2).to_le_bytes()); // byte rate
    buf.extend_from_slice(&2u16.to_le_bytes()); // block align
    buf.extend_from_slice(&16u16.to_le_bytes()); // bits per mostra
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&(data_len as u32).to_le_bytes());
    for &s in samples {
        buf.extend_from_slice(&s.to_le_bytes());
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header_is_well_formed() {
        let samples = vec![0i16, 100, -100, 32767, -32768];
        let wav = pcm16_to_wav(&samples, TARGET_RATE);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[36..40], b"data");
        // 44 bytes capçalera + 2 bytes per mostra
        assert_eq!(wav.len(), 44 + samples.len() * 2);
        // sample rate al byte 24
        assert_eq!(&wav[24..28], &TARGET_RATE.to_le_bytes());
    }

    #[test]
    fn rms_distingeix_silenci_de_veu() {
        let silence = vec![0i16; 320];
        let loud = vec![5000i16; 320];
        assert!(rms_i16(&silence) < SILENCE_RMS);
        assert!(rms_i16(&loud) > SILENCE_RMS);
    }

    #[test]
    fn fmt_offset_format() {
        assert_eq!(fmt_offset(0), "00:00");
        assert_eq!(fmt_offset(65), "01:05");
        assert_eq!(fmt_offset(600), "10:00");
    }

    #[test]
    fn http_to_ws_converteix_esquemes() {
        assert_eq!(http_to_ws("http://localhost:7880"), "ws://localhost:7880");
        assert_eq!(http_to_ws("https://x.livekit.cloud"), "wss://x.livekit.cloud");
        assert_eq!(http_to_ws("wss://x.livekit.cloud"), "wss://x.livekit.cloud");
    }

    #[test]
    fn decode_channel_key_valida_mida() {
        let key32 = STANDARD.encode([7u8; 32]);
        assert!(decode_channel_key(&key32).is_ok());

        // Mida incorrecta → error
        let key16 = STANDARD.encode([7u8; 16]);
        assert!(matches!(decode_channel_key(&key16), Err(AppError::AssistantKeyRequired)));

        // Base64 invàlid → error
        assert!(matches!(decode_channel_key("no-base64!!"), Err(AppError::AssistantKeyRequired)));
    }

    #[tokio::test]
    async fn stop_session_sense_assistent_actiu_error() {
        let result = stop_session(Uuid::new_v4()).await;
        assert!(matches!(result, Err(AppError::AssistantNotRunning)));
    }
}
