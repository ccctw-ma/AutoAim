#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use autoaim_capture::{
    cursor_position, point_in_screen, scaled_frame_size, CapturedFrame, CapturedFramePreview,
    ScreenCapturer,
};
use autoaim_core::{
    read_jsonl_path, suggest_frames, summarize, validate_records, DetectionObject, MetricsSummary,
    ObjectTracker, Point, TargetScorer, ValidationDiagnostic,
};
use autoaim_infer::{
    InferenceConfig as NativeInferenceConfig, NativeInferenceProvider, NativePersonDetector,
    PersonDetector, PoseEstimate, PoseKeypoint,
};
use autoaim_ipc::{
    AssistSuggestionEvent, InferenceProvider, InferenceResult, InferenceRuntimeConfig,
};
use autoaim_runtime::{JsonlEventWriter, ReviewPipeline};
use serde::Serialize;
#[cfg(target_os = "windows")]
use std::process::Command;
use std::{
    collections::HashMap,
    env,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::Manager;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetWindowLongW, SetWindowLongW, SetWindowPos, GWL_EXSTYLE, HWND_TOPMOST, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_EX_TRANSPARENT,
};

const BUNDLED_MOVENET_ONNX_MODEL: &str = "models/movenet_lightning.onnx";
const BUNDLED_MOVENET_TFLITE_MODEL: &str = "models/movenet_lightning.tflite";
const BUNDLED_YOLOV8_POSE_ONNX_MODEL: &str = "models/yolov8n-pose.onnx";
const BUNDLED_YOLOV8_ONNX_MODEL: &str = "models/yolov8n.onnx";
const LIVE_CAPTURE_MAX_FRAME_SIZE: [u32; 2] = [1920, 1080];
const LIVE_PREVIEW_MAX_FRAME_SIZE: [u32; 2] = [640, 360];
const LIVE_SNAPSHOT_SLOW_MS: u128 = 100;
const LIVE_SNAPSHOT_LOG_EVERY: u64 = 60;
const LIVE_CAPTURE_TIMEOUT_MS: u64 = 1000;
const LIVE_DATASET_MAX_FRAMES: u64 = 120;
const OVERLAY_WINDOW_LABEL: &str = "live-overlay";
const OVERLAY_CURSOR_INTERVAL_MS: u64 = 50;
const OVERLAY_CURSOR_LOG_EVERY_TICKS: u64 = 60;
static LIVE_SNAPSHOT_SEQUENCE: AtomicU64 = AtomicU64::new(0);
#[cfg(target_os = "windows")]
const WDA_EXCLUDEFROMCAPTURE: u32 = 0x11;
#[cfg(target_os = "windows")]
const VK_MENU: i32 = 0x12;

#[cfg(target_os = "windows")]
#[link(name = "user32")]
unsafe extern "system" {
    fn SetWindowDisplayAffinity(hwnd: isize, affinity: u32) -> i32;
    fn GetAsyncKeyState(virtual_key_code: i32) -> i16;
}

#[derive(Debug, Serialize)]
struct AppInfo {
    app_name: &'static str,
    runtime: &'static str,
}

#[derive(Debug, Serialize)]
struct EvaluationResult {
    summary: MetricsSummary,
    diagnostics: Vec<ValidationDiagnostic>,
}

#[derive(Debug, Serialize)]
struct PreviewResult {
    events: Vec<InferenceResult>,
    total_events: usize,
}

#[derive(Debug, Serialize)]
struct PersonPosition {
    frame_id: u64,
    object_index: usize,
    bbox: [f32; 4],
    head_point: Point,
    confidence: f32,
    track_id: Option<u64>,
    cursor: Point,
    dx: f32,
    dy: f32,
    left_mouse_down: bool,
}

#[derive(Debug, Serialize)]
struct PositionPreviewResult {
    positions: Vec<PersonPosition>,
    assist_events: Vec<AssistSuggestionEvent>,
    total_frames: usize,
    total_people: usize,
}

#[derive(Debug, Serialize)]
struct WriteEventsResult {
    output_path: String,
    written_events: usize,
}

#[derive(Debug, Serialize)]
struct UpdateCommandResult {
    success: bool,
    output: String,
    update_available: Option<bool>,
    installed_version: Option<String>,
    latest_version: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct ScreenInfo {
    id: String,
    name: String,
    origin: [i32; 2],
    size: [u32; 2],
    primary: bool,
}

#[derive(Clone, Debug, Serialize)]
struct LivePersonPosition {
    object_index: usize,
    class_name: String,
    bbox: [f32; 4],
    head_point: Point,
    keypoints: Vec<PoseKeypoint>,
    confidence: f32,
    track_id: Option<u64>,
    dx: f32,
    dy: f32,
}

#[derive(Clone, Debug, Serialize)]
struct LiveMonitorSnapshot {
    screen_id: String,
    frame: Option<CapturedFramePreview>,
    cursor: Point,
    cursor_on_screen: bool,
    people: Vec<LivePersonPosition>,
    activation_pressed: bool,
    latency: LiveLatency,
    telemetry: SystemTelemetry,
    model_status: String,
    capture_status: String,
    provider: String,
    review_only: bool,
}

#[derive(Clone, Debug, Default, Serialize)]
struct LiveLatency {
    capture_ms: u128,
    detect_ms: u128,
    tracking_ms: u128,
    total_ms: u128,
}

#[derive(Clone, Debug, Default, Serialize)]
struct SystemTelemetry {
    cpu_usage_percent: Option<f32>,
    gpu_usage_percent: Option<f32>,
    gpu_memory_used_mb: Option<u64>,
    gpu_memory_total_mb: Option<u64>,
    gpu_status: String,
}

#[derive(Clone, Debug, Serialize)]
struct LiveDatasetInfo {
    path: String,
    max_frames: u64,
}

#[derive(Serialize)]
struct LiveDatasetRecord<'a> {
    schema_version: u32,
    sequence: u64,
    timestamp_millis: u128,
    screen_id: &'a str,
    frame_file: &'a str,
    frame_format: &'static str,
    frame_size: [u32; 2],
    screen_origin: [i32; 2],
    screen_size: [u32; 2],
    capture_backend: &'static str,
    cursor: Point,
    cursor_on_screen: bool,
    provider: &'a str,
    model_status: &'a str,
    capture_status: &'a str,
    capture_ms: u128,
    detect_ms: u128,
    tracking_ms: u128,
    total_ms: u128,
    people: &'a [LivePersonPosition],
}

#[derive(Clone, Debug, Serialize)]
struct OverlaySnapshot {
    screen_id: String,
    screen_origin: [i32; 2],
    screen_size: [u32; 2],
    cursor: Point,
    cursor_on_screen: bool,
    people: Vec<LivePersonPosition>,
}

#[derive(Clone, Debug, Serialize)]
struct OverlayCursorSnapshot {
    screen_id: String,
    screen_origin: [i32; 2],
    screen_size: [u32; 2],
    cursor: Point,
    cursor_on_screen: bool,
}

#[derive(Debug, Serialize)]
struct DiagnosticContext {
    app_name: &'static str,
    app_version: &'static str,
    requested_provider: String,
    resolved_model_path: Option<String>,
    confidence_threshold: f32,
    selected_screen_id: Option<String>,
    screens: Vec<ScreenInfo>,
    overlay_enabled: bool,
    overlay_screen_id: Option<String>,
}

#[derive(Default)]
struct LiveTrackingState {
    trackers: Mutex<HashMap<String, ObjectTracker>>,
}

#[derive(Debug)]
struct CachedLiveDetector {
    config: NativeInferenceConfig,
    detector: NativePersonDetector,
}

#[derive(Default)]
struct LiveDetectorState {
    detector: Mutex<Option<CachedLiveDetector>>,
}

#[derive(Default)]
struct LiveCaptureState {
    capturer: Mutex<Option<ScreenCapturer>>,
}

#[derive(Clone)]
struct LiveSnapshotInFlightGuard {
    in_flight: Arc<AtomicBool>,
}

impl Drop for LiveSnapshotInFlightGuard {
    fn drop(&mut self) {
        self.in_flight.store(false, Ordering::Release);
    }
}

#[derive(Default)]
struct LiveSnapshotState {
    in_flight: Arc<AtomicBool>,
}

#[derive(Default)]
struct LiveDatasetState {
    recorder: Mutex<Option<LiveDatasetRecorder>>,
}

#[derive(Default)]
struct SystemTelemetryState {
    cache: Mutex<SystemTelemetryCache>,
}

#[derive(Default)]
struct SystemTelemetryCache {
    cpu_sample: Option<CpuTimesSample>,
    gpu_sample: Option<GpuTelemetrySample>,
}

#[derive(Clone, Copy, Debug)]
struct CpuTimesSample {
    idle: u64,
    total: u64,
}

#[derive(Clone, Debug)]
struct GpuTelemetrySample {
    sampled_at: Instant,
    usage_percent: Option<f32>,
    memory_used_mb: Option<u64>,
    memory_total_mb: Option<u64>,
    status: String,
}

struct LiveDatasetRecorder {
    root: PathBuf,
    frames_dir: PathBuf,
    records_path: PathBuf,
    frames_written: u64,
    max_frames: u64,
}

struct OverlayState {
    active_screen_id: Mutex<Option<String>>,
    refresh_generation: AtomicU64,
}

impl Default for OverlayState {
    fn default() -> Self {
        Self {
            active_screen_id: Mutex::new(None),
            refresh_generation: AtomicU64::new(0),
        }
    }
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        app_name: "AutoAim Review",
        runtime: "Rust + Tauri",
    }
}

#[tauri::command]
fn inference_runtime_config(
    provider: Option<String>,
    model_path: Option<String>,
    device_id: Option<u32>,
    confidence_threshold: Option<f32>,
) -> Result<InferenceRuntimeConfig, String> {
    let provider = match provider
        .unwrap_or_else(|| "directml".to_string())
        .to_ascii_lowercase()
        .as_str()
    {
        "cuda" => InferenceProvider::Cuda,
        "tensorrt" | "tensor_rt" | "tensor-rt" => InferenceProvider::TensorRt,
        "directml" | "direct_ml" | "direct-ml" => InferenceProvider::DirectMl,
        "cpu" => InferenceProvider::Cpu,
        value => return Err(format!("unsupported inference provider: {value}")),
    };

    Ok(InferenceRuntimeConfig::new(
        provider,
        resolve_model_path(model_path, provider_prefers_onnx(provider)),
        device_id,
        confidence_threshold.unwrap_or(0.25),
    ))
}

#[tauri::command]
fn list_screens(window: tauri::Window) -> Result<Vec<ScreenInfo>, String> {
    platform_list_screens(&window)
}

#[tauri::command]
fn set_compact_window(window: tauri::Window, compact: bool) -> Result<(), String> {
    if compact {
        let compact_size = tauri::PhysicalSize::new(460_u32, 320_u32);
        window
            .set_always_on_top(true)
            .map_err(|error| format!("failed to set compact window topmost: {error}"))?;
        window
            .set_resizable(false)
            .map_err(|error| format!("failed to set compact window resizable: {error}"))?;
        window
            .set_size(compact_size)
            .map_err(|error| format!("failed to set compact window size: {error}"))?;
        if let Ok(Some(monitor)) = window.current_monitor() {
            let position = monitor.position();
            let size = monitor.size();
            let x = position.x + size.width.saturating_sub(compact_size.width + 24) as i32;
            let y = position.y + 24;
            let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
        }
    } else {
        window
            .set_always_on_top(false)
            .map_err(|error| format!("failed to clear compact window topmost: {error}"))?;
        window
            .set_resizable(true)
            .map_err(|error| format!("failed to restore window resizable: {error}"))?;
        window
            .set_size(tauri::PhysicalSize::new(1480_u32, 900_u32))
            .map_err(|error| format!("failed to restore window size: {error}"))?;
    }
    Ok(())
}

#[tauri::command]
async fn live_monitor_snapshot(
    app: tauri::AppHandle,
    window: tauri::Window,
    screen_id: String,
    model_path: Option<String>,
    provider: Option<String>,
    confidence_threshold: Option<f32>,
    include_frame: Option<bool>,
) -> Result<LiveMonitorSnapshot, String> {
    let in_flight = app.state::<LiveSnapshotState>().in_flight.clone();
    if in_flight.swap(true, Ordering::AcqRel) {
        return Err("live snapshot busy".to_string());
    }
    let _in_flight_guard = LiveSnapshotInFlightGuard { in_flight };

    let screens = platform_list_screens(&window)?;
    let screen = screens
        .iter()
        .find(|item| item.id == screen_id)
        .or_else(|| screens.iter().find(|item| item.primary))
        .or_else(|| screens.first())
        .cloned()
        .ok_or_else(|| "no screens found".to_string())?;
    let config = native_inference_config(provider, model_path, confidence_threshold)?;

    tauri::async_runtime::spawn_blocking(move || {
        let tracking_state = app.state::<LiveTrackingState>();
        let detector_state = app.state::<LiveDetectorState>();
        let capture_state = app.state::<LiveCaptureState>();
        let dataset_state = app.state::<LiveDatasetState>();
        let overlay_state = app.state::<OverlayState>();
        let telemetry_state = app.state::<SystemTelemetryState>();
        build_live_monitor_snapshot(
            &app,
            &tracking_state,
            &detector_state,
            &capture_state,
            &dataset_state,
            &overlay_state,
            &telemetry_state,
            &screen,
            config,
            include_frame.unwrap_or(true),
        )
    })
    .await
    .map_err(|error| format!("live monitor worker failed: {error}"))?
}

fn build_live_monitor_snapshot(
    app: &tauri::AppHandle,
    tracking_state: &LiveTrackingState,
    detector_state: &LiveDetectorState,
    capture_state: &LiveCaptureState,
    dataset_state: &LiveDatasetState,
    overlay_state: &OverlayState,
    telemetry_state: &SystemTelemetryState,
    screen: &ScreenInfo,
    config: NativeInferenceConfig,
    include_frame: bool,
) -> Result<LiveMonitorSnapshot, String> {
    let sequence = LIVE_SNAPSHOT_SEQUENCE
        .fetch_add(1, Ordering::Relaxed)
        .wrapping_add(1);
    if !live_activation_pressed() {
        return build_live_idle_snapshot(app, overlay_state, telemetry_state, screen, sequence);
    }

    let total_started = Instant::now();
    let capture_started = Instant::now();
    let frame = match capture_with_cached_capturer(capture_state, screen) {
        Ok(frame) => frame,
        Err(error) => {
            append_app_log(
                "backend",
                "live snapshot capture failed",
                Some(&format!(
                    r#"{{"sequence":{sequence},"screen_id":"{}","error":"{}"}}"#,
                    screen.id,
                    json_escape(&error)
                )),
            );
            return Err(error);
        }
    };
    let capture_ms = capture_started.elapsed().as_millis();

    let detect_started = Instant::now();
    let inference = match detect_with_cached_live_detector(&detector_state, config, &frame) {
        Ok(inference) => inference,
        Err(error) => {
            append_app_log(
                "backend",
                "live snapshot inference failed",
                Some(&format!(
                    r#"{{"sequence":{sequence},"screen_id":"{}","capture_ms":{},"error":"{}"}}"#,
                    screen.id,
                    capture_ms,
                    json_escape(&error)
                )),
            );
            return Err(error);
        }
    };
    let detect_ms = detect_started.elapsed().as_millis();

    let tracking_started = Instant::now();
    let mut tracked_objects = inference.objects.clone();
    apply_live_tracking(&tracking_state, &screen.id, &mut tracked_objects)
        .map_err(|error| error.to_string())?;
    let people = live_positions_from_objects(&tracked_objects, &inference.poses, frame.cursor);
    let tracking_ms = tracking_started.elapsed().as_millis();
    let capture_status = format!(
        "native Windows capture [{}]: {}x{} preview from {}x{} screen",
        frame.capture_backend.as_str(),
        frame.frame_size[0],
        frame.frame_size[1],
        frame.screen_size[0],
        frame.screen_size[1]
    );
    emit_overlay_snapshot(
        &app,
        &overlay_state,
        OverlaySnapshot {
            screen_id: screen.id.clone(),
            screen_origin: frame.screen_origin,
            screen_size: frame.screen_size,
            cursor: frame.cursor,
            cursor_on_screen: frame.cursor_on_screen,
            people: people.clone(),
        },
    );
    let frame_preview = if include_frame {
        Some(live_frame_preview(&frame))
    } else {
        None
    };
    let total_ms = total_started.elapsed().as_millis();
    let latency = LiveLatency {
        capture_ms,
        detect_ms,
        tracking_ms,
        total_ms,
    };
    let telemetry = sample_system_telemetry(telemetry_state);
    if should_log_live_snapshot(sequence, total_ms) {
        let keypoints = people
            .iter()
            .map(|person| person.keypoints.len())
            .sum::<usize>();
        append_app_log(
            "backend",
            "live snapshot",
            Some(&format!(
                r#"{{"sequence":{sequence},"screen_id":"{}","capture_ms":{},"detect_ms":{},"tracking_ms":{},"total_ms":{},"include_frame":{},"capture_backend":"{}","frame_size":[{},{}],"screen_size":[{},{}],"objects":{},"poses":{},"people":{},"keypoints":{},"provider":"{}","model_status":"{}"}}"#,
                screen.id,
                capture_ms,
                detect_ms,
                tracking_ms,
                total_ms,
                include_frame,
                frame.capture_backend.as_str(),
                frame.frame_size[0],
                frame.frame_size[1],
                frame.screen_size[0],
                frame.screen_size[1],
                inference.objects.len(),
                inference.poses.len(),
                people.len(),
                keypoints,
                inference.provider.as_str(),
                json_escape(&inference.model_status)
            )),
        );
    }
    record_live_dataset_frame(
        dataset_state,
        sequence,
        screen,
        &frame,
        &inference,
        &capture_status,
        capture_ms,
        detect_ms,
        tracking_ms,
        total_ms,
        &people,
    );

    Ok(LiveMonitorSnapshot {
        screen_id: screen.id.clone(),
        frame: frame_preview,
        cursor: frame.cursor,
        cursor_on_screen: frame.cursor_on_screen,
        people,
        activation_pressed: true,
        latency,
        telemetry,
        model_status: inference.model_status,
        capture_status,
        provider: inference.provider.as_str().to_string(),
        review_only: true,
    })
}

fn build_live_idle_snapshot(
    app: &tauri::AppHandle,
    overlay_state: &OverlayState,
    telemetry_state: &SystemTelemetryState,
    screen: &ScreenInfo,
    sequence: u64,
) -> Result<LiveMonitorSnapshot, String> {
    let cursor = cursor_position().unwrap_or([
        screen.origin[0] as f32 + screen.size[0] as f32 / 2.0,
        screen.origin[1] as f32 + screen.size[1] as f32 / 2.0,
    ]);
    let cursor_on_screen = point_in_screen(cursor, screen.origin, screen.size);
    let people = Vec::new();
    emit_overlay_snapshot(
        app,
        overlay_state,
        OverlaySnapshot {
            screen_id: screen.id.clone(),
            screen_origin: screen.origin,
            screen_size: screen.size,
            cursor,
            cursor_on_screen,
            people: people.clone(),
        },
    );
    let telemetry = sample_system_telemetry(telemetry_state);
    if sequence <= 5 || sequence % LIVE_SNAPSHOT_LOG_EVERY == 0 {
        append_app_log(
            "backend",
            "live snapshot idle",
            Some(&format!(
                r#"{{"sequence":{sequence},"screen_id":"{}","reason":"hold_alt"}}"#,
                screen.id
            )),
        );
    }

    Ok(LiveMonitorSnapshot {
        screen_id: screen.id.clone(),
        frame: None,
        cursor,
        cursor_on_screen,
        people,
        activation_pressed: false,
        latency: LiveLatency::default(),
        telemetry,
        model_status: "Hold Alt to scan".to_string(),
        capture_status: "idle: hold Alt to capture and infer".to_string(),
        provider: "idle".to_string(),
        review_only: true,
    })
}

#[cfg(target_os = "windows")]
fn live_activation_pressed() -> bool {
    unsafe { GetAsyncKeyState(VK_MENU) < 0 }
}

#[cfg(not(target_os = "windows"))]
fn live_activation_pressed() -> bool {
    true
}

fn should_log_live_snapshot(sequence: u64, total_ms: u128) -> bool {
    sequence <= 5 || sequence % LIVE_SNAPSHOT_LOG_EVERY == 0 || total_ms >= LIVE_SNAPSHOT_SLOW_MS
}

fn capture_with_cached_capturer(
    capture_state: &LiveCaptureState,
    screen: &ScreenInfo,
) -> Result<CapturedFrame, String> {
    let mut cached = capture_state
        .capturer
        .lock()
        .map_err(|_| "live capture state lock poisoned".to_string())?;

    let needs_new = cached
        .as_ref()
        .map(|capturer| !capturer.is_alive() || !capturer.matches(screen.origin, screen.size))
        .unwrap_or(true);
    if needs_new {
        let capturer = ScreenCapturer::new(screen.origin, screen.size, LIVE_CAPTURE_MAX_FRAME_SIZE)
            .map_err(|error| error.to_string())?;
        *cached = Some(capturer);
    }

    let capturer = cached
        .as_mut()
        .ok_or_else(|| "live capturer cache is empty".to_string())?;
    let result = capturer.capture(Duration::from_millis(LIVE_CAPTURE_TIMEOUT_MS));

    // Drop a stalled or dead capturer so the next call rebuilds a fresh session.
    if matches!(
        result,
        Err(autoaim_capture::CaptureError::CaptureTimedOut)
            | Err(autoaim_capture::CaptureError::BackendUnavailable(_))
    ) {
        *cached = None;
    }

    result.map_err(|error| error.to_string())
}

fn live_frame_preview(frame: &CapturedFrame) -> CapturedFramePreview {
    let preview_size = scaled_frame_size(frame.frame_size, LIVE_PREVIEW_MAX_FRAME_SIZE)
        .unwrap_or(LIVE_PREVIEW_MAX_FRAME_SIZE);
    if preview_size == frame.frame_size {
        return CapturedFramePreview::from(frame);
    }

    let rgba = resize_rgba_nearest(&frame.rgba, frame.frame_size, preview_size);
    let preview_frame = CapturedFrame {
        screen_origin: frame.screen_origin,
        screen_size: frame.screen_size,
        frame_size: preview_size,
        capture_backend: frame.capture_backend,
        rgba,
        cursor: frame.cursor,
        cursor_on_screen: frame.cursor_on_screen,
        timestamp_millis: frame.timestamp_millis,
    };
    CapturedFramePreview::from(&preview_frame)
}

fn resize_rgba_nearest(rgba: &[u8], source_size: [u32; 2], target_size: [u32; 2]) -> Vec<u8> {
    let [source_width, source_height] = source_size;
    let [target_width, target_height] = target_size;
    if source_width == 0
        || source_height == 0
        || target_width == 0
        || target_height == 0
        || rgba.len() != source_width as usize * source_height as usize * 4
    {
        return Vec::new();
    }

    let source_width = source_width as usize;
    let source_height = source_height as usize;
    let target_width = target_width as usize;
    let target_height = target_height as usize;
    let scale_x = source_width as f32 / target_width as f32;
    let scale_y = source_height as f32 / target_height as f32;
    let mut resized = vec![0_u8; target_width * target_height * 4];

    for target_y in 0..target_height {
        let source_y = ((target_y as f32 + 0.5) * scale_y)
            .floor()
            .clamp(0.0, source_height.saturating_sub(1) as f32) as usize;
        for target_x in 0..target_width {
            let source_x = ((target_x as f32 + 0.5) * scale_x)
                .floor()
                .clamp(0.0, source_width.saturating_sub(1) as f32)
                as usize;
            let source_index = (source_y * source_width + source_x) * 4;
            let target_index = (target_y * target_width + target_x) * 4;
            resized[target_index..target_index + 4]
                .copy_from_slice(&rgba[source_index..source_index + 4]);
        }
    }

    resized
}

fn detect_with_cached_live_detector(
    detector_state: &LiveDetectorState,
    config: NativeInferenceConfig,
    frame: &CapturedFrame,
) -> Result<autoaim_infer::InferenceOutput, String> {
    let mut cached = detector_state
        .detector
        .lock()
        .map_err(|_| "live detector state lock poisoned".to_string())?;
    let should_reload = cached
        .as_ref()
        .map(|entry| entry.config != config)
        .unwrap_or(true);

    if should_reload {
        let detector =
            NativePersonDetector::from_config(config.clone()).map_err(|error| error.to_string())?;
        *cached = Some(CachedLiveDetector { config, detector });
    }

    let detector = cached
        .as_ref()
        .ok_or_else(|| "live detector cache is empty".to_string())?;
    match detector.detector.detect(frame) {
        Ok(output) => Ok(output),
        Err(error) => {
            // Drop the cached detector so a GPU/DirectML session that entered an
            // error state (device removed/hang) is rebuilt on the next frame
            // instead of failing forever.
            *cached = None;
            Err(error.to_string())
        }
    }
}

fn native_inference_config(
    provider: Option<String>,
    model_path: Option<String>,
    confidence_threshold: Option<f32>,
) -> Result<NativeInferenceConfig, String> {
    let provider_name = provider.unwrap_or_else(|| "directml".to_string());
    let provider = NativeInferenceProvider::from_name(&provider_name)
        .ok_or_else(|| format!("unsupported inference provider: {provider_name}"))?;

    Ok(NativeInferenceConfig::new(
        provider,
        resolve_model_path(model_path, native_provider_prefers_onnx(provider)),
        confidence_threshold.unwrap_or(0.25),
    ))
}

fn provider_prefers_onnx(provider: InferenceProvider) -> bool {
    matches!(
        provider,
        InferenceProvider::DirectMl | InferenceProvider::Cuda | InferenceProvider::TensorRt
    )
}

fn native_provider_prefers_onnx(provider: NativeInferenceProvider) -> bool {
    matches!(
        provider,
        NativeInferenceProvider::DirectMl
            | NativeInferenceProvider::Cuda
            | NativeInferenceProvider::TensorRt
    )
}

fn resolve_model_path(model_path: Option<String>, prefer_onnx: bool) -> Option<String> {
    let explicit = model_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    if let Some(value) = explicit {
        return resolve_existing_model_path(value)
            .or_else(|| Some(PathBuf::from(value)))
            .map(path_to_string);
    }

    let candidates: &[&str] = if prefer_onnx {
        &[
            BUNDLED_YOLOV8_POSE_ONNX_MODEL,
            BUNDLED_YOLOV8_ONNX_MODEL,
            BUNDLED_MOVENET_ONNX_MODEL,
            BUNDLED_MOVENET_TFLITE_MODEL,
        ]
    } else {
        &[BUNDLED_MOVENET_TFLITE_MODEL, BUNDLED_MOVENET_ONNX_MODEL]
    };

    candidates
        .iter()
        .find_map(|candidate| resolve_existing_model_path(candidate))
        .map(path_to_string)
}

fn resolve_existing_model_path(value: &str) -> Option<PathBuf> {
    let path = PathBuf::from(value);
    if path.is_file() {
        return Some(path);
    }

    let mut candidates = Vec::new();
    if path.is_relative() {
        if let Ok(exe_path) = env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                candidates.push(exe_dir.join(&path));
            }
        }
        if let Ok(current_dir) = env::current_dir() {
            candidates.push(current_dir.join(&path));
        }
    }

    candidates.into_iter().find(|candidate| candidate.is_file())
}

fn path_to_string(path: PathBuf) -> String {
    path.to_string_lossy().to_string()
}

fn live_positions_from_objects(
    objects: &[DetectionObject],
    poses: &[PoseEstimate],
    cursor: Point,
) -> Vec<LivePersonPosition> {
    objects
        .iter()
        .enumerate()
        .map(|(object_index, object)| {
            let head_point = object.aim_point();
            let keypoints = poses
                .get(object_index)
                .map(|pose| pose.keypoints.clone())
                .filter(|keypoints| !keypoints.is_empty())
                .unwrap_or_default();
            LivePersonPosition {
                object_index,
                class_name: object.class_name.clone(),
                bbox: object.bbox,
                head_point,
                keypoints,
                confidence: object.confidence,
                track_id: object.track_id,
                dx: head_point[0] - cursor[0],
                dy: head_point[1] - cursor[1],
            }
        })
        .collect()
}

fn apply_live_tracking(
    tracking_state: &LiveTrackingState,
    screen_id: &str,
    objects: &mut [DetectionObject],
) -> Result<(), String> {
    let mut trackers = tracking_state
        .trackers
        .lock()
        .map_err(|_| "live tracker state lock poisoned".to_string())?;
    let tracker = trackers
        .entry(screen_id.to_string())
        .or_insert_with(ObjectTracker::default);
    tracker.assign(objects);
    Ok(())
}

fn emit_overlay_snapshot(
    app: &tauri::AppHandle,
    overlay_state: &OverlayState,
    snapshot: OverlaySnapshot,
) {
    let active_screen_id = overlay_state
        .active_screen_id
        .lock()
        .ok()
        .and_then(|guard| guard.clone());
    if active_screen_id.as_deref() != Some(snapshot.screen_id.as_str()) {
        return;
    }

    if let Some(window) = app.get_window(OVERLAY_WINDOW_LABEL) {
        let _ = window.emit("overlay_snapshot", &snapshot);
    }
}

fn log_file_path() -> PathBuf {
    let base = env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("AutoAimReview")
        .join("logs")
        .join("autoaim-review.log")
}

fn append_app_log(scope: &str, message: &str, payload: Option<&str>) {
    let path = log_file_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let line = match payload {
        Some(value) if !value.trim().is_empty() => {
            format!("[{millis}] [{scope}] {message} | {value}\n")
        }
        _ => format!("[{millis}] [{scope}] {message}\n"),
    };

    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        use std::io::Write;
        let _ = file.write_all(line.as_bytes());
    }
}

fn unix_timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn sample_system_telemetry(state: &SystemTelemetryState) -> SystemTelemetry {
    let mut cache = match state.cache.lock() {
        Ok(cache) => cache,
        Err(_) => {
            return SystemTelemetry {
                gpu_status: "telemetry lock poisoned".to_string(),
                ..SystemTelemetry::default()
            };
        }
    };

    let cpu_usage_percent = sample_cpu_usage_percent(&mut cache.cpu_sample);
    let gpu_sample = sample_gpu_telemetry(&mut cache.gpu_sample);
    SystemTelemetry {
        cpu_usage_percent,
        gpu_usage_percent: gpu_sample.usage_percent,
        gpu_memory_used_mb: gpu_sample.memory_used_mb,
        gpu_memory_total_mb: gpu_sample.memory_total_mb,
        gpu_status: gpu_sample.status,
    }
}

fn sample_cpu_usage_percent(previous: &mut Option<CpuTimesSample>) -> Option<f32> {
    let current = sample_cpu_times()?;
    let usage = previous.and_then(|last| {
        let idle_delta = current.idle.saturating_sub(last.idle);
        let total_delta = current.total.saturating_sub(last.total);
        if total_delta == 0 {
            None
        } else {
            Some(((total_delta - idle_delta) as f32 * 100.0 / total_delta as f32).clamp(0.0, 100.0))
        }
    });
    *previous = Some(current);
    usage
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct KernelFileTime {
    low_date_time: u32,
    high_date_time: u32,
}

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetSystemTimes(
        idle_time: *mut KernelFileTime,
        kernel_time: *mut KernelFileTime,
        user_time: *mut KernelFileTime,
    ) -> i32;
}

#[cfg(target_os = "windows")]
fn sample_cpu_times() -> Option<CpuTimesSample> {
    let mut idle = KernelFileTime {
        low_date_time: 0,
        high_date_time: 0,
    };
    let mut kernel = KernelFileTime {
        low_date_time: 0,
        high_date_time: 0,
    };
    let mut user = KernelFileTime {
        low_date_time: 0,
        high_date_time: 0,
    };
    let ok = unsafe { GetSystemTimes(&mut idle, &mut kernel, &mut user) };
    if ok == 0 {
        return None;
    }
    let idle = filetime_to_u64(idle);
    let kernel = filetime_to_u64(kernel);
    let user = filetime_to_u64(user);
    Some(CpuTimesSample {
        idle,
        total: kernel.saturating_add(user),
    })
}

#[cfg(target_os = "windows")]
fn filetime_to_u64(value: KernelFileTime) -> u64 {
    ((value.high_date_time as u64) << 32) | value.low_date_time as u64
}

#[cfg(not(target_os = "windows"))]
fn sample_cpu_times() -> Option<CpuTimesSample> {
    None
}

fn sample_gpu_telemetry(previous: &mut Option<GpuTelemetrySample>) -> GpuTelemetrySample {
    if let Some(sample) = previous.as_ref() {
        if sample.sampled_at.elapsed() < Duration::from_secs(2) {
            return sample.clone();
        }
    }

    let sample = query_gpu_telemetry().unwrap_or_else(|status| GpuTelemetrySample {
        sampled_at: Instant::now(),
        usage_percent: None,
        memory_used_mb: None,
        memory_total_mb: None,
        status,
    });
    *previous = Some(sample.clone());
    sample
}

#[cfg(target_os = "windows")]
fn query_gpu_telemetry() -> Result<GpuTelemetrySample, String> {
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=utilization.gpu,memory.used,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .map_err(|error| format!("nvidia-smi unavailable: {error}"))?;
    if !output.status.success() {
        return Err("nvidia-smi failed".to_string());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let Some(line) = text
        .lines()
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    else {
        return Err("nvidia-smi returned empty output".to_string());
    };
    let parts = line.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() < 3 {
        return Err(format!("unexpected nvidia-smi output: {line}"));
    }
    Ok(GpuTelemetrySample {
        sampled_at: Instant::now(),
        usage_percent: parts[0].parse::<f32>().ok(),
        memory_used_mb: parts[1].parse::<u64>().ok(),
        memory_total_mb: parts[2].parse::<u64>().ok(),
        status: "nvidia-smi".to_string(),
    })
}

#[cfg(not(target_os = "windows"))]
fn query_gpu_telemetry() -> Result<GpuTelemetrySample, String> {
    Err("GPU telemetry is available only on Windows".to_string())
}

#[tauri::command]
fn start_live_dataset_recording(
    dataset_state: tauri::State<LiveDatasetState>,
) -> Result<LiveDatasetInfo, String> {
    let timestamp = unix_timestamp_millis();
    let root = log_file_path()
        .parent()
        .and_then(Path::parent)
        .map(|path| path.join("datasets").join(format!("live-{timestamp}")))
        .unwrap_or_else(|| PathBuf::from("datasets").join(format!("live-{timestamp}")));
    let frames_dir = root.join("frames");
    std::fs::create_dir_all(&frames_dir).map_err(|error| error.to_string())?;
    let records_path = root.join("records.jsonl");
    let manifest_path = root.join("manifest.json");
    let manifest = serde_json::json!({
        "schema_version": 1,
        "created_at_millis": timestamp,
        "max_frames": LIVE_DATASET_MAX_FRAMES,
        "frame_format": "rgba8",
        "records": "records.jsonl",
        "frames_dir": "frames"
    });
    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    std::fs::File::create(&records_path).map_err(|error| error.to_string())?;

    let mut recorder = dataset_state
        .recorder
        .lock()
        .map_err(|_| "live dataset state lock poisoned".to_string())?;
    *recorder = Some(LiveDatasetRecorder {
        root: root.clone(),
        frames_dir,
        records_path,
        frames_written: 0,
        max_frames: LIVE_DATASET_MAX_FRAMES,
    });
    append_app_log(
        "backend",
        "live dataset recording started",
        Some(&format!(r#"{{"path":"{}"}}"#, root.display())),
    );
    Ok(LiveDatasetInfo {
        path: root.display().to_string(),
        max_frames: LIVE_DATASET_MAX_FRAMES,
    })
}

#[tauri::command]
fn stop_live_dataset_recording(
    dataset_state: tauri::State<LiveDatasetState>,
) -> Result<(), String> {
    let mut recorder = dataset_state
        .recorder
        .lock()
        .map_err(|_| "live dataset state lock poisoned".to_string())?;
    if let Some(recorder) = recorder.take() {
        append_app_log(
            "backend",
            "live dataset recording stopped",
            Some(&format!(
                r#"{{"path":"{}","frames":{}}}"#,
                recorder.root.display(),
                recorder.frames_written
            )),
        );
    }
    Ok(())
}

fn record_live_dataset_frame(
    dataset_state: &LiveDatasetState,
    sequence: u64,
    screen: &ScreenInfo,
    frame: &CapturedFrame,
    inference: &autoaim_infer::InferenceOutput,
    capture_status: &str,
    capture_ms: u128,
    detect_ms: u128,
    tracking_ms: u128,
    total_ms: u128,
    people: &[LivePersonPosition],
) {
    let mut recorder = match dataset_state.recorder.lock() {
        Ok(recorder) => recorder,
        Err(_) => return,
    };
    let Some(recorder) = recorder.as_mut() else {
        return;
    };
    if recorder.frames_written >= recorder.max_frames {
        return;
    }

    let frame_name = format!("frame-{sequence:06}.rgba");
    let frame_path = recorder.frames_dir.join(&frame_name);
    if std::fs::write(&frame_path, &frame.rgba).is_err() {
        append_app_log("backend", "live dataset frame write failed", None);
        return;
    }
    let frame_file = format!("frames/{frame_name}");
    let record = LiveDatasetRecord {
        schema_version: 1,
        sequence,
        timestamp_millis: frame.timestamp_millis,
        screen_id: &screen.id,
        frame_file: &frame_file,
        frame_format: "rgba8",
        frame_size: frame.frame_size,
        screen_origin: frame.screen_origin,
        screen_size: frame.screen_size,
        capture_backend: frame.capture_backend.as_str(),
        cursor: frame.cursor,
        cursor_on_screen: frame.cursor_on_screen,
        provider: inference.provider.as_str(),
        model_status: &inference.model_status,
        capture_status,
        capture_ms,
        detect_ms,
        tracking_ms,
        total_ms,
        people,
    };
    let Ok(line) = serde_json::to_string(&record) else {
        return;
    };
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&recorder.records_path)
    {
        use std::io::Write;
        let _ = writeln!(file, "{line}");
        recorder.frames_written += 1;
        if recorder.frames_written == recorder.max_frames {
            append_app_log(
                "backend",
                "live dataset recording reached frame limit",
                Some(&format!(
                    r#"{{"path":"{}","frames":{}}}"#,
                    recorder.root.display(),
                    recorder.frames_written
                )),
            );
        }
    }
}

#[tauri::command]
fn frontend_log(scope: String, message: String, payload: Option<String>) {
    append_app_log(&scope, &message, payload.as_deref());
}

fn activate_overlay_refresh(
    overlay_state: &OverlayState,
    screen_id: String,
) -> Result<u64, String> {
    {
        let mut active_screen_id = overlay_state
            .active_screen_id
            .lock()
            .map_err(|_| "overlay state lock poisoned".to_string())?;
        *active_screen_id = Some(screen_id);
    }

    let generation = overlay_state
        .refresh_generation
        .fetch_add(1, Ordering::SeqCst)
        .wrapping_add(1);
    append_app_log(
        "backend",
        "overlay refresh activated",
        Some(&format!(r#"{{"generation":{generation}}}"#)),
    );
    Ok(generation)
}

fn deactivate_overlay_refresh(overlay_state: &OverlayState) -> Result<(), String> {
    {
        let mut active_screen_id = overlay_state
            .active_screen_id
            .lock()
            .map_err(|_| "overlay state lock poisoned".to_string())?;
        *active_screen_id = None;
    }

    let previous = overlay_state
        .refresh_generation
        .fetch_add(1, Ordering::SeqCst);
    append_app_log(
        "backend",
        "overlay refresh deactivated",
        Some(&format!(r#"{{"previous_generation":{previous}}}"#)),
    );
    Ok(())
}

fn overlay_refresh_is_current(
    overlay_state: &OverlayState,
    screen_id: &str,
    generation: u64,
) -> bool {
    if overlay_state.refresh_generation.load(Ordering::SeqCst) != generation {
        return false;
    }

    overlay_state
        .active_screen_id
        .lock()
        .map(|active| active.as_deref() == Some(screen_id))
        .unwrap_or(false)
}

fn point_in_overlay_screen(cursor: Point, screen: &ScreenInfo) -> bool {
    let x = cursor[0];
    let y = cursor[1];
    let left = screen.origin[0] as f32;
    let top = screen.origin[1] as f32;
    let right = left + screen.size[0] as f32;
    let bottom = top + screen.size[1] as f32;
    x >= left && x < right && y >= top && y < bottom
}

fn start_overlay_cursor_loop(app: tauri::AppHandle, screen: ScreenInfo, generation: u64) {
    let screen_id = screen.id.clone();
    append_app_log(
        "backend",
        "overlay cursor loop start",
        Some(&format!(
            r#"{{"screen_id":"{}","generation":{generation}}}"#,
            screen_id
        )),
    );
    let mut tick = 0u64;
    thread::spawn(move || loop {
        let overlay_state = app.state::<OverlayState>();
        if !overlay_refresh_is_current(&overlay_state, &screen_id, generation) {
            append_app_log(
                "backend",
                "overlay cursor loop exit",
                Some(&format!(
                    r#"{{"screen_id":"{}","generation":{generation}}}"#,
                    screen_id
                )),
            );
            break;
        }

        let Some(window) = app.get_window(OVERLAY_WINDOW_LABEL) else {
            append_app_log("backend", "overlay cursor loop exit: window missing", None);
            break;
        };
        if let Ok(cursor) = cursor_position() {
            let snapshot = OverlayCursorSnapshot {
                screen_id: screen.id.clone(),
                screen_origin: screen.origin,
                screen_size: screen.size,
                cursor,
                cursor_on_screen: point_in_overlay_screen(cursor, &screen),
            };
            if tick % OVERLAY_CURSOR_LOG_EVERY_TICKS == 0 {
                append_app_log(
                    "backend",
                    "overlay cursor",
                    Some(&format!(
                        r#"{{"screen_id":"{}","cursor":[{:.1},{:.1}],"cursor_on_screen":{}}}"#,
                        snapshot.screen_id,
                        snapshot.cursor[0],
                        snapshot.cursor[1],
                        snapshot.cursor_on_screen
                    )),
                );
            }
            let _ = window.emit("overlay_cursor", &snapshot);
        } else if tick % OVERLAY_CURSOR_LOG_EVERY_TICKS == 0 {
            append_app_log("backend", "overlay cursor error", None);
        }
        tick = tick.wrapping_add(1);

        thread::sleep(Duration::from_millis(OVERLAY_CURSOR_INTERVAL_MS));
    });
}

#[tauri::command]
fn diagnostics_context(
    overlay_state: tauri::State<OverlayState>,
    window: tauri::Window,
    selected_screen_id: Option<String>,
    provider: Option<String>,
    model_path: Option<String>,
    confidence_threshold: Option<f32>,
) -> Result<DiagnosticContext, String> {
    let screens = platform_list_screens(&window)?;
    let provider_name = provider.unwrap_or_else(|| "directml".to_string());
    let provider = NativeInferenceProvider::from_name(&provider_name)
        .ok_or_else(|| format!("unsupported inference provider: {provider_name}"))?;
    let resolved_model_path =
        resolve_model_path(model_path, native_provider_prefers_onnx(provider));
    let overlay_screen_id = overlay_state
        .active_screen_id
        .lock()
        .map_err(|_| "overlay state lock poisoned".to_string())?
        .clone();

    Ok(DiagnosticContext {
        app_name: "AutoAim Review",
        app_version: env!("CARGO_PKG_VERSION"),
        requested_provider: provider.as_str().to_string(),
        resolved_model_path,
        confidence_threshold: confidence_threshold.unwrap_or(0.25),
        selected_screen_id,
        screens,
        overlay_enabled: overlay_screen_id.is_some(),
        overlay_screen_id,
    })
}

#[tauri::command]
async fn open_overlay_window(
    app: tauri::AppHandle,
    overlay_state: tauri::State<'_, OverlayState>,
    window: tauri::Window,
    screen_id: String,
    model_path: Option<String>,
    provider: Option<String>,
    confidence_threshold: Option<f32>,
) -> Result<(), String> {
    let screens = platform_list_screens(&window)?;
    let screen = screens
        .iter()
        .find(|item| item.id == screen_id)
        .cloned()
        .ok_or_else(|| format!("screen not found: {screen_id}"))?;
    let _ = (provider, model_path, confidence_threshold);

    let overlay = if let Some(existing) = app.get_window(OVERLAY_WINDOW_LABEL) {
        existing
    } else {
        let overlay_builder = tauri::WindowBuilder::new(
            &app,
            OVERLAY_WINDOW_LABEL,
            tauri::WindowUrl::App("overlay.html".into()),
        )
        .title("AutoAim Overlay");
        #[cfg(not(target_os = "macos"))]
        let overlay_builder = overlay_builder.transparent(true);
        let built_overlay = overlay_builder
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .focused(false)
            .visible(false)
            .build()
            .map_err(|error| format!("failed to create overlay window: {error}"))?;
        built_overlay
    };

    sync_overlay_window(&overlay, &screen)?;
    let _ = overlay.set_ignore_cursor_events(true);
    overlay
        .show()
        .map_err(|error| format!("failed to show overlay window: {error}"))?;
    let generation = activate_overlay_refresh(&overlay_state, screen.id.clone())?;
    start_overlay_cursor_loop(app.clone(), screen.clone(), generation);
    Ok(())
}

#[tauri::command]
fn close_overlay_window(
    app: tauri::AppHandle,
    overlay_state: tauri::State<OverlayState>,
) -> Result<(), String> {
    if let Some(window) = app.get_window(OVERLAY_WINDOW_LABEL) {
        window
            .hide()
            .map_err(|error| format!("failed to hide overlay window: {error}"))?;
    }
    deactivate_overlay_refresh(&overlay_state)?;
    Ok(())
}

fn sync_overlay_window(window: &tauri::Window, screen: &ScreenInfo) -> Result<(), String> {
    window
        .set_position(tauri::PhysicalPosition::new(
            screen.origin[0],
            screen.origin[1],
        ))
        .map_err(|error| format!("failed to set overlay position: {error}"))?;
    window
        .set_size(tauri::PhysicalSize::new(screen.size[0], screen.size[1]))
        .map_err(|error| format!("failed to set overlay size: {error}"))?;
    enforce_overlay_topmost(window)?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn enforce_overlay_topmost(window: &tauri::Window) -> Result<(), String> {
    let hwnd = window
        .hwnd()
        .map_err(|error| format!("failed to get overlay hwnd: {error}"))?;
    let raw_hwnd = hwnd.0 as isize;
    let hwnd = raw_hwnd as _;
    unsafe {
        let style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        let next_style = style
            | WS_EX_LAYERED
            | WS_EX_TRANSPARENT
            | WS_EX_TOOLWINDOW
            | WS_EX_TOPMOST
            | WS_EX_NOACTIVATE;
        SetWindowLongW(hwnd, GWL_EXSTYLE, next_style as i32);
        let ok = SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
        if ok == 0 {
            return Err("failed to force overlay topmost window style".to_string());
        }
        if SetWindowDisplayAffinity(raw_hwnd, WDA_EXCLUDEFROMCAPTURE) == 0 {
            append_app_log("backend", "overlay exclude-from-capture unsupported", None);
        }
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn enforce_overlay_topmost(_window: &tauri::Window) -> Result<(), String> {
    Ok(())
}

fn platform_list_screens(window: &tauri::Window) -> Result<Vec<ScreenInfo>, String> {
    let monitors = window
        .available_monitors()
        .map_err(|error| format!("failed to list screens: {error}"))?;
    let primary = window
        .primary_monitor()
        .map_err(|error| format!("failed to query primary screen: {error}"))?;
    let primary_key = primary.as_ref().map(monitor_key);

    let screens = monitors
        .iter()
        .enumerate()
        .map(|(index, monitor)| {
            let size = monitor.size();
            let position = monitor.position();
            let key = monitor_key(monitor);
            ScreenInfo {
                id: format!("monitor-{}", index + 1),
                name: monitor
                    .name()
                    .cloned()
                    .unwrap_or_else(|| format!("Display {}", index + 1)),
                origin: [position.x, position.y],
                size: [size.width, size.height],
                primary: primary_key.as_ref() == Some(&key),
            }
        })
        .collect::<Vec<_>>();

    if screens.is_empty() {
        return Err("no screens found".to_string());
    }

    Ok(screens)
}

fn monitor_key(monitor: &tauri::Monitor) -> (i32, i32, u32, u32) {
    let position = monitor.position();
    let size = monitor.size();
    (position.x, position.y, size.width, size.height)
}

#[tauri::command]
fn validate_dataset(path: String) -> Result<Vec<ValidationDiagnostic>, String> {
    let records = read_jsonl_path(PathBuf::from(path)).map_err(|error| error.to_string())?;
    Ok(validate_records(&records))
}

#[tauri::command]
fn evaluate_dataset(path: String) -> Result<EvaluationResult, String> {
    let records = read_jsonl_path(PathBuf::from(path)).map_err(|error| error.to_string())?;
    let diagnostics = validate_records(&records);
    let suggestions = suggest_frames(&records, TargetScorer::default());
    let summary = summarize(&records, &suggestions);

    Ok(EvaluationResult {
        summary,
        diagnostics,
    })
}

#[tauri::command]
fn preview_events(path: String, limit: Option<usize>) -> Result<PreviewResult, String> {
    let records = read_jsonl_path(PathBuf::from(path)).map_err(|error| error.to_string())?;
    let mut pipeline = ReviewPipeline::default();
    let all_events = pipeline.process_records(&records);
    let total_events = all_events.len();
    let limit = limit.unwrap_or(20).min(total_events);

    Ok(PreviewResult {
        events: all_events.into_iter().take(limit).collect(),
        total_events,
    })
}

#[tauri::command]
fn preview_person_positions(
    path: String,
    limit: Option<usize>,
) -> Result<PositionPreviewResult, String> {
    let records = read_jsonl_path(PathBuf::from(path)).map_err(|error| error.to_string())?;
    let mut pipeline = ReviewPipeline::default();
    let limit = limit.unwrap_or(50);
    let mut positions = Vec::new();
    let mut assist_events = Vec::new();
    let mut total_people = 0;

    for record in &records {
        let (_result, assist_event) = pipeline.process_frame_with_assist(record);
        if let Some(event) = assist_event {
            assist_events.push(event);
        }

        for (object_index, object) in record.objects.iter().enumerate() {
            if object.class_name != "person" {
                continue;
            }

            total_people += 1;
            if positions.len() >= limit {
                continue;
            }

            positions.push(person_position(
                record.frame_id,
                object_index,
                object,
                record,
            ));
        }
    }

    Ok(PositionPreviewResult {
        positions,
        assist_events,
        total_frames: records.len(),
        total_people,
    })
}

fn person_position(
    frame_id: u64,
    object_index: usize,
    object: &DetectionObject,
    record: &autoaim_core::FrameRecord,
) -> PersonPosition {
    let head_point = object.aim_point();
    let cursor = record.input.cursor;
    PersonPosition {
        frame_id,
        object_index,
        bbox: object.bbox,
        head_point,
        confidence: object.confidence,
        track_id: object.track_id,
        cursor,
        dx: head_point[0] - cursor[0],
        dy: head_point[1] - cursor[1],
        left_mouse_down: record.input.mouse_down,
    }
}

#[tauri::command]
fn write_events(input_path: String, output_path: String) -> Result<WriteEventsResult, String> {
    let records = read_jsonl_path(PathBuf::from(input_path)).map_err(|error| error.to_string())?;
    let mut pipeline = ReviewPipeline::default();
    let mut writer =
        JsonlEventWriter::create(PathBuf::from(&output_path)).map_err(|error| error.to_string())?;

    let mut written_events = 0;
    for record in &records {
        let result = pipeline.process_frame(record);
        writer
            .write_inference_result(&result)
            .map_err(|error| error.to_string())?;
        written_events += 1;
    }
    writer.flush().map_err(|error| error.to_string())?;

    Ok(WriteEventsResult {
        output_path,
        written_events,
    })
}

#[tauri::command]
async fn check_updates(install_dir: Option<String>) -> Result<UpdateCommandResult, String> {
    tauri::async_runtime::spawn_blocking(move || run_update_command(install_dir, true))
        .await
        .map_err(|error| format!("update check worker failed: {error}"))?
}

#[tauri::command]
fn apply_update(
    app: tauri::AppHandle,
    install_dir: Option<String>,
) -> Result<UpdateCommandResult, String> {
    let script = find_update_script(install_dir.as_deref())?;
    let install_root = install_root_for(script.as_path(), install_dir.as_deref())?;

    #[cfg(target_os = "windows")]
    {
        let command = format!(
            "Start-Sleep -Seconds 2; & '{}' -InstallDir '{}'",
            escape_powershell_path(&script),
            escape_powershell_path(&install_root)
        );
        Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &command,
            ])
            .spawn()
            .map_err(|error| format!("failed to start updater: {error}"))?;
        app.exit(0);
        Ok(UpdateCommandResult {
            success: true,
            output:
                "Updater started. AutoAim Review will close so the update can replace app files."
                    .to_string(),
            update_available: None,
            installed_version: None,
            latest_version: None,
        })
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, install_root);
        Err("self-update is available only in the Windows installed app".to_string())
    }
}

fn run_update_command(
    install_dir: Option<String>,
    check_only: bool,
) -> Result<UpdateCommandResult, String> {
    let script = find_update_script(install_dir.as_deref())?;
    let install_root = install_root_for(script.as_path(), install_dir.as_deref())?;

    #[cfg(not(target_os = "windows"))]
    {
        let _ = (script, install_root, check_only);
        return Err("self-update is available only in the Windows installed app".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        let mut command = Command::new("powershell.exe");
        command.args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            script.to_string_lossy().as_ref(),
            "-InstallDir",
            install_root.to_string_lossy().as_ref(),
        ]);
        if check_only {
            command.arg("-CheckOnly");
        }
        let output = command
            .output()
            .map_err(|error| format!("failed to run updater: {error}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let text = if stderr.trim().is_empty() {
            stdout.to_string()
        } else {
            format!("{stdout}\n{stderr}")
        };

        let (update_available, installed_version, latest_version) =
            parse_update_check_output(&text);
        Ok(UpdateCommandResult {
            success: output.status.success(),
            output: text,
            update_available,
            installed_version,
            latest_version,
        })
    }
}

#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
fn parse_update_check_output(text: &str) -> (Option<bool>, Option<String>, Option<String>) {
    let mut installed_version = None;
    let mut latest_version = None;
    let mut update_available = None;

    for line in text.lines().map(str::trim) {
        if let Some(value) = line.strip_prefix("Installed version:") {
            installed_version = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("Latest version:") {
            latest_version = Some(value.trim().to_string());
        } else if line.contains("Incremental update available") {
            update_available = Some(true);
        } else if line.contains("Already up to date") {
            update_available = Some(false);
        }
    }

    if update_available.is_none() {
        if let (Some(installed), Some(latest)) = (&installed_version, &latest_version) {
            update_available = Some(installed != latest);
        }
    }

    (update_available, installed_version, latest_version)
}

fn find_update_script(install_dir: Option<&str>) -> Result<PathBuf, String> {
    if let Some(root) = install_dir.filter(|value| !value.trim().is_empty()) {
        let script = PathBuf::from(root).join("windows").join("update.ps1");
        if script.is_file() {
            return Ok(script);
        }
        return Err(format!("update script not found at {}", script.display()));
    }

    let current_exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let Some(root) = current_exe.parent() else {
        return Err("cannot resolve app install directory".to_string());
    };
    let script = root.join("windows").join("update.ps1");
    if script.is_file() {
        return Ok(script);
    }

    Err(format!(
        "update script not found at {}. Run from an installed Windows package.",
        script.display()
    ))
}

fn install_root_for(script: &Path, install_dir: Option<&str>) -> Result<PathBuf, String> {
    if let Some(root) = install_dir.filter(|value| !value.trim().is_empty()) {
        return Ok(PathBuf::from(root));
    }

    script
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "cannot resolve install root from update script".to_string())
}

#[cfg(target_os = "windows")]
fn escape_powershell_path(path: &Path) -> String {
    path.to_string_lossy().replace('\'', "''")
}

fn main() {
    tauri::Builder::default()
        .manage(LiveTrackingState::default())
        .manage(LiveDetectorState::default())
        .manage(LiveCaptureState::default())
        .manage(LiveSnapshotState::default())
        .manage(LiveDatasetState::default())
        .manage(SystemTelemetryState::default())
        .manage(OverlayState::default())
        .setup(|_app| {
            let log_path = log_file_path();
            append_app_log(
                "backend",
                "application startup",
                Some(&format!(r#"{{"log_path":"{}"}}"#, log_path.display())),
            );
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_info,
            frontend_log,
            inference_runtime_config,
            list_screens,
            set_compact_window,
            live_monitor_snapshot,
            start_live_dataset_recording,
            stop_live_dataset_recording,
            diagnostics_context,
            open_overlay_window,
            close_overlay_window,
            validate_dataset,
            evaluate_dataset,
            preview_events,
            preview_person_positions,
            write_events,
            check_updates,
            apply_update
        ])
        .run(tauri::generate_context!())
        .expect("failed to run AutoAim Review");
}
