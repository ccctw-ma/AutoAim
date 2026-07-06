# AutoAim Review Architecture

AutoAim Review is a Rust-first Windows capture and inference review
application. It intentionally does not control the system mouse or third-party
games. Runtime output is limited to overlays, suggestions, metrics, and dataset
logs.

Python remains in the repository for training, annotation conversion, dataset
validation, and offline evaluation. It is not the primary runtime.

## Runtime

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

## Workspace Layout

```text
crates/
  autoaim-core/        # DTOs, geometry, JSONL, validation, scoring
  autoaim-ipc/         # IPC event schemas and JSON line encoding
  autoaim-runtime/     # scoring pipeline and JSONL runtime event logging
  autoaim-cli/         # validate / evaluate / suggest commands
  autoaim-capture/     # planned Windows.Graphics.Capture through windows-rs
  autoaim-infer/       # planned ONNX Runtime wrapper, TensorRT feature gate
  autoaim-app/         # planned desktop control panel and overlay renderer

src/autoaim_review/    # Python offline dataset and evaluation tools
```

## Windows Client

- Language: Rust is the default for the application and runtime.
- Windows bindings: `windows` crate for Win32, WinRT, D3D11, Direct2D, and
  DirectComposition access.
- Shell: Rust desktop shell using `winit` plus `egui`/`wgpu` for MVP controls.
  A WinUI front-end can be revisited later only if native Windows UX becomes
  more important than keeping the stack Rust-first.
- Capture: `Windows.Graphics.Capture` first, `Desktop Duplication API` only
  after profiling proves the need.
- Rendering: Rust overlay renderer backed by DirectComposition/Direct2D or
  `wgpu`, depending on interop complexity.
- IPC: named pipes with JSON messages for MVP; local gRPC or Cap'n Proto can
  replace this after schemas stabilize.
- Packaging: `cargo wix`, MSIX, or a signed installer once the runtime is
  stable.

## Capture

Capture starts only after explicit user selection of a window or display.
Metadata is written per frame:

- `frame_id`
- QPC timestamp
- resolution
- selected window handle
- cursor position
- mouse button state

D3D11 textures remain in a Rust-owned ring buffer to avoid frequent CPU copies.
The data logger may sample frames to disk only when the user explicitly enables
dataset capture.

## Inference

Training and runtime are separate:

- Training: Python + PyTorch + Ultralytics YOLO or RT-DETR.
- Export: ONNX.
- Windows runtime: Rust wrapper around ONNX Runtime.
- NVIDIA low-latency path: TensorRT behind an optional Cargo feature.
- Fallback: ONNX Runtime DirectML or CPU.

Pipeline:

```text
D3D11 frame
 -> full-frame and crosshair-focused ROI selection
 -> Rust resize/letterbox
 -> Rust normalize
 -> model inference
 -> bbox geometry and pose-keypoint quality filtering
 -> person bbox / head bbox / head point
 -> Rust tracker smoothing
 -> Rust target scoring
 -> Rust overlay draw
 -> Rust metrics/logging
```

The YOLOv8 live path keeps a full-frame scan for broad awareness and a smaller
crosshair-focused zoom scan for distant targets. Candidate filtering rejects
tiny boxes, extreme aspect ratios, low-quality keypoint structures, and large
bottom-of-screen detections that are likely to be the player's own avatar.

## Output

The runtime may output:

- `person_bbox`
- `head_bbox`
- `head_point`
- `confidence`
- `suggested_point`
- `dx/dy`
- latency and FPS metrics

The runtime must not output OS input commands.

## Dataset

The JSONL frame format is defined in `schemas/frame_record.schema.json`.

Rust owns runtime dataset recording. Python tools can validate, split, export,
and evaluate those records offline.

Splits must be grouped by recording session, map, scene, or capture source.
Random frame-level splits are disallowed because adjacent frames leak nearly
identical data into validation.

Recommended tooling:

- Annotation: CVAT or Label Studio.
- Quality review: FiftyOne.
- Local index: SQLite, written from Rust or Python depending on the workflow.
- Large dataset versioning: DVC + S3/MinIO.

## MVP Milestones

1. Done: create Rust workspace and shared `autoaim-core` message/model crate.
2. Done: port target scoring from Python to Rust with parity tests.
3. Done: add Rust JSONL reader, validation, evaluation summary, IPC schemas,
   runtime event generation, and CLI commands.
4. Next: build Rust window capture viewer with frame metadata logging.
5. Next: add Rust named-pipe transport and explicit dataset logger controls.
6. Next: add Rust ONNX Runtime inference loader and overlay draw.
7. Next: add metrics panel: latency, FPS, mAP proxy metrics, head localization
   error.
8. Later: add TensorRT/FP16/GPU preprocess optimization behind optional
   features.
