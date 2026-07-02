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
use autoaim_runtime::{JsonlEventWriter, ReviewPipeline};
use serde::Serialize;
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
