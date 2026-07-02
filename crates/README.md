# Rust Workspace

The runtime direction is Rust-first.

Current crates:

- `autoaim-core`: shared geometry, frame models, target scoring, and tracker
  primitives; includes JSONL reading, validation, suggestions, and summaries.
- `autoaim-ipc`: IPC message schemas matching `contracts/ipc.md`.
- `autoaim-runtime`: offline/runtime pipeline primitives and JSONL event logging.
- `autoaim-cli`: Rust command line entry point for validation, evaluation, and
  suggestion event output.
- `autoaim-app`: Tauri desktop UI for the offline review workflow.
- `autoaim-capture`: Windows capture through `windows-rs`.
- `autoaim-infer`: ONNX Runtime and optional TensorRT integration.

`autoaim-capture` and `autoaim-infer` are the next runtime crates to implement.
