# Rust Workspace

The runtime direction is Rust-first.

Current crates:

- `autoaim-core`: shared geometry, frame models, target scoring, and tracker
  primitives; includes JSONL reading, validation, suggestions, and summaries.
- `autoaim-ipc`: IPC message schemas matching `contracts/ipc.md`.
- `autoaim-runtime`: offline/runtime pipeline primitives and JSONL event logging.
- `autoaim-cli`: Rust command line entry point for validation, evaluation, and
  suggestion event output.
- `autoaim-capture`: native Windows screen and cursor capture through Win32 APIs.
- `autoaim-infer`: MoveNet and YOLOv8/YOLOv8-pose inference through Rust
  adapters, including ONNX Runtime providers and a visual fallback when no
  model file is configured.
- `autoaim-app`: Tauri desktop UI for live screen review and offline dataset
  workflows.
