use anyhow::Result;
use autoaim_core::{
    read_jsonl_path, summarize, suggest_frames, validate_records, TargetScorer,
};
use autoaim_ipc::encode_json_line;
use autoaim_runtime::{JsonlEventWriter, ReviewPipeline};
use clap::{Parser, Subcommand};
use std::{path::PathBuf, process::ExitCode};

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
