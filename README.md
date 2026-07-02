# AutoAim Review

AutoAim Review is a visualization-only screen capture, inference review, and
dataset tooling project. It is designed for legal training/review environments:
the software may display model suggestions, log datasets, and compute metrics,
but it must not move the operating-system cursor, click, inject input, or
control third-party games.

The upstream repository snapshot used here was empty except for a README,
license, and `.gitignore`, so this commit initializes a safe MVP scaffold.

## Scope

Implemented now:

- Frame annotation data model.
- Target scoring that outputs `suggested_point` and `dx/dy`.
- JSONL dataset reader/writer.
- Evaluation CLI.
- Dataset validation CLI.
- YOLO person/head label export.
- Session/scene grouped split planning.
- JSON schema for frame records.
- IPC contract and Windows client architecture notes.

Planned Windows runtime:

```text
Windows Capture Service
 -> Frame Ring Buffer
 -> Preprocess GPU/CPU
 -> ONNX Runtime / TensorRT Inference
 -> Tracker + Target Scorer
 -> Overlay / Review UI
 -> Dataset Logger
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

## Quick Start

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

- `docs/architecture.md` describes the Windows-first architecture.
- `contracts/ipc.md` defines review-only IPC messages.
- `windows/README.md` outlines the WinUI 3 + C++/WinRT implementation boundary.
