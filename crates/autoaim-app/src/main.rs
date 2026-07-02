#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

use autoaim_core::{
    read_jsonl_path, suggest_frames, summarize, validate_records, MetricsSummary, TargetScorer,
    ValidationDiagnostic,
};
use autoaim_ipc::InferenceResult;
use autoaim_runtime::{JsonlEventWriter, ReviewPipeline};
use serde::Serialize;
use std::path::PathBuf;

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
struct WriteEventsResult {
    output_path: String,
    written_events: usize,
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        app_name: "AutoAim Review",
        runtime: "Rust + Tauri",
        safety_note: "Visualization-only: no mouse movement, clicks, input injection, or process control.",
    }
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

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            app_info,
            validate_dataset,
            evaluate_dataset,
            preview_events,
            write_events
        ])
        .run(tauri::generate_context!())
        .expect("failed to run AutoAim Review");
}
