# Rust Workspace

The runtime direction is Rust-first.

Current crates:

- `autoaim-core`: shared geometry, frame models, target scoring, and tracker
  primitives; includes JSONL reading, validation, suggestions, and summaries.
- `autoaim-ipc`: IPC message schemas matching `contracts/ipc.md`.
- `autoaim-runtime`: offline/runtime pipeline primitives and JSONL event logging.
- `autoaim-cli`: Rust command line entry point for validation, evaluation, and
  suggestion event output.
- `autoaim-capture`: Windows capture through `windows-rs`.
- `autoaim-infer`: ONNX Runtime and optional TensorRT integration.
- `autoaim-app`: desktop shell, preview, overlay, and review controls.

`autoaim-capture`, `autoaim-infer`, and `autoaim-app` are the next runtime
crates to implement.
