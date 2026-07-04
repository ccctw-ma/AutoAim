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
  autoaim-app/         # implemented: Rust + Tauri desktop UI
  autoaim-capture/     # planned: Windows.Graphics.Capture through windows-rs
  autoaim-infer/       # planned: ONNX Runtime wrapper, TensorRT feature gate

windows/
  README.md            # Windows-specific implementation notes
  installer/           # implemented: Inno Setup installer definition
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
`SendInput`, install hooks for third-party games, write process memory, or
attach to game processes. The only cursor-control path is the explicit
`Auto move mouse` toggle, which uses `SetCursorPos` while the configured
activation key is held.

## Implemented Runtime Foundation

1. Rust workspace with `autoaim-core`, `autoaim-ipc`, `autoaim-runtime`, and
   `autoaim-cli`.
2. Rust JSONL frame model compatible with the current schema.
3. Rust target scoring and inference event generation.
4. Rust CLI commands for validation, evaluation, and suggestion event output.
5. Rust + Tauri GUI with a live monitor first screen, screen selection, live
   screen preview, real-time cursor coordinates, browser-side person detection,
   offline JSONL tools, update buttons, and English/Chinese language switching.

The current GUI can show a live screen preview through the system screen picker
and send frame metadata through the Rust detector interface. This release uses a
deterministic Rust mock detector for E2E validation and wiring. Native
ONNX/TensorRT inference and overlay rendering are still planned runtime crates.

## Release Zip GUI Entry

After extracting `AutoAimReview-windows-x64.zip`, run:

```powershell
.\AutoAimReview.exe
```

The package contains:

- `AutoAimReview.exe`: Tauri GUI application.
- `bin/autoaim.exe`: Rust CLI used by the GUI.
- `assets/logo.svg` and generated `assets/logo.ico`: package and shortcut
  branding.
- `examples/sample_frames.jsonl`: bundled sample input for first launch.

On first launch, click `Start now`, choose the screen in the system picker, and
the app displays the live preview, mouse coordinates, and detected person
positions. If `Auto move mouse` is enabled, the app moves the system cursor to
the selected person/head point while the activation key is held.

## Setup Installer

The preferred install artifact is:

```text
AutoAimReviewSetup-x64.exe
```

It is built by GitHub Actions with Inno Setup from
`windows/installer/AutoAimReview.iss`. The installer provides a normal Windows
setup wizard, Start Menu shortcuts, optional desktop shortcut, and an uninstall
entry in Windows Settings.

The setup installer installs the same files as the portable zip package:

- `AutoAimReview.exe`: Rust + Tauri GUI application.
- `bin/autoaim.exe`: Rust CLI used by the GUI.
- `assets/logo.svg` and generated `assets/logo.ico`: package and shortcut
  branding.
- `examples/sample_frames.jsonl`: bundled sample input for first launch.

## Portable Zip and Scripted Install

This folder also includes a script-based installer/updater for the current
runtime scaffold:

- `install.ps1` downloads the prebuilt `AutoAimReview-windows-x64.zip` release
  asset, verifies it against `AutoAimReview-windows-x64-manifest.json`, installs
  it into `%LOCALAPPDATA%\AutoAimReview`, adds the `bin` directory to the user
  `PATH`, and creates desktop plus Start Menu shortcuts.
- `update.ps1` reads the installed version, downloads
  `AutoAimReview-windows-x64-deltas.json`, finds the matching old-version to
  new-version delta, verifies SHA256 hashes, and applies 64 KiB block-level
  binary patches for changed files.
- `autoaim-update.cmd` is generated during installation as the normal user
  entry point for update checks.
- `autoaim-review.cmd` is generated during installation as the normal user entry
  point for the GUI.

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

Launch the GUI after installation:

```powershell
autoaim-review
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
- `-NoDesktopShortcut` skips the desktop shortcut.
- `-NoStartMenuShortcut` skips the Start Menu shortcut.
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

- `AutoAimReviewSetup-x64.exe`
- `AutoAimReview-windows-x64.zip`
- `AutoAimReview-windows-x64-manifest.json`
- `AutoAimReview-windows-x64-deltas.json`
- optional `AutoAimReview-windows-x64-<old>-to-<new>.delta.json`

When previous release assets are supplied, the workflow verifies the generated
delta with `scripts/verify_delta_update.py` before publishing.

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
