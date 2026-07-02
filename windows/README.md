# Windows Runtime Scaffold

This folder documents the Windows-specific boundary. The runtime direction is
Rust-first. Python remains available for offline dataset tooling, but capture,
inference, overlay, IPC, and runtime logging should be implemented in Rust.

## Planned Rust Workspace

```text
crates/
  autoaim-core/        # implemented: structs, geometry, JSONL, scoring
  autoaim-ipc/         # implemented: JSON IPC event schemas
  autoaim-runtime/     # implemented: frame -> inference event pipeline
  autoaim-cli/         # implemented: validate / evaluate / suggest commands
  autoaim-capture/     # planned: Windows.Graphics.Capture through windows-rs
  autoaim-infer/       # planned: ONNX Runtime wrapper, TensorRT feature gate
  autoaim-app/         # planned: desktop shell, preview, overlay controls

windows/
  README.md            # Windows-specific implementation notes
```

## Windows API Boundary

- Use the `windows` crate for Win32, WinRT, D3D11, Direct2D, and
  DirectComposition calls.
- Use `Windows.Graphics.Capture` for explicit user-selected window or display
  capture.
- Keep frame pixels in a D3D11 texture ring buffer owned by Rust.
- Use named pipes for MVP IPC with the JSON contracts in `contracts/ipc.md`.
- Keep TensorRT behind an optional Cargo feature so the default build can run
  through ONNX Runtime DirectML or CPU.

## Safety Boundary

The Rust runtime may read selected-window pixels through
`Windows.Graphics.Capture` and emit overlay metadata. It must not call
`SendInput`, install hooks for third-party games, write process memory, attach
to game processes, or move the system cursor.

## Implemented Runtime Foundation

1. Rust workspace with `autoaim-core`, `autoaim-ipc`, `autoaim-runtime`, and
   `autoaim-cli`.
2. Rust JSONL frame model compatible with the current schema.
3. Rust target scoring and inference event generation.
4. Rust CLI commands for validation, evaluation, and suggestion event output.

## Next Windows Implementation Tasks

1. Build a Rust window picker and preview surface.
2. Capture frames into a D3D11 texture ring buffer.
3. Emit `CaptureFrameMeta` over the named pipe contract.
4. Render the latest frame and draw review-only boxes/points from
   `InferenceResult`.
5. Allow dataset logging only after explicit opt-in.
