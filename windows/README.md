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

## Install and Incremental Update Scripts

This folder includes a binary Windows installer/updater for the current runtime
scaffold:

- `install.ps1` downloads the prebuilt `AutoAimReview-windows-x64.zip` release
  asset, verifies it against `AutoAimReview-windows-x64-manifest.json`, installs
  it into `%LOCALAPPDATA%\AutoAimReview`, and adds the `bin` directory to the
  user `PATH`.
- `update.ps1` reads the installed version, downloads
  `AutoAimReview-windows-x64-deltas.json`, finds the matching old-version to
  new-version delta, verifies SHA256 hashes, and applies 64 KiB block-level
  binary patches for changed files.
- `autoaim-update.cmd` is generated during installation as the normal user
  entry point for update checks.

The target Windows machine does not need Rust, Cargo, or Git.

Install directly from the latest release:

```powershell
$installer = "$env:TEMP\autoaim-install.ps1"
iwr https://raw.githubusercontent.com/ccctw-ma/AutoAim/main/windows/install.ps1 -OutFile $installer
powershell.exe -NoProfile -ExecutionPolicy Bypass -File $installer
```

Install a specific version:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File $installer -Version v0.1.0
```

Check for an incremental update:

```powershell
autoaim-update -CheckOnly
```

Apply the incremental update:

```powershell
autoaim-update
```

The Rust CLI wrapper supports the same flow:

```powershell
autoaim update --check
autoaim update
```

Useful flags:

- `-InstallDir <path>` installs somewhere other than
  `%LOCALAPPDATA%\AutoAimReview`.
- `-Repo <owner/name>` installs or updates from another GitHub repository.
- `-NoPathUpdate` leaves the user `PATH` unchanged during install.
- `-TargetVersion <tag>` updates to a specific release tag instead of latest.

The updater intentionally does not download and overwrite the latest zip by
default. If no matching delta exists, it stops and tells the user to reinstall
from a full package. This is a real incremental-update path: changed files are
patched by block offsets, while added files are embedded in the delta and removed
files are deleted. A signed MSI/MSIX package can be added later after
`autoaim-app`, capture, preview, and overlay controls exist.

## Release Packaging

GitHub Actions workflow `.github/workflows/windows-release.yml` builds the
Windows package on `windows-latest` and uploads:

- `AutoAimReview-windows-x64.zip`
- `AutoAimReview-windows-x64-manifest.json`
- `AutoAimReview-windows-x64-deltas.json`
- optional `AutoAimReview-windows-x64-<old>-to-<new>.delta.json`

For a release with an incremental update, run the workflow manually and provide
the previous release package URL plus previous manifest URL. The package builder
uses `scripts/build_windows_package.py` to create the new package and the delta
asset.

## Next Windows Implementation Tasks

1. Build a Rust window picker and preview surface.
2. Capture frames into a D3D11 texture ring buffer.
3. Emit `CaptureFrameMeta` over the named pipe contract.
4. Render the latest frame and draw review-only boxes/points from
   `InferenceResult`.
5. Allow dataset logging only after explicit opt-in.
