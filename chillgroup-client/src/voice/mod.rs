use livekit::e2ee::key_provider::{KeyProvider, KeyProviderOptions, KeyDerivationAlgorithm};
use livekit::e2ee::{E2eeOptions, EncryptionType as LkEncryptionType};
use livekit::prelude::*;
use livekit::options::TrackPublishOptions;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum VoiceCmd {
    ToggleMute,
    ToggleDeafen,
    Disconnect,
}

#[derive(Clone, Debug)]
pub struct VoiceParticipant {
    pub user_id: String,
    pub username: String,
    pub initial: String,
    pub is_speaking: bool,
    pub is_suppressed: bool,
}

#[derive(Debug)]
pub enum VoiceEvent {
    Connected { session_gen: u64 },
    Disconnected { session_gen: u64 },
    ParticipantsUpdated(Vec<VoiceParticipant>),
    MuteChanged(bool),
    DeafenChanged(bool),
    Error { session_gen: u64, msg: String },
}

fn collect_participants(room: &Room) -> Vec<VoiceParticipant> {
    let mut parts = Vec::new();

    let lp = room.local_participant();
    let local_id = lp.identity().to_string();
    let local_name = lp.name();
    let local_has_mic = lp
        .track_publications()
        .values()
        .any(|p| p.source() == TrackSource::Microphone && !p.is_muted());
    let display_name = if local_name.is_empty() { "Tu".to_string() } else { local_name };
    let initial = display_name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "?".into());
    parts.push(VoiceParticipant {
        user_id: local_id,
        username: display_name,
        initial,
        is_speaking: lp.is_speaking(),
        is_suppressed: !local_has_mic,
    });

    for p in room.remote_participants().values() {
        let uid = p.identity().to_string();
        let name = p.name();
        let has_mic = p
            .track_publications()
            .values()
            .any(|pub_| pub_.source() == TrackSource::Microphone && !pub_.is_muted());
        let display_name = if name.is_empty() { uid.clone() } else { name };
        let initial = display_name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "?".into());
        parts.push(VoiceParticipant {
            user_id: uid,
            username: display_name,
            initial,
            is_speaking: p.is_speaking(),
            is_suppressed: !has_mic,
        });
    }
    parts
}

fn set_remote_audio_enabled(room: &Room, enabled: bool) {
    for p in room.remote_participants().values() {
        for pub_ in p.track_publications().values() {
            if pub_.kind() == TrackKind::Audio {
                pub_.set_enabled(enabled);
            }
        }
    }
}

pub async fn run(
    livekit_url: String,
    token: String,
    e2ee_key: Option<[u8; 32]>,
    session_gen: u64,
    mut cmd_rx: mpsc::Receiver<VoiceCmd>,
    event_tx: mpsc::Sender<VoiceEvent>,
) {
    let e2ee_enabled = e2ee_key.is_some();
    let mut opts = RoomOptions::default();
    if let Some(key) = e2ee_key {
        let key_provider = KeyProvider::with_shared_key(
            KeyProviderOptions {
                // JS SDK (livekit-client ExternalE2EEKeyProvider) uses HKDF-SHA256
                // with salt="LKFrameEncryptionKey", info=128 zero bytes, output=128 bits (AES-128-GCM).
                // libwebrtc uses the same parameters when algorithm=HKDF.
                // Default PBKDF2 is incompatible — never change this back without testing.
                key_derivation_algorithm: KeyDerivationAlgorithm::HKDF,
                ..KeyProviderOptions::default()
            },
            key.to_vec(),
        );
        opts.encryption = Some(E2eeOptions {
            encryption_type: LkEncryptionType::Gcm,
            key_provider,
        });
        tracing::info!("voice: E2EE enabled (GCM)");
    } else {
        tracing::info!("voice: E2EE disabled (unencrypted channel)");
    }

    tracing::info!("voice: connecting to LiveKit url={}", livekit_url);
    let connect_result = tokio::time::timeout(
        std::time::Duration::from_secs(20),
        Room::connect(&livekit_url, &token, opts),
    ).await;

    let (room, mut room_rx) = match connect_result {
        Ok(Ok(r)) => {
            tracing::info!("voice: Room::connect OK (e2ee={})", e2ee_enabled);
            r
        }
        Ok(Err(e)) => {
            tracing::error!("voice: Room::connect error: {e}");
            let _ = event_tx.send(VoiceEvent::Error { session_gen, msg: e.to_string() }).await;
            return;
        }
        Err(_) => {
            tracing::error!("voice: Room::connect timeout after 20s");
            let _ = event_tx.send(VoiceEvent::Error { session_gen, msg: "Timeout connectant a LiveKit (20s)".into() }).await;
            return;
        }
    };

    tracing::info!("voice: initializing PlatformAudio");
    let platform_audio = match PlatformAudio::new() {
        Ok(a) => { tracing::info!("voice: PlatformAudio OK"); a }
        Err(e) => {
            tracing::error!("voice: PlatformAudio error: {e}");
            let _ = event_tx.send(VoiceEvent::Error { session_gen, msg: format!("Dispositiu d'àudio: {e}") }).await;
            return;
        }
    };

    let audio_track = LocalAudioTrack::create_audio_track("microphone", platform_audio.rtc_source());
    let pub_opts = TrackPublishOptions {
        source: TrackSource::Microphone,
        ..Default::default()
    };

    tracing::info!("voice: publishing microphone track");
    let mic_pub = match room
        .local_participant()
        .publish_track(LocalTrack::Audio(audio_track), pub_opts)
        .await
    {
        Ok(p) => { tracing::info!("voice: mic track published"); p }
        Err(e) => {
            tracing::error!("voice: publish_track error: {e}");
            let _ = event_tx.send(VoiceEvent::Error { session_gen, msg: format!("Publicació micròfon: {e}") }).await;
            return;
        }
    };

    // Start muted by default (same as frontend)
    mic_pub.mute();
    let mut muted = true;
    let mut deafened = false;

    let _ = event_tx.send(VoiceEvent::MuteChanged(true)).await;
    let _ = event_tx.send(VoiceEvent::Connected { session_gen }).await;
    let _ = event_tx
        .send(VoiceEvent::ParticipantsUpdated(collect_participants(&room)))
        .await;

    loop {
        tokio::select! {
            Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    VoiceCmd::ToggleMute => {
                        muted = !muted;
                        if muted { mic_pub.mute(); } else { mic_pub.unmute(); }
                        let _ = event_tx.send(VoiceEvent::MuteChanged(muted)).await;
                        let _ = event_tx.send(VoiceEvent::ParticipantsUpdated(collect_participants(&room))).await;
                    }
                    VoiceCmd::ToggleDeafen => {
                        deafened = !deafened;
                        set_remote_audio_enabled(&room, !deafened);
                        let _ = event_tx.send(VoiceEvent::DeafenChanged(deafened)).await;
                    }
                    VoiceCmd::Disconnect => break,
                }
            }
            Some(event) = room_rx.recv() => {
                match event {
                    RoomEvent::ParticipantConnected(_)
                    | RoomEvent::ParticipantDisconnected(_)
                    | RoomEvent::TrackUnsubscribed { .. }
                    | RoomEvent::TrackMuted { .. }
                    | RoomEvent::TrackUnmuted { .. } => {
                        if deafened {
                            set_remote_audio_enabled(&room, false);
                        }
                        let _ = event_tx.send(VoiceEvent::ParticipantsUpdated(collect_participants(&room))).await;
                    }
                    RoomEvent::TrackSubscribed { publication, .. } => {
                        // Explicitly enable audio playback (default might be false)
                        publication.set_enabled(!deafened);
                        let _ = event_tx.send(VoiceEvent::ParticipantsUpdated(collect_participants(&room))).await;
                    }
                    RoomEvent::ActiveSpeakersChanged { speakers } => {
                        let speaking: std::collections::HashSet<String> = speakers
                            .iter()
                            .map(|p| match p {
                                Participant::Local(lp) => lp.identity().to_string(),
                                Participant::Remote(rp) => rp.identity().to_string(),
                            })
                            .collect();
                        let mut parts = collect_participants(&room);
                        for p in &mut parts {
                            p.is_speaking = speaking.contains(&p.user_id);
                        }
                        let _ = event_tx.send(VoiceEvent::ParticipantsUpdated(parts)).await;
                    }
                    RoomEvent::Disconnected { .. } => {
                        let _ = event_tx.send(VoiceEvent::Disconnected { session_gen }).await;
                        return;
                    }
                    _ => {}
                }
            }
        }
    }

    room.close().await.ok();
    let _ = event_tx.send(VoiceEvent::Disconnected { session_gen }).await;
}
