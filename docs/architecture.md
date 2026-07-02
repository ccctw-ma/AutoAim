# AutoAim Review Architecture

AutoAim Review is a Windows-first capture and inference review application.
It intentionally does not control the system mouse or third-party games. The
runtime output is limited to overlays, suggestions, metrics, and dataset logs.

## Runtime

```text
Windows Capture Service
 -> Frame Ring Buffer
 -> Preprocess GPU/CPU
 -> ONNX Runtime / TensorRT Inference
 -> Tracker + Target Scorer
 -> Overlay / Review UI
 -> Dataset Logger
```

## Windows Client

- Shell: WinUI 3 + .NET.
- Native worker: C++/WinRT + Direct3D 11.
- Capture: `Windows.Graphics.Capture` first, `Desktop Duplication API` only
  after profiling proves the need.
- IPC: named pipes for MVP, local gRPC after schemas stabilize.
- Rendering: WinUI overlay with Direct2D/DirectComposition.
- Packaging: MSIX with code signing for distribution.

## Capture

Capture starts only after explicit user selection of a window or display.
Metadata is written per frame:

- `frame_id`
- QPC timestamp
- resolution
- selected window handle
- cursor position
- mouse button state

D3D11 textures remain in a ring buffer to avoid frequent CPU copies. The data
logger may sample frames to disk only when the user explicitly enables dataset
capture.

## Inference

Training and runtime are separate:

- Training: Python + PyTorch + Ultralytics YOLO or RT-DETR.
- Export: ONNX.
- Windows runtime: ONNX Runtime.
- NVIDIA low-latency path: TensorRT.
- Fallback: ONNX Runtime DirectML or CPU.

Pipeline:

```text
D3D11 frame
 -> resize/letterbox
 -> normalize
 -> model inference
 -> person bbox / head bbox / head point
 -> tracker smoothing
 -> target scoring
 -> overlay draw
 -> metrics/logging
```

## Output

The worker may output:

- `person_bbox`
- `head_bbox`
- `head_point`
- `confidence`
- `suggested_point`
- `dx/dy`
- latency and FPS metrics

The worker must not output OS input commands.

## Dataset

The JSONL frame format is defined in `schemas/frame_record.schema.json`.

Splits must be grouped by recording session, map, scene, or capture source.
Random frame-level splits are disallowed because adjacent frames leak nearly
identical data into validation.

Recommended tooling:

- Annotation: CVAT or Label Studio.
- Quality review: FiftyOne.
- Local index: SQLite.
- Large dataset versioning: DVC + S3/MinIO.

## MVP Milestones

1. Window capture viewer with frame metadata logging.
2. JSONL dataset logger and schema validation.
3. CVAT/COCO/YOLO export and import scripts.
4. ONNX inference loader and overlay draw.
5. Metrics panel: latency, FPS, mAP proxy metrics, head localization error.
6. TensorRT/FP16/GPU preprocess optimization.
