use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use livekit::e2ee::key_provider::{KeyProvider, KeyProviderOptions, KeyDerivationAlgorithm};
use livekit::e2ee::{E2eeOptions, EncryptionType as LkEncryptionType};
use livekit::prelude::*;
use livekit::options::TrackPublishOptions;
use livekit::webrtc::{
    video_source::{RtcVideoSource, VideoResolution, native::NativeVideoSource},
    video_frame::{I420Buffer, VideoFrame, VideoRotation},
    desktop_capturer::{DesktopCapturer, DesktopCapturerOptions, DesktopCaptureSourceType},
};
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum VoiceCmd {
    ToggleMute,
    ToggleDeafen,
    ToggleCamera,
    ToggleScreenShare,
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
    CameraChanged(bool),
    ScreenShareChanged(bool),
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

// YUYV (YUY2) → I420: Y already at full res, U/V subsampled ×2 horiz only → subsample ×2 vert too
fn yuyv_to_i420(yuyv: &[u8], i420: &mut I420Buffer, width: usize, height: usize) {
    let (stride_y, stride_u, _) = i420.strides();
    let stride_y = stride_y as usize;
    let stride_u = stride_u as usize;
    let (y_plane, u_plane, v_plane) = i420.data_mut();

    for row in 0..height {
        for col in (0..width).step_by(2) {
            let off = row * width * 2 + col * 2;
            let y0 = yuyv[off];
            let u  = yuyv[off + 1];
            let y1 = yuyv[off + 2];
            let v  = yuyv[off + 3];
            y_plane[row * stride_y + col]     = y0;
            y_plane[row * stride_y + col + 1] = y1;
            // Subsample U/V: take from even rows only
            if row % 2 == 0 {
                let cr = row / 2;
                let cc = col / 2;
                u_plane[cr * stride_u + cc] = u;
                v_plane[cr * stride_u + cc] = v;
            }
        }
    }
}

// BT.601 BGRA→I420 (DesktopFrame data on Linux is BGRA)
fn bgra_to_i420(bgra: &[u8], bgra_stride: usize, i420: &mut I420Buffer, width: usize, height: usize) {
    let (stride_y, stride_u, _) = i420.strides();
    let stride_y = stride_y as usize;
    let stride_u = stride_u as usize;
    let (y_plane, u_plane, v_plane) = i420.data_mut();

    for row in 0..height {
        for col in 0..width {
            let off = row * bgra_stride + col * 4;
            let b = bgra[off] as i32;
            let g = bgra[off + 1] as i32;
            let r = bgra[off + 2] as i32;
            let y = ((66 * r + 129 * g + 25 * b + 128) >> 8) + 16;
            y_plane[row * stride_y + col] = y.clamp(0, 255) as u8;
            if row % 2 == 0 && col % 2 == 0 {
                let u = ((-38 * r - 74 * g + 112 * b + 128) >> 8) + 128;
                let v = ((112 * r - 94 * g - 18 * b + 128) >> 8) + 128;
                u_plane[(row / 2) * stride_u + col / 2] = u.clamp(0, 255) as u8;
                v_plane[(row / 2) * stride_u + col / 2] = v.clamp(0, 255) as u8;
            }
        }
    }
}

// BT.601 RGB→I420 (nokhwa camera output)
fn rgb_to_i420(rgb: &[u8], i420: &mut I420Buffer, width: usize, height: usize) {
    let (stride_y, stride_u, _) = i420.strides();
    let stride_y = stride_y as usize;
    let stride_u = stride_u as usize;
    let (y_plane, u_plane, v_plane) = i420.data_mut();

    for row in 0..height {
        for col in 0..width {
            let off = (row * width + col) * 3;
            let r = rgb[off] as i32;
            let g = rgb[off + 1] as i32;
            let b = rgb[off + 2] as i32;
            let y = ((66 * r + 129 * g + 25 * b + 128) >> 8) + 16;
            y_plane[row * stride_y + col] = y.clamp(0, 255) as u8;
            if row % 2 == 0 && col % 2 == 0 {
                let u = ((-38 * r - 74 * g + 112 * b + 128) >> 8) + 128;
                let v = ((112 * r - 94 * g - 18 * b + 128) >> 8) + 128;
                u_plane[(row / 2) * stride_u + col / 2] = u.clamp(0, 255) as u8;
                v_plane[(row / 2) * stride_u + col / 2] = v.clamp(0, 255) as u8;
            }
        }
    }
}

fn start_screen_capture(stop: Arc<AtomicBool>, source: NativeVideoSource) {
    std::thread::spawn(move || {
        let opts = DesktopCapturerOptions::new(DesktopCaptureSourceType::Screen);
        let Some(mut capturer) = DesktopCapturer::new(opts) else {
            tracing::error!("screen share: DesktopCapturer::new failed (Wayland?)");
            return;
        };
        let source_cb = source.clone();
        capturer.start_capture(None, move |result| {
            match result {
                Ok(frame) => {
                    let w = frame.width() as usize;
                    let h = frame.height() as usize;
                    if w == 0 || h == 0 { return; }
                    let bgra = frame.data();
                    let stride = frame.stride() as usize;
                    if bgra.len() < stride * h { return; }
                    let mut i420 = I420Buffer::new(w as u32, h as u32);
                    bgra_to_i420(bgra, stride, &mut i420, w, h);
                    let vf = VideoFrame::new(VideoRotation::VideoRotation0, i420);
                    source_cb.capture_frame(&vf);
                }
                Err(e) => tracing::trace!("screen capture: {:?}", e),
            }
        });
        while !stop.load(Ordering::Relaxed) {
            capturer.capture_frame();
            std::thread::sleep(std::time::Duration::from_millis(66)); // ~15fps
        }
        tracing::info!("screen capture thread stopped");
    });
}

fn start_camera_capture(stop: Arc<AtomicBool>, source: NativeVideoSource) {
    std::thread::spawn(move || {
        use nokhwa::{Camera, utils::{CameraIndex, CameraFormat, FrameFormat, Resolution,
            RequestedFormat, RequestedFormatType}};

        // Request YUYV 640x480 30fps — most webcams support it
        let desired = CameraFormat::new(Resolution::new(640, 480), FrameFormat::YUYV, 30);
        let req = RequestedFormat::new::<nokhwa::pixel_format::RgbFormat>(
            RequestedFormatType::Closest(desired)
        );
        let mut cam = match Camera::new(CameraIndex::Index(0), req) {
            Ok(c) => c,
            Err(e) => { tracing::error!("camera: open failed: {e}"); return; }
        };
        if let Err(e) = cam.open_stream() {
            tracing::error!("camera: open_stream failed: {e}"); return;
        }
        let actual_fmt = cam.camera_format();
        tracing::info!("camera: stream opened format={:?}", actual_fmt);

        while !stop.load(Ordering::Relaxed) {
            match cam.frame() {
                Ok(buf) => {
                    let res = buf.resolution();
                    let w = res.width() as usize;
                    let h = res.height() as usize;
                    if w == 0 || h == 0 { continue; }
                    let raw = buf.buffer();
                    let fmt = buf.source_frame_format();

                    if fmt != FrameFormat::YUYV {
                        tracing::warn!("camera: format {fmt:?} not supported (need YUYV); retrying with explicit format");
                        // Try to switch to YUYV
                        drop(buf);
                        let yuyv_fmt = CameraFormat::new(Resolution::new(w as u32, h as u32), FrameFormat::YUYV, 30);
                        let _ = cam.set_camera_format(yuyv_fmt);
                        continue;
                    }
                    if raw.len() < w * h * 2 { continue; }
                    let mut i420 = I420Buffer::new(w as u32, h as u32);
                    yuyv_to_i420(raw, &mut i420, w, h);
                    let vf = VideoFrame::new(VideoRotation::VideoRotation0, i420);
                    source.capture_frame(&vf);
                }
                Err(e) => {
                    tracing::warn!("camera frame: {e}");
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            }
        }
        let _ = cam.stop_stream();
        tracing::info!("camera thread stopped");
    });
}

async fn publish_video(
    room: &Room,
    name: &str,
    source: TrackSource,
    native_source: NativeVideoSource,
) -> Option<TrackSid> {
    let track = LocalVideoTrack::create_video_track(
        name,
        RtcVideoSource::Native(native_source),
    );
    let opts = TrackPublishOptions {
        source,
        ..Default::default()
    };
    match room.local_participant().publish_track(LocalTrack::Video(track), opts).await {
        Ok(pub_) => Some(pub_.sid()),
        Err(e) => { tracing::error!("{name} publish error: {e}"); None }
    }
}

async fn unpublish_video(room: &Room, sid: TrackSid, stop: Arc<AtomicBool>) {
    stop.store(true, Ordering::Relaxed);
    let _ = room.local_participant().unpublish_track(&sid).await;
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

    // Start muted by default
    mic_pub.mute();
    let mut muted = true;
    let mut deafened = false;
    let mut camera_sid: Option<TrackSid> = None;
    let mut camera_stop: Option<Arc<AtomicBool>> = None;
    let mut screen_sid: Option<TrackSid> = None;
    let mut screen_stop: Option<Arc<AtomicBool>> = None;

    let _ = event_tx.send(VoiceEvent::MuteChanged(true)).await;
    let _ = event_tx.send(VoiceEvent::Connected { session_gen }).await;
    let _ = event_tx.send(VoiceEvent::ParticipantsUpdated(collect_participants(&room))).await;

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
                    VoiceCmd::ToggleCamera => {
                        if let (Some(sid), Some(stop)) = (camera_sid.take(), camera_stop.take()) {
                            unpublish_video(&room, sid, stop).await;
                            let _ = event_tx.send(VoiceEvent::CameraChanged(false)).await;
                        } else {
                            let native = NativeVideoSource::new(
                                VideoResolution { width: 1280, height: 720 }, false,
                            );
                            if let Some(sid) = publish_video(&room, "camera", TrackSource::Camera, native.clone()).await {
                                let stop = Arc::new(AtomicBool::new(false));
                                start_camera_capture(stop.clone(), native);
                                camera_sid = Some(sid);
                                camera_stop = Some(stop);
                                let _ = event_tx.send(VoiceEvent::CameraChanged(true)).await;
                            }
                        }
                    }
                    VoiceCmd::ToggleScreenShare => {
                        if let (Some(sid), Some(stop)) = (screen_sid.take(), screen_stop.take()) {
                            unpublish_video(&room, sid, stop).await;
                            let _ = event_tx.send(VoiceEvent::ScreenShareChanged(false)).await;
                        } else {
                            let native = NativeVideoSource::new(
                                VideoResolution { width: 1920, height: 1080 }, true,
                            );
                            if let Some(sid) = publish_video(&room, "screen", TrackSource::Screenshare, native.clone()).await {
                                let stop = Arc::new(AtomicBool::new(false));
                                start_screen_capture(stop.clone(), native);
                                screen_sid = Some(sid);
                                screen_stop = Some(stop);
                                let _ = event_tx.send(VoiceEvent::ScreenShareChanged(true)).await;
                            }
                        }
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
                        if deafened { set_remote_audio_enabled(&room, false); }
                        let _ = event_tx.send(VoiceEvent::ParticipantsUpdated(collect_participants(&room))).await;
                    }
                    RoomEvent::TrackSubscribed { publication, .. } => {
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
                        for p in &mut parts { p.is_speaking = speaking.contains(&p.user_id); }
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

    // Stop video capture threads
    if let Some(stop) = camera_stop { stop.store(true, Ordering::Relaxed); }
    if let Some(stop) = screen_stop { stop.store(true, Ordering::Relaxed); }

    room.close().await.ok();
    let _ = event_tx.send(VoiceEvent::Disconnected { session_gen }).await;
}
