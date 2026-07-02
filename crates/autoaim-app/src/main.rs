#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use autoaim_core::{
    read_jsonl_path, suggest_frames, summarize, validate_records, DetectionObject, MetricsSummary,
    Point, TargetScorer, ValidationDiagnostic,
};
use autoaim_ipc::{
    AssistSuggestionEvent, InferenceProvider, InferenceResult, InferenceRuntimeConfig,
};
use autoaim_runtime::{
    mock_native_detections, JsonlEventWriter, LiveDetectionInput, ReviewPipeline,
};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Serialize)]
struct AppInfo {
    app_name: &'static str,
    runtime: &'static str,
    safety_note: &'static str,
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
}

#[derive(Debug, Serialize)]
struct ScreenInfo {
    id: String,
    name: String,
    origin: [i32; 2],
    size: [u32; 2],
    primary: bool,
}

#[derive(Debug, Serialize)]
struct LivePersonPosition {
    object_index: usize,
    bbox: [f32; 4],
    head_point: Point,
    confidence: f32,
    track_id: Option<u64>,
    dx: f32,
    dy: f32,
}

#[derive(Debug, Serialize)]
struct LiveMonitorSnapshot {
    screen_id: String,
    cursor: Point,
    cursor_on_screen: bool,
    people: Vec<LivePersonPosition>,
    model_status: String,
    capture_status: String,
    review_only: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LiveDetectionRequest {
    screen_id: String,
    screen_origin: [i32; 2],
    screen_size: [u32; 2],
    frame_size: [u32; 2],
    cursor: Point,
    model_path: Option<String>,
    provider: Option<String>,
    confidence_threshold: Option<f32>,
}

#[derive(Debug, Serialize)]
struct LiveDetectionResult {
    screen_id: String,
    people: Vec<LivePersonPosition>,
    model_status: String,
    provider: String,
    review_only: bool,
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        app_name: "AutoAim Review",
        runtime: "Rust + Tauri",
        safety_note:
            "Visualization-only: no mouse movement, clicks, input injection, or process control.",
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
        .unwrap_or_else(|| "cuda".to_string())
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
        model_path.filter(|value| !value.trim().is_empty()),
        device_id,
        confidence_threshold.unwrap_or(0.25),
    ))
}

#[tauri::command]
fn list_screens(window: tauri::Window) -> Result<Vec<ScreenInfo>, String> {
    platform_list_screens(&window)
}

#[tauri::command]
fn live_monitor_snapshot(
    window: tauri::Window,
    screen_id: String,
    model_path: Option<String>,
    provider: Option<String>,
) -> Result<LiveMonitorSnapshot, String> {
    let screens = platform_list_screens(&window)?;
    let screen = screens
        .iter()
        .find(|item| item.id == screen_id)
        .or_else(|| screens.iter().find(|item| item.primary))
        .or_else(|| screens.first())
        .ok_or_else(|| "no screens found".to_string())?;
    let cursor = platform_cursor_position()?;
    let cursor_on_screen = point_in_screen(cursor, screen);
    let model_configured = model_path
        .as_ref()
        .map(|value| !value.trim().is_empty() && Path::new(value).is_file())
        .unwrap_or(false);
    let provider = provider.unwrap_or_else(|| "cuda".to_string());
    let model_status = if model_configured {
        format!(
            "model configured for {provider}; live capture/inference backend is pending integration"
        )
    } else {
        "no model configured; person list is empty".to_string()
    };

    Ok(LiveMonitorSnapshot {
        screen_id: screen.id.clone(),
        cursor,
        cursor_on_screen,
        people: Vec::new(),
        model_status,
        capture_status:
            "screen video is provided by the system picker; backend reports cursor position"
                .to_string(),
        review_only: true,
    })
}

#[tauri::command]
fn detect_live_frame(request: LiveDetectionRequest) -> Result<LiveDetectionResult, String> {
    let provider = request.provider.unwrap_or_else(|| "cuda".to_string());
    let threshold = request.confidence_threshold.unwrap_or(0.25).clamp(0.0, 1.0);
    let model_path = request
        .model_path
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    let model_configured = model_path
        .map(|value| Path::new(value).is_file())
        .unwrap_or(false);

    let (people, model_status) = if model_configured {
        (
            mock_native_detector(&request, threshold),
            format!(
                "native {provider} detector interface ready; ONNX/TensorRT execution backend pending model binding"
            ),
        )
    } else {
        (
            mock_native_detector(&request, threshold),
            "mock Rust detector active; configure an ONNX/TensorRT model for native GPU inference"
                .to_string(),
        )
    };

    Ok(LiveDetectionResult {
        screen_id: request.screen_id,
        people,
        model_status,
        provider,
        review_only: true,
    })
}

fn mock_native_detector(request: &LiveDetectionRequest, threshold: f32) -> Vec<LivePersonPosition> {
    let input = LiveDetectionInput {
        screen_origin: request.screen_origin,
        screen_size: request.screen_size,
        frame_size: request.frame_size,
        cursor: request.cursor,
        confidence_threshold: threshold,
    };

    mock_native_detections(&input)
        .into_iter()
        .map(|detection| LivePersonPosition {
            object_index: detection.object_index,
            bbox: detection.bbox,
            head_point: detection.head_point,
            confidence: detection.confidence,
            track_id: detection.track_id,
            dx: detection.dx,
            dy: detection.dy,
        })
        .collect()
}

fn point_in_screen(point: Point, screen: &ScreenInfo) -> bool {
    let x = point[0];
    let y = point[1];
    let left = screen.origin[0] as f32;
    let top = screen.origin[1] as f32;
    let right = left + screen.size[0] as f32;
    let bottom = top + screen.size[1] as f32;
    x >= left && x < right && y >= top && y < bottom
}

#[cfg(target_os = "windows")]
fn platform_cursor_position() -> Result<Point, String> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let mut point = POINT { x: 0, y: 0 };
    let ok = unsafe { GetCursorPos(&mut point) };
    if ok == 0 {
        return Err("GetCursorPos failed".to_string());
    }

    Ok([point.x as f32, point.y as f32])
}

#[cfg(not(target_os = "windows"))]
fn platform_cursor_position() -> Result<Point, String> {
    Err("live cursor monitoring is available only on Windows".to_string())
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
fn check_updates(install_dir: Option<String>) -> Result<UpdateCommandResult, String> {
    run_update_command(install_dir, true)
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
        })
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = app;
        Err("self-update is available only in the Windows installed app".to_string())
    }
}

fn run_update_command(
    install_dir: Option<String>,
    check_only: bool,
) -> Result<UpdateCommandResult, String> {
    let script = find_update_script(install_dir.as_deref())?;
    let install_root = install_root_for(script.as_path(), install_dir.as_deref())?;

    #[cfg(target_os = "windows")]
    let output = {
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
        command
            .output()
            .map_err(|error| format!("failed to run updater: {error}"))?
    };

    #[cfg(not(target_os = "windows"))]
    let output = {
        let _ = (script, install_root, check_only);
        return Err("self-update is available only in the Windows installed app".to_string());
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let text = if stderr.trim().is_empty() {
        stdout.to_string()
    } else {
        format!("{stdout}\n{stderr}")
    };

    Ok(UpdateCommandResult {
        success: output.status.success(),
        output: text,
    })
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
        .invoke_handler(tauri::generate_handler![
            app_info,
            inference_runtime_config,
            list_screens,
            live_monitor_snapshot,
            detect_live_frame,
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
