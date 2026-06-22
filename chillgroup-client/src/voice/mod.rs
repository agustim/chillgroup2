use std::collections::HashMap;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

use livekit::e2ee::key_provider::{KeyProvider, KeyProviderOptions, KeyDerivationAlgorithm};
use livekit::e2ee::{E2eeOptions, EncryptionType as LkEncryptionType};
use livekit::prelude::*;
use livekit::options::TrackPublishOptions;
use livekit::webrtc::{
    video_source::{RtcVideoSource, VideoResolution, native::NativeVideoSource},
    video_frame::{I420Buffer, VideoFrame, VideoRotation, VideoBuffer, VideoFormatType},
    video_stream::native::NativeVideoStream,
    desktop_capturer::{DesktopCapturer, DesktopCapturerOptions, DesktopCaptureSourceType, CaptureSource},
};
use tokio::sync::mpsc;
use tokio_stream::StreamExt as _;

#[derive(Debug)]
pub enum VoiceCmd {
    ToggleMute,
    ToggleDeafen,
    ToggleCamera,
    ToggleScreenShare,
    StartScreenShare { source_id: u64, is_window: bool },
    Disconnect,
}

#[derive(Clone, Debug)]
pub struct VoiceParticipant {
    pub user_id: String,
    pub username: String,
    pub initial: String,
    pub is_speaking: bool,
    pub is_suppressed: bool,
    pub has_video: bool,
    pub is_screen: bool,
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
    ScreenSources(Vec<(u64, String, bool)>),
    RemoteVideoFrame { participant_id: String, bytes: Vec<u8>, w: u32, h: u32 },
    Error { session_gen: u64, msg: String },
}

fn collect_participants(room: &Room, local_screen_sharing: bool) -> Vec<VoiceParticipant> {
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
        user_id: local_id.clone(),
        username: display_name,
        initial,
        is_speaking: lp.is_speaking(),
        is_suppressed: !local_has_mic,
        has_video: false,
        is_screen: false,
    });
    if local_screen_sharing {
        parts.push(VoiceParticipant {
            user_id: format!("{local_id}-screen"),
            username: "La meva pantalla".to_string(),
            initial: "P".to_string(),
            is_speaking: false,
            is_suppressed: true,
            has_video: false,
            is_screen: true,
        });
    }

    for p in room.remote_participants().values() {
        let uid = p.identity().to_string();
        let name = p.name();
        let pubs = p.track_publications();
        let has_mic = pubs.values().any(|pub_| pub_.source() == TrackSource::Microphone && !pub_.is_muted());
        let has_camera = pubs.values().any(|pub_| pub_.kind() == TrackKind::Video && pub_.source() == TrackSource::Camera && !pub_.is_muted());
        let has_screen = pubs.values().any(|pub_| pub_.kind() == TrackKind::Video && pub_.source() == TrackSource::Screenshare && !pub_.is_muted());
        let display_name = if name.is_empty() { uid.clone() } else { name };
        let initial = display_name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_else(|| "?".into());
        parts.push(VoiceParticipant {
            user_id: uid.clone(),
            username: display_name.clone(),
            initial: initial.clone(),
            is_speaking: p.is_speaking(),
            is_suppressed: !has_mic,
            has_video: has_camera,
            is_screen: false,
        });
        if has_screen {
            parts.push(VoiceParticipant {
                user_id: format!("{uid}-screen"),
                username: format!("{display_name} (pantalla)"),
                initial: "P".to_string(),
                is_speaking: false,
                is_suppressed: true,
                has_video: true,
                is_screen: true,
            });
        }
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

// I420 planes → flat RGB bytes (BT.601 limited-range)
fn i420_to_rgb(y: &[u8], u: &[u8], v: &[u8], stride_y: usize, stride_u: usize, width: u32, height: u32) -> Vec<u8> {
    let w = width as usize;
    let h = height as usize;
    let mut rgb = vec![0u8; w * h * 3];
    for row in 0..h {
        for col in 0..w {
            let yv = y[row * stride_y + col] as i32;
            let uv = u[(row / 2) * stride_u + col / 2] as i32;
            let vv = v[(row / 2) * stride_u + col / 2] as i32;
            let c = yv - 16; let d = uv - 128; let e = vv - 128;
            let base = (row * w + col) * 3;
            rgb[base]     = ((298*c + 409*e + 128) >> 8).clamp(0, 255) as u8;
            rgb[base + 1] = ((298*c - 100*d - 208*e + 128) >> 8).clamp(0, 255) as u8;
            rgb[base + 2] = ((298*c + 516*d + 128) >> 8).clamp(0, 255) as u8;
        }
    }
    rgb
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

// YUYV → flat RGB bytes for local camera preview (BT.601 full-range)
fn yuyv_to_rgb(yuyv: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut rgb = vec![0u8; width * height * 3];
    for i in 0..(width * height / 2) {
        let y0 = yuyv[i * 4] as i32;
        let u  = yuyv[i * 4 + 1] as i32;
        let y1 = yuyv[i * 4 + 2] as i32;
        let v  = yuyv[i * 4 + 3] as i32;
        let conv = |y: i32| -> (u8, u8, u8) {
            let c = y - 16; let d = u - 128; let e = v - 128;
            (
                ((298*c + 409*e + 128) >> 8).clamp(0, 255) as u8,
                ((298*c - 100*d - 208*e + 128) >> 8).clamp(0, 255) as u8,
                ((298*c + 516*d + 128) >> 8).clamp(0, 255) as u8,
            )
        };
        let (r0, g0, b0) = conv(y0);
        let (r1, g1, b1) = conv(y1);
        let base = i * 6;
        rgb[base] = r0; rgb[base+1] = g0; rgb[base+2] = b0;
        rgb[base+3] = r1; rgb[base+4] = g1; rgb[base+5] = b1;
    }
    rgb
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

fn start_screen_capture(
    stop: Arc<AtomicBool>,
    source: NativeVideoSource,
    source_id: Option<u64>,
    is_window: bool,
    preview_tx: mpsc::Sender<VoiceEvent>,
    local_screen_pid: String,
) {
    std::thread::spawn(move || {
        // source_id == None → Wayland PipeWire portal: Generic permet escollir Monitor i Window
        #[cfg(target_os = "linux")]
        let src_type = if source_id.is_none() {
            DesktopCaptureSourceType::Generic
        } else if is_window {
            DesktopCaptureSourceType::Window
        } else {
            DesktopCaptureSourceType::Screen
        };
        #[cfg(not(target_os = "linux"))]
        let src_type = if is_window { DesktopCaptureSourceType::Window } else { DesktopCaptureSourceType::Screen };
        let opts = DesktopCapturerOptions::new(src_type);
        let Some(mut capturer) = DesktopCapturer::new(opts) else {
            tracing::error!("screen share: DesktopCapturer::new failed (Wayland?)");
            return;
        };
        let capture_source: Option<CaptureSource> = if let Some(id) = source_id {
            capturer.get_source_list().into_iter().find(|s| s.id() == id)
        } else {
            None
        };
        let source_cb = source.clone();
        let mut rgb_buf: Vec<u8> = Vec::new();
        let mut frame_count = 0u32;
        capturer.start_capture(capture_source, move |result| {
            match result {
                Ok(frame) => {
                    let w = frame.width() as usize;
                    let h = frame.height() as usize;
                    if w == 0 || h == 0 { return; }
                    let raw = frame.data();
                    let stride = frame.stride() as usize;
                    if raw.len() < stride * h { return; }

                    if frame_count == 0 {
                        tracing::info!(
                            "screen first frame: {}×{} stride={} ({}×4={}) bytes={}",
                            w, h, stride, w, w * 4, raw.len()
                        );
                    }
                    frame_count += 1;

                    // BGRA → RGB24 (WebRTC encode)
                    let need = w * h * 3;
                    if rgb_buf.len() != need { rgb_buf.resize(need, 0); }
                    for row in 0..h {
                        for col in 0..w {
                            let src = row * stride + col * 4;
                            let dst = (row * w + col) * 3;
                            rgb_buf[dst]     = raw[src + 2]; // R
                            rgb_buf[dst + 1] = raw[src + 1]; // G
                            rgb_buf[dst + 2] = raw[src];     // B
                        }
                    }

                    let mut i420 = I420Buffer::new(w as u32, h as u32);
                    rgb_to_i420(&rgb_buf, &mut i420, w, h);
                    let vf = VideoFrame::new(VideoRotation::VideoRotation0, i420);
                    source_cb.capture_frame(&vf);

                    // Preview local: BGRA → RGBA per al tile "La meva pantalla", cada 5 frames
                    if frame_count % 5 == 0 {
                        let mut rgba = vec![0u8; w * h * 4];
                        for row in 0..h {
                            for col in 0..w {
                                let src = row * stride + col * 4;
                                let dst = (row * w + col) * 4;
                                rgba[dst]     = raw[src + 2]; // R
                                rgba[dst + 1] = raw[src + 1]; // G
                                rgba[dst + 2] = raw[src];     // B
                                rgba[dst + 3] = 255;
                            }
                        }
                        let _ = preview_tx.try_send(VoiceEvent::RemoteVideoFrame {
                            participant_id: local_screen_pid.clone(),
                            bytes: rgba,
                            w: w as u32,
                            h: h as u32,
                        });
                    }
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

fn start_camera_capture(
    stop: Arc<AtomicBool>,
    source: NativeVideoSource,
    frame_cb: Option<Arc<dyn Fn(Vec<u8>, u32, u32) + Send + Sync>>,
) {
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
                    if let Some(cb) = &frame_cb {
                        cb(yuyv_to_rgb(raw, w, h), w as u32, h as u32);
                    }
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
    frame_cb: Option<Arc<dyn Fn(Vec<u8>, u32, u32) + Send + Sync>>,
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
    let mut video_tasks: HashMap<TrackSid, tokio::task::JoinHandle<()>> = HashMap::new();

    let _ = event_tx.send(VoiceEvent::MuteChanged(true)).await;
    let _ = event_tx.send(VoiceEvent::Connected { session_gen }).await;
    let _ = event_tx.send(VoiceEvent::ParticipantsUpdated(collect_participants(&room, false))).await;

    loop {
        tokio::select! {
            Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    VoiceCmd::ToggleMute => {
                        muted = !muted;
                        if muted { mic_pub.mute(); } else { mic_pub.unmute(); }
                        let _ = event_tx.send(VoiceEvent::MuteChanged(muted)).await;
                        let _ = event_tx.send(VoiceEvent::ParticipantsUpdated(collect_participants(&room, screen_sid.is_some()))).await;
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
                                start_camera_capture(stop.clone(), native, frame_cb.clone());
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
                            let _ = event_tx.send(VoiceEvent::ParticipantsUpdated(collect_participants(&room, false))).await;
                        } else {
                            let on_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
                            if on_wayland {
                                // PipeWire portal: start directly, sistema mostra selector
                                tracing::info!("screen: Wayland detected, using PipeWire portal (no source picker)");
                                let native = NativeVideoSource::new(VideoResolution { width: 1920, height: 1080 }, true);
                                if let Some(sid) = publish_video(&room, "screen", TrackSource::Screenshare, native.clone()).await {
                                    let stop = Arc::new(AtomicBool::new(false));
                                    let local_screen_pid = format!("{}-screen", room.local_participant().identity());
                                    start_screen_capture(stop.clone(), native, None, false, event_tx.clone(), local_screen_pid);
                                    screen_sid = Some(sid);
                                    screen_stop = Some(stop);
                                    let _ = event_tx.send(VoiceEvent::ScreenShareChanged(true)).await;
                                    let _ = event_tx.send(VoiceEvent::ParticipantsUpdated(collect_participants(&room, true))).await;
                                }
                            } else {
                                // X11: enumera fonts i mostra picker propi
                                let sources = tokio::task::spawn_blocking(|| {
                                    let mut result: Vec<(u64, String, bool)> = Vec::new();
                                    let screen_opts = DesktopCapturerOptions::new(DesktopCaptureSourceType::Screen);
                                    if let Some(sc) = DesktopCapturer::new(screen_opts) {
                                        for src in sc.get_source_list() {
                                            result.push((src.id(), src.title(), false));
                                        }
                                    }
                                    let win_opts = DesktopCapturerOptions::new(DesktopCaptureSourceType::Window);
                                    if let Some(wc) = DesktopCapturer::new(win_opts) {
                                        for src in wc.get_source_list() {
                                            if !src.title().is_empty() {
                                                result.push((src.id(), src.title(), true));
                                            }
                                        }
                                    }
                                    result
                                }).await.unwrap_or_default();
                                let _ = event_tx.send(VoiceEvent::ScreenSources(sources)).await;
                            }
                        }
                    }
                    VoiceCmd::StartScreenShare { source_id, is_window } => {
                        if screen_sid.is_none() {
                            let native = NativeVideoSource::new(VideoResolution { width: 1920, height: 1080 }, true);
                            if let Some(sid) = publish_video(&room, "screen", TrackSource::Screenshare, native.clone()).await {
                                let stop = Arc::new(AtomicBool::new(false));
                                let local_screen_pid = format!("{}-screen", room.local_participant().identity());
                                start_screen_capture(stop.clone(), native, Some(source_id), is_window, event_tx.clone(), local_screen_pid);
                                screen_sid = Some(sid);
                                screen_stop = Some(stop);
                                let _ = event_tx.send(VoiceEvent::ScreenShareChanged(true)).await;
                                let _ = event_tx.send(VoiceEvent::ParticipantsUpdated(collect_participants(&room, true))).await;
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
                    | RoomEvent::TrackMuted { .. }
                    | RoomEvent::TrackUnmuted { .. } => {
                        if deafened { set_remote_audio_enabled(&room, false); }
                        let _ = event_tx.send(VoiceEvent::ParticipantsUpdated(collect_participants(&room, screen_sid.is_some()))).await;
                    }
                    RoomEvent::TrackUnsubscribed { track, publication, .. } => {
                        if matches!(track, RemoteTrack::Video(_)) {
                            if let Some(handle) = video_tasks.remove(&publication.sid()) {
                                handle.abort();
                            }
                        }
                        if deafened { set_remote_audio_enabled(&room, false); }
                        let _ = event_tx.send(VoiceEvent::ParticipantsUpdated(collect_participants(&room, screen_sid.is_some()))).await;
                    }
                    RoomEvent::TrackSubscribed { track, publication, participant } => {
                        publication.set_enabled(!deafened);
                        if let RemoteTrack::Video(vt) = track {
                            let pid = participant.identity().to_string();
                            let is_screen = publication.source() == TrackSource::Screenshare;
                            let effective_pid = if is_screen { format!("{pid}-screen") } else { pid };
                            let rtc = vt.rtc_track();
                            let tx2 = event_tx.clone();
                            let handle = tokio::spawn(async move {
                                let mut stream = NativeVideoStream::new(rtc);
                                while let Some(frame) = stream.next().await {
                                    let w = frame.buffer.width();
                                    let h = frame.buffer.height();
                                    let dst_stride = w * 4;
                                    let mut rgba = vec![0u8; (dst_stride * h) as usize];
                                    frame.buffer.to_argb(
                                        VideoFormatType::ABGR,
                                        &mut rgba,
                                        dst_stride,
                                        w as i32,
                                        h as i32,
                                    );
                                    if tx2.send(VoiceEvent::RemoteVideoFrame {
                                        participant_id: effective_pid.clone(), bytes: rgba, w, h,
                                    }).await.is_err() { break; }
                                }
                            });
                            video_tasks.insert(publication.sid(), handle);
                        }
                        let _ = event_tx.send(VoiceEvent::ParticipantsUpdated(collect_participants(&room, screen_sid.is_some()))).await;
                    }
                    RoomEvent::ActiveSpeakersChanged { speakers } => {
                        let speaking: std::collections::HashSet<String> = speakers
                            .iter()
                            .map(|p| match p {
                                Participant::Local(lp) => lp.identity().to_string(),
                                Participant::Remote(rp) => rp.identity().to_string(),
                            })
                            .collect();
                        let mut parts = collect_participants(&room, screen_sid.is_some());
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

    // Stop video capture threads and remote video tasks
    if let Some(stop) = camera_stop { stop.store(true, Ordering::Relaxed); }
    if let Some(stop) = screen_stop { stop.store(true, Ordering::Relaxed); }
    for (_, handle) in video_tasks { handle.abort(); }

    room.close().await.ok();
    let _ = event_tx.send(VoiceEvent::Disconnected { session_gen }).await;
}
