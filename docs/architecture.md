# AutoAim Review Architecture

English version. For the Chinese version, see
[`architecture.zh.md`](architecture.zh.md).

AutoAim Review is a Rust-first Windows screen capture, inference review, and
dataset tooling application. It is built for controlled review and training
workflows where every runtime side effect is explicit and observable. Python
stays in the repository for offline data work; the live runtime is Rust.

## Design Goals

- Keep the live loop low latency enough for 60 FPS UI feedback.
- Keep capture, inference, tracking, overlay, telemetry, and dataset recording
  separated by explicit ownership boundaries.
- Avoid hidden side effects: capture starts after user selection, dataset
  recording starts after user action, and cursor movement requires both the
  `Auto move mouse` toggle and a held activation key.
- Prefer cached, long-lived native resources over per-frame rebuilding.
- Make every performance-sensitive step measurable through logs and UI latency
  counters.

## Non-Goals And Safety Boundary

The application may:

- capture pixels from a user-selected screen,
- run native person or pose inference,
- draw overlays and previews,
- log frame metadata, inference outputs, and latency,
- optionally send bounded relative cursor movement while the activation key is
  held.

The application must not:

- click or fire automatically,
- install keyboard or mouse hooks into third-party processes,
- modify process memory,
- inspect or alter game network traffic,
- rebuild GPU/capture sessions every frame,
- block the UI thread on capture, inference, disk IO, or telemetry.

## Workspace Layout

```text
crates/
  autoaim-core/        # shared DTOs, geometry, JSONL, validation, scoring
  autoaim-ipc/         # JSON IPC event schemas and JSON line encoding
  autoaim-runtime/     # frame -> inference event pipeline and event logging
  autoaim-cli/         # validate / evaluate / suggest / replay commands
  autoaim-capture/     # native Windows screen and cursor capture
  autoaim-infer/       # MoveNet and YOLOv8/YOLOv8-pose inference adapters
  autoaim-app/         # Tauri desktop UI, live monitor, overlay, updater

src/autoaim_review/    # Python offline dataset and evaluation tools
schemas/               # JSON schemas for runtime and dataset files
contracts/             # IPC contracts
windows/               # Windows installer and updater scripts
docs/                  # architecture documentation
```

## Runtime Topology

```text
Tauri UI
 -> live monitor command
 -> capture worker with cached ScreenCapturer
 -> inference worker with cached NativePersonDetector
 -> object tracker and head prediction
 -> optional bounded relative mouse move
 -> overlay window event
 -> UI preview and telemetry update
 -> optional dataset recorder
```

The live monitor command is guarded by an in-flight flag. If a previous snapshot
is still running, the next request returns `live snapshot busy` instead of
queuing more GPU or capture work. This prevents slow inference frames from
building an unbounded backlog.

## Threading Model

- UI thread: Tauri window, command dispatch, and webview event handling.
- Snapshot worker: `spawn_blocking` path that runs capture, inference, tracking,
  target selection, and snapshot construction.
- Capture worker: `ScreenCapturer` owns the native capture session and services
  frame requests without rebuilding the session.
- Detector cache: `LiveDetectorState` keeps a detector for the active runtime
  config and rebuilds only when provider, model path, or threshold changes.
- Dataset writer: recording writes large RGBA frames off the hot path.
- Telemetry loop: CPU/GPU telemetry is sampled at low frequency and cached.
- Overlay window: browser-side canvas rendering uses `requestAnimationFrame`
  batching and separates high-frequency cursor drawing from lower-frequency pose
  drawing.

## Capture Pipeline

The capture crate provides native screen and cursor capture. The important
runtime rules are:

- Use a long-lived `ScreenCapturer`; do not recreate a DXGI/session object every
  frame.
- On `WouldBlock`, return a cached frame when possible instead of blocking the
  live loop.
- Keep the inference frame capped at the live capture maximum, currently
  1920x1080.
- Keep the UI preview capped separately, currently 640x360, so IPC and base64
  preview transfer do not dominate frame time.
- Record full RGBA datasets only when explicitly enabled, and write them from a
  background path to avoid disk IO spikes.

Each captured frame carries:

- screen origin and screen size,
- frame size after capture scaling,
- capture backend name,
- RGBA pixels,
- cursor position,
- cursor-on-screen flag,
- timestamp in milliseconds.

## Inference Providers

Training and runtime are intentionally separate:

- Training: Python, PyTorch, Ultralytics YOLO, or RT-DETR.
- Export: ONNX.
- Runtime: Rust inference adapters.
- Provider options: CPU, DirectML, CUDA, and TensorRT where available.
- Fallback: deterministic visual candidates when no model is configured.

The runtime chooses the YOLO path when the model path looks like a YOLO model.
YOLOv8 detect models produce person boxes. YOLOv8-pose models produce person
boxes plus keypoints. MoveNet remains supported for pose models, but YOLOv8-pose
is the preferred path for the current game-like datasets because it handles
small and partially visible targets better.

## YOLOv8 Scan Strategy

YOLOv8 preprocessing performs letterbox resize and maps model coordinates back
to screen space through `FrameToModelTransform`.

The live scan currently uses:

- a full-frame region for broad awareness,
- a smaller crosshair-focused zoom region for distant or scoped targets,
- a rotating grid scan every few frames or when the primary scan finds nothing.

This gives the detector a higher effective pixel density near the aiming area
without paying for a full multi-scale grid every frame.

## Candidate Filtering

YOLO raw outputs are filtered in multiple layers before a person reaches the
tracker:

- confidence threshold from live config,
- pre-NMS candidate limit,
- bbox minimum side and area,
- bbox aspect ratio guard,
- bottom-of-screen own-avatar rejection,
- NMS by IoU,
- pose keypoint score threshold,
- minimum visible keypoint count,
- visible keypoint average score,
- required body anchors or face-plus-shoulder structure,
- keypoints must land inside an expanded bbox,
- visible keypoints must have a minimum spatial span,
- head point must remain in the upper portion of the body box.

These filters are deliberately conservative. They reduce false positives from
background UI, weapon or character HUD elements, and face-only texture matches.

## Tracking And Prediction

`autoaim-core::ObjectTracker` assigns track ids to person objects using IoU and
aim-point distance. The live app then keeps a per-screen head prediction map:

- prediction horizon: 120 ms,
- velocity smoothing: exponential moving average,
- maximum accepted tracking delta: 250 ms,
- maximum head speed clamp: 5000 px/s.

Prediction is only used when the user enables prediction. It affects overlay
display and optional auto-mouse target selection; raw detection output remains
available for review.

## Target Selection

Auto-mouse target selection scores people by:

- confidence,
- distance from the active aim anchor,
- optional predicted head point when prediction is enabled.

The anchor is the cursor position when the cursor is on the selected screen.
Otherwise it falls back to the center of the selected screen. This keeps target
selection deterministic even when cursor capture is temporarily unavailable.

## Bounded Cursor Movement

The cursor path is Windows-only and guarded by user controls:

- the `Auto move mouse` UI toggle must be enabled,
- the configured activation key must be held,
- the target class must be `person`,
- the required relative move must exceed the small dead zone.

The runtime sends relative `SendInput` movement, not absolute cursor warps. The
movement calculation is bounded by:

- base relative gain,
- distance-based gain that slows near the target,
- target-width gain that moves smaller/farther targets more aggressively,
- maximum relative step,
- minimum rounding threshold.

The runtime does not click. It also logs target bbox, target score, aim delta,
and input delta periodically so tuning can be done from logs.

## Overlay And UI

The UI has two visual surfaces:

- Main Tauri window: controls, preview frame, telemetry, latency, model status,
  and dataset controls.
- Overlay window: transparent click-through drawing layer for live people,
  skeletons, head point, prediction, and cursor crosshair.

Overlay rendering is intentionally batched. Pose drawings are updated with
snapshot events, while cursor drawings can update more often. The canvas is
split so high-frequency cursor refresh does not force the full pose layer to
clear and redraw.

## Dataset Recording

Runtime dataset records are written under the user's local app data directory.
The Windows log path is:

```text
%LOCALAPPDATA%\AutoAimReview\logs\autoaim-review.log
```

Dataset records include:

- schema version,
- sequence id,
- timestamp,
- screen id,
- frame file path,
- frame and screen dimensions,
- capture backend,
- cursor position,
- provider and model status,
- capture, detect, tracking, and total latency,
- people with bbox, head point, predicted head point, keypoints, confidence, and
  track id.

Large RGBA frame writes are moved off the live path because a 1080p RGBA frame
is roughly 8 MB and synchronous writes can produce visible frame-time spikes.

## Offline Tooling

Rust owns runtime recording and event generation. Python remains useful for:

- annotation conversion,
- dataset validation,
- split planning,
- offline evaluation,
- model training.

Splits must be grouped by recording session, map, scene, or capture source.
Random frame-level splits are disallowed because adjacent frames leak nearly
identical data into validation.

Recommended tooling:

- Annotation: CVAT or Label Studio.
- Quality review: FiftyOne.
- Local index: SQLite.
- Large dataset versioning: DVC with S3 or MinIO.

## Packaging And Update

Windows packaging lives under `windows/`:

- Inno Setup creates the setup executable.
- `install.ps1` installs the release zip into `%LOCALAPPDATA%\AutoAimReview`.
- `update.ps1` applies manifest-verified incremental block updates.
- The updater must start child processes with hidden windows on Windows to avoid
  terminal flicker.

The installed app should be runnable without Rust, Cargo, Git, or Python.

## Configuration Surface

The current user-facing runtime configuration includes:

- screen selection,
- provider selection,
- model path,
- confidence threshold,
- activation key,
- prediction overlay toggle,
- auto-mouse toggle,
- preview frame inclusion,
- live dataset recording.

Activation keys include Alt variants, right mouse, Mouse4, Mouse5, and always-on
mode. Mouse side buttons are useful when games intercept normal keyboard
modifiers.

## Performance Budget

The live path should optimize for:

- stable 16 ms UI polling cadence where feasible,
- no per-frame capture session rebuild,
- no unbounded inference queue,
- low-frequency telemetry sampling,
- throttled preview transfer,
- background dataset writes,
- batched overlay rendering.

Slow frames are logged. The expected tuning workflow is to record a small live
dataset, replay it offline, inspect p95 latency and failure cases, and only then
change thresholds, scan regions, or model choices.

## Failure Modes

Known failure classes and mitigations:

- Capture `WouldBlock`: return cached frame or rebuild the capturer only after a
  hard capture error.
- GPU/provider hang: in-flight guard prevents request pile-up.
- Model false positive: bbox geometry, keypoint quality, and own-avatar filters.
- UI preview overload: downsample preview and disable frame transfer when not
  needed.
- Dataset IO spike: write frames off the live path.
- Terminal flicker on Windows: hidden process creation for telemetry and update
  helpers.
- Activation key eaten by game: support Mouse4/Mouse5 and always-on mode.

## Testing

Useful local checks:

```bash
cargo fmt
cargo test -p autoaim-infer
cargo test -p autoaim-app
cargo test --workspace
```

Inference-specific tests cover:

- YOLO output layout detection,
- detect and pose decoding,
- low-quality keypoint rejection,
- face-only false positive rejection,
- own-avatar bottom-box rejection,
- scan-region mapping back to screen space,
- merge and NMS behavior.

## Roadmap

- Keep improving dataset-driven YOLOv8-pose thresholds with replay statistics.
- Add more explicit per-provider latency reporting.
- Add a small target-review panel for rejected candidates and rejection reasons.
- Add configurable scan presets for full screen, scoped center, and low-latency
  modes.
- Add package signing when release flow stabilizes.
- Keep all driver-level or hardware-HID mouse paths out of the default runtime;
  if ever needed, they must remain optional and explicitly documented.
