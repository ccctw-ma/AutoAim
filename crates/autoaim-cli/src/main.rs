use anyhow::Result;
use autoaim_core::{read_jsonl_path, suggest_frames, summarize, validate_records, TargetScorer};
use autoaim_ipc::encode_json_line;
use autoaim_runtime::{
    mock_native_detections, JsonlEventWriter, LiveDetectionInput, ReviewPipeline,
};
use clap::{Parser, Subcommand};
use serde::Serialize;
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::ExitCode,
    time::Instant,
};

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

#[derive(Debug, Serialize)]
struct DetectorBenchResult {
    images: usize,
    iterations: usize,
    total_runs: usize,
    total_ms: f64,
    mean_ms: f64,
    min_ms: f64,
    p50_ms: f64,
    p95_ms: f64,
    max_ms: f64,
    detections_per_run: f64,
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
    /// Benchmark the current Rust live detector path on image dimensions.
    BenchDetector {
        /// Directory containing PNG/JPEG images.
        image_dir: PathBuf,
        /// Iterations per image.
        #[arg(long, default_value_t = 1000)]
        iterations: usize,
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
        Command::BenchDetector {
            image_dir,
            iterations,
        } => bench_detector(image_dir, iterations),
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

fn bench_detector(image_dir: PathBuf, iterations: usize) -> Result<()> {
    let mut image_sizes = Vec::new();
    for entry in fs::read_dir(&image_dir)? {
        let path = entry?.path();
        if !path.is_file() {
            continue;
        }
        if let Some(size) = read_image_size(&path)? {
            image_sizes.push((path, size));
        }
    }

    if image_sizes.is_empty() {
        anyhow::bail!("no PNG/JPEG images found in {}", image_dir.display());
    }

    let iterations = iterations.max(1);
    let mut samples = Vec::with_capacity(image_sizes.len() * iterations);
    let mut detection_count = 0usize;

    for (_path, [width, height]) in &image_sizes {
        let input = LiveDetectionInput {
            screen_origin: [0, 0],
            screen_size: [*width, *height],
            frame_size: [*width, *height],
            cursor: [*width as f32 / 2.0, *height as f32 / 2.0],
            confidence_threshold: 0.25,
        };

        for _ in 0..iterations {
            let start = Instant::now();
            let detections = mock_native_detections(&input);
            let elapsed = start.elapsed().as_secs_f64() * 1000.0;
            detection_count += detections.len();
            samples.push(elapsed);
        }
    }

    samples.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let total_runs = samples.len();
    let total_ms: f64 = samples.iter().sum();
    let result = DetectorBenchResult {
        images: image_sizes.len(),
        iterations,
        total_runs,
        total_ms,
        mean_ms: total_ms / total_runs as f64,
        min_ms: samples[0],
        p50_ms: percentile(&samples, 0.50),
        p95_ms: percentile(&samples, 0.95),
        max_ms: samples[total_runs - 1],
        detections_per_run: detection_count as f64 / total_runs as f64,
    };

    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let index = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[index.min(sorted.len() - 1)]
}

fn read_image_size(path: &Path) -> Result<Option<[u32; 2]>> {
    let file = fs::File::open(path)?;
    let mut bytes = Vec::new();
    file.take(128 * 1024).read_to_end(&mut bytes)?;

    if let Some(size) = png_size(&bytes) {
        return Ok(Some(size));
    }
    if let Some(size) = jpeg_size(&bytes) {
        return Ok(Some(size));
    }
    Ok(None)
}

fn png_size(bytes: &[u8]) -> Option<[u32; 2]> {
    if bytes.len() < 24 || &bytes[0..8] != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    Some([
        u32::from_be_bytes(bytes[16..20].try_into().ok()?),
        u32::from_be_bytes(bytes[20..24].try_into().ok()?),
    ])
}

fn jpeg_size(bytes: &[u8]) -> Option<[u32; 2]> {
    if bytes.len() < 4 || bytes[0] != 0xff || bytes[1] != 0xd8 {
        return None;
    }

    let mut index = 2usize;
    while index + 9 < bytes.len() {
        while index < bytes.len() && bytes[index] != 0xff {
            index += 1;
        }
        while index < bytes.len() && bytes[index] == 0xff {
            index += 1;
        }
        if index >= bytes.len() {
            return None;
        }
        let marker = bytes[index];
        index += 1;
        if matches!(marker, 0xd8 | 0xd9 | 0x01) {
            continue;
        }
        if index + 2 > bytes.len() {
            return None;
        }
        let length = u16::from_be_bytes(bytes[index..index + 2].try_into().ok()?) as usize;
        if length < 2 || index + length > bytes.len() {
            return None;
        }
        if matches!(
            marker,
            0xc0 | 0xc1
                | 0xc2
                | 0xc3
                | 0xc5
                | 0xc6
                | 0xc7
                | 0xc9
                | 0xca
                | 0xcb
                | 0xcd
                | 0xce
                | 0xcf
        ) {
            if index + 7 > bytes.len() {
                return None;
            }
            let height = u16::from_be_bytes(bytes[index + 3..index + 5].try_into().ok()?) as u32;
            let width = u16::from_be_bytes(bytes[index + 5..index + 7].try_into().ok()?) as u32;
            return Some([width, height]);
        }
        index += length;
    }
    None
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
