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
- Rust validation, evaluation summary, and inference event CLI.
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
  autoaim-app/         # planned: desktop shell, preview, overlay controls
```

Rust runtime commands:

```bash
cargo test --workspace
cargo run -p autoaim-cli -- validate examples/sample_frames.jsonl
cargo run -p autoaim-cli -- evaluate examples/sample_frames.jsonl --json
cargo run -p autoaim-cli -- suggest examples/sample_frames.jsonl
cargo run -p autoaim-cli -- run-jsonl examples/sample_frames.jsonl .e2e-output/events.jsonl
```

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
