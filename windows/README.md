# Windows Client Scaffold

This folder documents the planned Windows implementation boundary. The current
repository is initialized from an empty upstream snapshot, so the verified MVP
code lives in the Python package first.

## Planned Solution Layout

```text
windows/
  AutoAim.Review.sln
  src/
    AutoAim.Review.App/          # WinUI 3 shell and overlay
    AutoAim.Worker/              # C++/WinRT capture + ONNX/TensorRT worker
    AutoAim.Contracts/           # shared DTOs / generated IPC bindings
```

## Safety Boundary

The Windows worker may read selected-window pixels through
`Windows.Graphics.Capture` and emit overlay metadata. It must not call
`SendInput`, install hooks for third-party games, write process memory, or move
the system cursor.

## First Implementation Task

Create a WinUI 3 window picker and preview surface:

1. Ask the user to select a window/display with `GraphicsCapturePicker`.
2. Capture frames into a D3D11 texture ring buffer.
3. Send `CaptureFrameMeta` messages over the named pipe contract.
4. Render the latest frame and draw review-only boxes/points from
   `InferenceResult`.
5. Allow dataset logging only after explicit opt-in.
