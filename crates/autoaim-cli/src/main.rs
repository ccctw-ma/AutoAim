use anyhow::Result;
use autoaim_core::{read_jsonl_path, suggest_frames, summarize, validate_records, TargetScorer};
use autoaim_ipc::encode_json_line;
use autoaim_runtime::{JsonlEventWriter, ReviewPipeline};
use clap::{Parser, Subcommand};
use serde::Serialize;
use std::{path::PathBuf, process::ExitCode};

#[derive(Debug, Serialize)]
struct PersonPosition {
    frame_id: u64,
    object_index: usize,
    bbox: [f32; 4],
    head_point: [f32; 2],
    confidence: f32,
    track_id: Option<u64>,
    cursor: [f32; 2],
    dx: f32,
    dy: f32,
    left_mouse_down: bool,
}

#[derive(Debug, Parser)]
#[command(name = "autoaim")]
#[command(about = "Rust runtime CLI for AutoAim Review")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Validate dataset JSONL records.
    Validate {
        /// Input JSONL file.
        input: PathBuf,
        /// Emit machine-readable JSON diagnostics.
        #[arg(long)]
        json: bool,
    },
    /// Evaluate target suggestions and print aggregate metrics.
    Evaluate {
        /// Input JSONL file.
        input: PathBuf,
        /// Emit machine-readable JSON summary.
        #[arg(long)]
        json: bool,
    },
    /// Convert JSONL frame records into inference.result events.
    Suggest {
        /// Input JSONL file.
        input: PathBuf,
    },
    /// Run the review pipeline from JSONL frames and write inference events.
    RunJsonl {
        /// Input frame JSONL file.
        input: PathBuf,
        /// Output inference event JSONL file.
        output: PathBuf,
    },
    /// Print person body/head screen positions from frame records.
    Positions {
        /// Input JSONL file.
        input: PathBuf,
        /// Also emit review-only assist.suggestion events for mouse_down frames.
        #[arg(long)]
        assist_events: bool,
    },
    /// Check for or apply updates from the Windows binary installation.
    Update {
        /// Show detected upstream changes without updating files.
        #[arg(long)]
        check: bool,
        /// Print extra delta details when an incremental update is detected.
        #[arg(long)]
        show_diff: bool,
        /// Override the installation directory used by windows/update.ps1.
        #[arg(long)]
        install_dir: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Validate { input, json } => validate(input, json),
        Command::Evaluate { input, json } => evaluate(input, json),
        Command::Suggest { input } => suggest(input),
        Command::RunJsonl { input, output } => run_jsonl(input, output),
        Command::Positions {
            input,
            assist_events,
        } => positions(input, assist_events),
        Command::Update {
            check,
            show_diff,
            install_dir,
        } => update(check, show_diff, install_dir),
    }
}

fn validate(input: PathBuf, json: bool) -> Result<()> {
    let records = read_jsonl_path(input)?;
    let diagnostics = validate_records(&records);

    if json {
        println!("{}", serde_json::to_string_pretty(&diagnostics)?);
    } else if diagnostics.is_empty() {
        println!("ok: {} frame records", records.len());
    } else {
        for diagnostic in &diagnostics {
            match diagnostic.frame_id {
                Some(frame_id) => eprintln!("frame {frame_id}: {}", diagnostic.message),
                None => eprintln!("{}", diagnostic.message),
            }
        }
    }

    if diagnostics.is_empty() {
        Ok(())
    } else {
        anyhow::bail!("validation failed with {} diagnostics", diagnostics.len())
    }
}

fn evaluate(input: PathBuf, json: bool) -> Result<()> {
    let records = read_jsonl_path(input)?;
    let suggestions = suggest_frames(&records, TargetScorer::default());
    let summary = summarize(&records, &suggestions);

    if json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        println!("frames: {}", summary.frame_count);
        println!("objects: {}", summary.object_count);
        println!("targets: {}", summary.target_count);
        println!("mean_confidence: {:.4}", summary.mean_confidence);
        println!("mean_abs_dx: {:.2}", summary.mean_abs_dx);
        println!("mean_abs_dy: {:.2}", summary.mean_abs_dy);
        println!("mean_distance: {:.2}", summary.mean_distance);
    }

    Ok(())
}

fn suggest(input: PathBuf) -> Result<()> {
    let records = read_jsonl_path(input)?;
    let mut pipeline = ReviewPipeline::default();

    for record in &records {
        let result = pipeline.process_frame(record);
        print!("{}", encode_json_line(&result)?);
    }

    Ok(())
}

fn run_jsonl(input: PathBuf, output: PathBuf) -> Result<()> {
    let records = read_jsonl_path(input)?;
    let mut pipeline = ReviewPipeline::default();
    let mut writer = JsonlEventWriter::create(output)?;

    for record in &records {
        let result = pipeline.process_frame(record);
        writer.write_inference_result(&result)?;
    }

    writer.flush()?;
    println!("processed {} frame records", records.len());
    Ok(())
}

fn positions(input: PathBuf, assist_events: bool) -> Result<()> {
    let records = read_jsonl_path(input)?;
    let mut pipeline = ReviewPipeline::default();

    for record in &records {
        let (_result, assist) = pipeline.process_frame_with_assist(record);
        for (object_index, object) in record.objects.iter().enumerate() {
            if object.class_name != "person" {
                continue;
            }

            let head_point = object.aim_point();
            let cursor = record.input.cursor;
            let position = PersonPosition {
                frame_id: record.frame_id,
                object_index,
                bbox: object.bbox,
                head_point,
                confidence: object.confidence,
                track_id: object.track_id,
                cursor,
                dx: head_point[0] - cursor[0],
                dy: head_point[1] - cursor[1],
                left_mouse_down: record.input.mouse_down,
            };
            println!("{}", serde_json::to_string(&position)?);
        }

        if assist_events {
            if let Some(event) = assist {
                println!("{}", serde_json::to_string(&event)?);
            }
        }
    }

    Ok(())
}

fn update(check: bool, show_diff: bool, install_dir: Option<PathBuf>) -> Result<()> {
    #[cfg(windows)]
    {
        return run_windows_updater(check, show_diff, install_dir);
    }

    #[cfg(not(windows))]
    {
        let _ = (check, show_diff, install_dir);
        anyhow::bail!(
            "autoaim update is available only for the Windows installation created by windows/install.ps1"
        )
    }
}

#[cfg(windows)]
fn run_windows_updater(check: bool, show_diff: bool, install_dir: Option<PathBuf>) -> Result<()> {
    use anyhow::Context;
    use std::process::Command as ProcessCommand;

    let updater = find_windows_updater()?;
    let mut command = ProcessCommand::new(&updater);

    if check {
        command.arg("-CheckOnly");
    }
    if show_diff {
        command.arg("-ShowDiff");
    }
    if let Some(path) = install_dir {
        command.arg("-InstallDir").arg(path);
    }

    let status = command
        .status()
        .with_context(|| format!("failed to run updater at {}", updater.display()))?;
    if !status.success() {
        anyhow::bail!("updater exited with status {status}");
    }

    Ok(())
}

#[cfg(windows)]
fn find_windows_updater() -> Result<PathBuf> {
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(bin_dir) = current_exe.parent() {
            let sibling = bin_dir.join("autoaim-update.cmd");
            if sibling.is_file() {
                return Ok(sibling);
            }
        }
    }

    Ok(PathBuf::from("autoaim-update.cmd"))
}
