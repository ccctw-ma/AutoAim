from __future__ import annotations

import argparse
import base64
import hashlib
import json
import os
import shutil
import struct
import subprocess
from dataclasses import dataclass
from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile


PACKAGE_NAME = "AutoAimReview-windows-x64"
PATCH_BLOCK_SIZE = 64 * 1024


@dataclass(frozen=True)
class FileRecord:
    path: str
    size: int
    sha256: str


def sha256_file(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as file:
        for chunk in iter(lambda: file.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def run(command: list[str], cwd: Path) -> None:
    subprocess.run(command, cwd=cwd, check=True)


def copy_file(source: Path, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)


def copy_required_model(repo: Path, name: str, destination: Path) -> None:
    source = repo / "models" / name
    if not source.is_file():
        raise FileNotFoundError(
            f"missing model file: {source}. Run `python scripts/prepare_models.py` "
            "or copy the model into models/ before building a Windows package."
        )
    copy_file(source, destination)


def write_logo_ico(path: Path, size: int = 64) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    pixels: list[tuple[int, int, int, int]] = []
    center = (size - 1) / 2.0

    for y in range(size):
        for x in range(size):
            dx = x - center
            dy = y - center
            radius = (dx * dx + dy * dy) ** 0.5
            r, g, b, a = 11, 16, 32, 255

            if 20 <= radius <= 24:
                r, g, b = 34, 211, 238
            elif 30 <= radius <= 32:
                r, g, b = 31, 41, 55

            if abs(dx) <= 2 and (radius < 12 or radius > 25):
                r, g, b = 34, 211, 238
            if abs(dy) <= 2 and (radius < 12 or radius > 25):
                r, g, b = 34, 211, 238

            left_stem = abs(x - (22 + y * 0.22)) <= 2 and 18 <= y <= 47
            right_stem = abs(x - (42 - y * 0.22)) <= 2 and 18 <= y <= 47
            crossbar = 35 <= y <= 39 and 26 <= x <= 38
            if left_stem or right_stem or crossbar:
                r, g, b = 248, 250, 252

            if radius <= 3:
                r, g, b = 249, 115, 22

            pixels.append((b, g, r, a))

    xor_bitmap = bytearray()
    for y in reversed(range(size)):
        row_start = y * size
        for pixel in pixels[row_start : row_start + size]:
            xor_bitmap.extend(pixel)

    and_mask = bytes((size // 8) * size)
    bitmap_info_header = struct.pack(
        "<IIIHHIIIIII",
        40,
        size,
        size * 2,
        1,
        32,
        0,
        len(xor_bitmap),
        0,
        0,
        0,
        0,
    )
    image = bitmap_info_header + bytes(xor_bitmap) + and_mask
    header = struct.pack("<HHH", 0, 1, 1)
    directory = struct.pack("<BBBBHHII", size, size, 0, 0, 1, 32, len(image), 22)
    path.write_bytes(header + directory + image)


def collect_files(root: Path) -> list[FileRecord]:
    records: list[FileRecord] = []
    for path in sorted(item for item in root.rglob("*") if item.is_file()):
        relative = path.relative_to(root).as_posix()
        records.append(
            FileRecord(path=relative, size=path.stat().st_size, sha256=sha256_file(path))
        )
    return records


def write_zip(package_root: Path, output_path: Path) -> None:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with ZipFile(output_path, "w", ZIP_DEFLATED) as archive:
        for path in sorted(item for item in package_root.rglob("*") if item.is_file()):
            archive.write(path, path.relative_to(package_root).as_posix())


def read_manifest(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def find_package_root(extract_dir: Path) -> Path:
    children = list(extract_dir.iterdir())
    if len(children) == 1 and children[0].is_dir():
        return children[0]
    return extract_dir


def extract_zip(path: Path, output_dir: Path) -> Path:
    if output_dir.exists():
        shutil.rmtree(output_dir)
    output_dir.mkdir(parents=True)
    with ZipFile(path) as archive:
        archive.extractall(output_dir)
    return find_package_root(output_dir)


def load_records(root: Path, manifest: dict) -> dict[str, FileRecord]:
    records: dict[str, FileRecord] = {}
    for file in manifest["files"]:
        path = root / file["path"]
        records[file["path"]] = FileRecord(
            path=file["path"], size=path.stat().st_size, sha256=sha256_file(path)
        )
    return records


def base64_file(path: Path) -> str:
    return base64.b64encode(path.read_bytes()).decode("ascii")


def build_binary_patch(old_path: Path, new_path: Path) -> dict:
    blocks: list[dict] = []
    offset = 0
    with old_path.open("rb") as old_file, new_path.open("rb") as new_file:
        while True:
            old_block = old_file.read(PATCH_BLOCK_SIZE)
            new_block = new_file.read(PATCH_BLOCK_SIZE)
            if not old_block and not new_block:
                break
            if old_block != new_block:
                blocks.append(
                    {
                        "offset": offset,
                        "data_base64": base64.b64encode(new_block).decode("ascii"),
                    }
                )
            offset += PATCH_BLOCK_SIZE

    return {
        "block_size": PATCH_BLOCK_SIZE,
        "blocks": blocks,
    }


def build_delta(
    old_package: Path,
    old_manifest_path: Path,
    new_package_root: Path,
    new_manifest: dict,
    output_dir: Path,
) -> tuple[dict, Path]:
    old_extract = output_dir / "_old-package"
    old_root = extract_zip(old_package, old_extract)
    old_manifest = read_manifest(old_manifest_path)
    old_records = load_records(old_root, old_manifest)
    new_records = {record.path: record for record in collect_files(new_package_root)}

    changes: list[dict] = []
    for path in sorted(set(old_records) | set(new_records)):
        old = old_records.get(path)
        new = new_records.get(path)
        if old is None and new is not None:
            changes.append(
                {
                    "action": "add",
                    "path": path,
                    "to_sha256": new.sha256,
                    "to_size": new.size,
                    "content_base64": base64_file(new_package_root / path),
                }
            )
        elif old is not None and new is None:
            changes.append(
                {
                    "action": "remove",
                    "path": path,
                    "from_sha256": old.sha256,
                    "from_size": old.size,
                }
            )
        elif old is not None and new is not None and old.sha256 != new.sha256:
            patch = build_binary_patch(old_root / path, new_package_root / path)
            changes.append(
                {
                    "action": "patch",
                    "path": path,
                    "from_sha256": old.sha256,
                    "from_size": old.size,
                    "to_sha256": new.sha256,
                    "to_size": new.size,
                    "patch": patch,
                }
            )

    delta_name = f"{PACKAGE_NAME}-{old_manifest['version']}-to-{new_manifest['version']}.delta.json"
    delta_path = output_dir / delta_name
    delta_payload = {
        "format": "autoaim.windows.delta.v1",
        "from_version": old_manifest["version"],
        "to_version": new_manifest["version"],
        "files": changes,
        "summary": {
            "changed_files": sum(1 for change in changes if change["action"] == "patch"),
            "added_files": sum(1 for change in changes if change["action"] == "add"),
            "removed_files": sum(1 for change in changes if change["action"] == "remove"),
            "changed_blocks": sum(
                len(change.get("patch", {}).get("blocks", [])) for change in changes
            ),
            "patch_bytes": 0,
        },
    }
    delta_path.write_text(json.dumps(delta_payload, indent=2, sort_keys=True), encoding="utf-8")
    delta_payload["summary"]["patch_bytes"] = delta_path.stat().st_size
    delta_path.write_text(json.dumps(delta_payload, indent=2, sort_keys=True), encoding="utf-8")

    delta_entry = {
        "from_version": old_manifest["version"],
        "to_version": new_manifest["version"],
        "asset": delta_name,
        "sha256": sha256_file(delta_path),
        "summary": delta_payload["summary"],
        "files": [
            {"action": change["action"], "path": change["path"]} for change in changes
        ],
    }
    return delta_entry, delta_path


def build_package(args: argparse.Namespace) -> None:
    repo = Path(args.repo).resolve()
    output_dir = Path(args.output_dir).resolve()
    package_root = output_dir / "package-root"
    bin_dir = package_root / "bin"
    windows_dir = package_root / "windows"
    assets_dir = package_root / "assets"
    examples_dir = package_root / "examples"
    models_dir = package_root / "models"

    if package_root.exists():
        shutil.rmtree(package_root)
    output_dir.mkdir(parents=True, exist_ok=True)
    bin_dir.mkdir(parents=True)
    windows_dir.mkdir(parents=True)
    assets_dir.mkdir(parents=True)
    examples_dir.mkdir(parents=True)
    models_dir.mkdir(parents=True)

    if args.cli_exe_path or args.exe_path:
        cli_source = Path(args.cli_exe_path or args.exe_path).resolve()
    elif not args.skip_build:
        run(["cargo", "build", "--release", "-p", "autoaim-cli", "-p", "autoaim-app"], cwd=repo)
        cli_source = repo / "target" / "release" / "autoaim.exe"
    else:
        cli_source = repo / "target" / "release" / "autoaim.exe"

    if args.app_exe_path:
        app_source = Path(args.app_exe_path).resolve()
    elif not args.skip_build:
        app_source = repo / "target" / "release" / "AutoAimReview.exe"
    else:
        app_source = repo / "target" / "release" / "AutoAimReview.exe"

    if not cli_source.exists():
        raise FileNotFoundError(f"missing built CLI executable: {cli_source}")
    if not app_source.exists():
        raise FileNotFoundError(f"missing built app executable: {app_source}")

    copy_file(app_source, package_root / "AutoAimReview.exe")
    copy_file(cli_source, bin_dir / "autoaim.exe")
    for dll_source in sorted(app_source.parent.glob("*.dll")):
        if dll_source.name.lower().startswith(("directml", "onnxruntime")):
            copy_file(dll_source, package_root / dll_source.name)
    copy_file(repo / "windows" / "install.ps1", windows_dir / "install.ps1")
    copy_file(repo / "windows" / "update.ps1", windows_dir / "update.ps1")
    copy_file(repo / "assets" / "logo.svg", assets_dir / "logo.svg")
    write_logo_ico(assets_dir / "logo.ico")
    copy_required_model(repo, "yolov8n-pose.onnx", models_dir / "yolov8n-pose.onnx")
    copy_required_model(repo, "yolov8n.onnx", models_dir / "yolov8n.onnx")
    copy_required_model(repo, "movenet_lightning.tflite", models_dir / "movenet_lightning.tflite")
    copy_required_model(repo, "movenet_lightning.onnx", models_dir / "movenet_lightning.onnx")
    copy_file(repo / "examples" / "sample_frames.jsonl", examples_dir / "sample_frames.jsonl")
    copy_file(repo / "README.md", package_root / "README.md")
    copy_file(repo / "LICENSE", package_root / "LICENSE")

    package_zip = output_dir / f"{PACKAGE_NAME}.zip"
    write_zip(package_root, package_zip)

    manifest = {
        "format": "autoaim.windows.package.v1",
        "app": "AutoAim Review",
        "version": args.version,
        "target": "windows-x64",
        "package_asset": {
            "name": package_zip.name,
            "sha256": sha256_file(package_zip),
            "size": package_zip.stat().st_size,
        },
        "files": [record.__dict__ for record in collect_files(package_root)],
    }
    manifest_path = output_dir / f"{PACKAGE_NAME}-manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2, sort_keys=True), encoding="utf-8")

    delta_entries: list[dict] = []
    if args.previous_package and args.previous_manifest:
        delta_entry, _ = build_delta(
            old_package=Path(args.previous_package).resolve(),
            old_manifest_path=Path(args.previous_manifest).resolve(),
            new_package_root=package_root,
            new_manifest=manifest,
            output_dir=output_dir,
        )
        delta_entries.append(delta_entry)

    delta_index = {
        "format": "autoaim.windows.delta-index.v1",
        "target": "windows-x64",
        "to_version": args.version,
        "deltas": delta_entries,
    }
    delta_index_path = output_dir / f"{PACKAGE_NAME}-deltas.json"
    delta_index_path.write_text(json.dumps(delta_index, indent=2, sort_keys=True), encoding="utf-8")

    print(f"package={package_zip}")
    print(f"manifest={manifest_path}")
    print(f"delta_index={delta_index_path}")


def main() -> int:
    parser = argparse.ArgumentParser(description="Build the Windows release package.")
    parser.add_argument("--repo", default=os.getcwd())
    parser.add_argument("--version", required=True)
    parser.add_argument("--output-dir", default="dist/windows")
    parser.add_argument("--previous-package")
    parser.add_argument("--previous-manifest")
    parser.add_argument("--exe-path", help="Deprecated alias for --cli-exe-path")
    parser.add_argument("--cli-exe-path")
    parser.add_argument("--app-exe-path")
    parser.add_argument("--skip-build", action="store_true")
    args = parser.parse_args()

    if bool(args.previous_package) != bool(args.previous_manifest):
        parser.error("--previous-package and --previous-manifest must be used together")

    build_package(args)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
