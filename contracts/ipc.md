# Local IPC Contract

This project is visualization-only. The worker never emits OS mouse movement,
click injection, game memory writes, or process manipulation commands.

## Processes

- `autoaim-app`: Rust desktop shell, overlay, review controls, metrics.
- `autoaim-runtime`: Rust capture, preprocess, inference, logging pipeline.
- `autoaim-review`: offline Python CLI for dataset validation and metrics.

## Transport

Use named pipes for the MVP. gRPC local can replace this when protobuf schemas
are stable.

```text
\\\\.\\pipe\\autoaim.capture.control
\\\\.\\pipe\\autoaim.inference.events
```

## Message Types

### CaptureFrameMeta

```json
{
  "type": "capture.frame",
  "frame_id": 10231,
  "timestamp_qpc": 123456789,
  "resolution": [1280, 720],
  "window_handle": "0x0000000000120A4E",
  "cursor": [512, 384],
  "mouse_down": false
}
```

Frame pixels stay in the Rust-owned D3D11 ring buffer. Metadata crosses IPC.

### InferenceResult

```json
{
  "type": "inference.result",
  "frame_id": 10231,
  "latency_ms": 8.3,
  "objects": [
    {
      "class": "person",
      "bbox": [420, 180, 120, 360],
      "head_bbox": [455, 185, 48, 52],
      "head_point": [479, 211],
      "confidence": 0.91,
      "track_id": 7
    }
  ],
  "suggestion": {
    "suggested_point": [479, 211],
    "dx": -33,
    "dy": -173,
    "score": 0.82
  }
}
```

### DatasetRecord

Dataset records are JSONL lines matching `schemas/frame_record.schema.json`.
They may be written alongside extracted frames or exported to COCO/YOLO later.

## Forbidden IPC

The following commands are intentionally not part of the protocol:

- `move_mouse`
- `click_mouse`
- `inject_input`
- `attach_process`
- `write_process_memory`
- any command that targets a third-party game's control path
