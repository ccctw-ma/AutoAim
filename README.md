# AutoAim Review

AutoAim Review is a visualization-only screen capture, inference review, and
dataset tooling project. It is designed for legal training/review environments:
the software may display model suggestions, log datasets, and compute metrics,
but it must not move the operating-system cursor, click, inject input, or
control third-party games.

The project is now Rust-first for the Windows runtime. Python is kept for
training, dataset conversion, validation, and offline evaluation.

## Scope

Implemented now:

- Rust workspace with `autoaim-core`, `autoaim-ipc`, `autoaim-runtime`, and
  `autoaim-cli`.
- Rust frame/detection models compatible with the JSONL schema.
- Rust JSONL dataset reader/writer.
- Rust target scoring that outputs `suggested_point` and `dx/dy`.
- Rust validation, evaluation summary, person/head position, and inference
  event CLI.
- Rust + Tauri GUI with a live monitor first screen, screen selection, live
  screen preview, real-time cursor position, browser-side person detection,
  offline JSONL tools, self-update controls, and English/Chinese switching.
- Windows Setup installer, package logo assets, and installer-created
  shortcuts.
- Frame annotation data model.
- Python evaluation CLI.
- Python dataset validation CLI.
- YOLO person/head label export.
- Session/scene grouped split planning.
- JSON schema for frame records.
- IPC contract and Rust-first Windows runtime architecture notes.

Planned Rust runtime:

```text
Rust Desktop Shell
 -> Rust Windows Capture
 -> D3D11 / GPU Frame Ring Buffer
 -> Rust Preprocess
 -> ONNX Runtime / TensorRT Inference
 -> Rust Tracker + Target Scorer
 -> Rust Overlay Renderer
 -> Rust Dataset Logger
```

Rust workspace:

```text
crates/
  autoaim-core/        # implemented: models, JSONL, validation, scoring
  autoaim-ipc/         # implemented: IPC event schemas
  autoaim-runtime/     # implemented: frame -> inference event pipeline
  autoaim-cli/         # implemented: validate / evaluate / suggest commands
  autoaim-capture/     # planned: Windows.Graphics.Capture through windows-rs
  autoaim-infer/       # planned: ONNX Runtime wrapper, TensorRT feature gate
  autoaim-app/         # implemented: Tauri desktop UI for live monitor + offline review
```

Rust runtime commands:

```bash
cargo test --workspace
cargo run -p autoaim-cli -- validate examples/sample_frames.jsonl
cargo run -p autoaim-cli -- evaluate examples/sample_frames.jsonl --json
cargo run -p autoaim-cli -- suggest examples/sample_frames.jsonl
cargo run -p autoaim-cli -- positions examples/sample_frames.jsonl
cargo run -p autoaim-cli -- positions examples/sample_frames.jsonl --assist-events
cargo run -p autoaim-cli -- run-jsonl examples/sample_frames.jsonl .e2e-output/events.jsonl
```

## Windows GUI

The current Windows GUI is a Rust + Tauri desktop app. It is intended to make
the live monitor and offline review runtime usable through a normal desktop interface.
It supports English and Chinese from the language selector in the top-right
corner; the selection is saved locally.

Run it from an extracted release zip:

```powershell
Expand-Archive .\AutoAimReview-windows-x64.zip -DestinationPath .\AutoAimReview
cd .\AutoAimReview
.\AutoAimReview.exe
```

The GUI can:

- start live monitoring from the first screen,
- use the system picker to select which screen to share,
- draw the live screen preview into the app,
- show the current mouse position in screen coordinates,
- send live frames through the Rust detector interface and draw person/head
  positions returned by the backend,
- select a frame JSONL file,
- validate dataset records,
- evaluate suggestions and show metrics,
- preview person body boxes, inferred head points, and screen-space `dx/dy`,
- preview `inference.result` events,
- write event JSONL output,
- show the review-only CUDA/TensorRT/DirectML/CPU inference config,
- check and apply incremental app updates,
- switch between English and Chinese.

Basic use:

1. Open `AutoAimReview.exe`.
2. Choose a screen from the live monitor panel.
3. Click `Start now`.
4. Pick the target screen in the system screen-sharing dialog.
5. Watch the app display the live screen, cursor coordinates, and detected
   person/head positions.

The live detector path now goes through Rust. The current release includes a
deterministic native mock detector so the app and E2E tests can validate the
full UI/backend contract without shipping large model weights. Native
ONNX/TensorRT inference and a Rust overlay renderer are still planned runtime
modules; plug the production model into the configured model path/provider
interface.

## Person Position Output

`autoaim positions` prints one JSON line per detected person. Each line includes
the body bbox, inferred head point, confidence, track id, cursor point, and
screen-space `dx/dy` from cursor to head point:

```bash
cargo run -p autoaim-cli -- positions examples/sample_frames.jsonl
```

When frame metadata has `input.mouse_down=true`, `--assist-events` additionally
emits a review-only `assist.suggestion` event. It reports the suggested head
point but does not move the system cursor or inject input:

```bash
cargo run -p autoaim-cli -- positions examples/sample_frames.jsonl --assist-events
```

The app also exposes a runtime config preview for NVIDIA CUDA or TensorRT model
execution. Actual live ONNX/TensorRT inference still requires the planned
capture/inference crates and a supplied person/head model file.

## Windows Install

Windows users should install the prebuilt setup executable. The target machine
does not need Rust, Cargo, Git, or Python.

Recommended install flow:

1. Open the latest GitHub release.
2. Download `AutoAimReviewSetup-x64.exe`.
3. Double-click it and follow the installer.
4. Launch `AutoAim Review` from the desktop or Start Menu.

Latest release:

```text
https://github.com/ccctw-ma/AutoAim/releases/latest
```

The setup installer creates a normal Windows uninstall entry, desktop shortcut,
and Start Menu shortcuts.

Portable zip fallback:

```powershell
Expand-Archive .\AutoAimReview-windows-x64.zip -DestinationPath .\AutoAimReview
cd .\AutoAimReview
.\AutoAimReview.exe
```

Scripted install fallback:

```powershell
$installer = "$env:TEMP\autoaim-install.ps1"
iwr https://raw.githubusercontent.com/ccctw-ma/AutoAim/main/windows/install.ps1 -OutFile $installer
powershell.exe -NoProfile -ExecutionPolicy Bypass -File $installer
```

The scripted installer downloads `AutoAimReview-windows-x64.zip`, verifies it with
`AutoAimReview-windows-x64-manifest.json`, installs it into
`%LOCALAPPDATA%\AutoAimReview`, and adds the `bin` directory to the user `PATH`.
It also creates a desktop shortcut, a Start Menu shortcut, and an
`autoaim-review` launcher command. Open a new terminal after installation.

Launch after installation:

```powershell
autoaim-review
```

## Incremental Update

Check for an incremental update:

```powershell
autoaim-update -CheckOnly
```

Apply the incremental update:

```powershell
autoaim-update
```

The updater reads the installed version, downloads the release delta index, and
applies only the matching old-version to new-version `.delta.json` patch after
SHA256 verification. Changed files use 64 KiB block-level binary patches, so a
small change in a large executable does not require downloading the full release
zip. It does not compile locally and does not download a full replacement zip by
default. If no matching delta exists, the updater stops and reports that a full
reinstall is required.

The Rust CLI exposes the same updater as `autoaim update --check` and
`autoaim update`.

The GUI has `Check updates` and `Apply update` buttons. `Apply update` starts
the external updater and closes the app so Windows can replace locked files.

Release builds verify generated delta packages with
`scripts/verify_delta_update.py` when a previous package and manifest are
provided to the workflow.

## Safety Rules

This project intentionally excludes:

- system mouse movement
- mouse click injection
- third-party game process control
- memory writing or process attachment
- automatic firing or aiming control

Use the generated suggestions only for overlays, reviews, metrics, and dataset
analysis.

## Python Offline Tools

```bash
python -m venv .venv
. .venv/bin/activate
pip install -e '.[dev]'
pytest
autoaim-review evaluate examples/sample_frames.jsonl
```

Without installation:

```bash
PYTHONPATH=src python -m autoaim_review.cli evaluate examples/sample_frames.jsonl
PYTHONPATH=src python -m autoaim_review.cli validate examples/sample_frames.jsonl
PYTHONPATH=src python -m autoaim_review.cli export-yolo examples/sample_frames.jsonl /tmp/autoaim-yolo-labels
PYTHONPATH=src python -m autoaim_review.cli split examples/sample_frames.jsonl
```

## Data Format

Frame records are JSONL lines matching `schemas/frame_record.schema.json`:

```json
{
  "frame_id": 10231,
  "timestamp_qpc": 123456789,
  "image": "frames/00010231.jpg",
  "session_id": "session-001",
  "scene_id": "training-map-a",
  "objects": [
    {
      "class": "person",
      "bbox": [420, 180, 120, 360],
      "head_bbox": [455, 185, 48, 52],
      "head_point": [479, 211],
      "confidence": 1.0
    }
  ],
  "input": {
    "cursor": [512, 384],
    "mouse_down": false
  }
}
```

## Dataset Hygiene

Use `session_id` or `scene_id` on every frame. Dataset splits are planned by
group, not by random frame, so adjacent frames from the same capture session do
not leak into validation/test sets.

```bash
PYTHONPATH=src python -m autoaim_review.cli validate examples/sample_frames.jsonl
PYTHONPATH=src python -m autoaim_review.cli split examples/sample_frames.jsonl
```

YOLO export writes class `0` for `person` and class `1` for `head`:

```bash
PYTHONPATH=src python -m autoaim_review.cli export-yolo examples/sample_frames.jsonl labels/
```

## Documentation

- `docs/architecture.md` describes the Rust-first Windows runtime architecture.
- `contracts/ipc.md` defines review-only IPC messages.
- `windows/README.md` outlines the Windows API boundary for the Rust runtime.
