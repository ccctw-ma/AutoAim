from __future__ import annotations

import argparse
import base64
import hashlib
import json
import shutil
import tempfile
from pathlib import Path
from zipfile import ZipFile


def sha256_file(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as file:
        for chunk in iter(lambda: file.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def extract_package(path: Path, output_dir: Path) -> Path:
    with ZipFile(path) as archive:
        archive.extractall(output_dir)
    children = list(output_dir.iterdir())
    if len(children) == 1 and children[0].is_dir():
        return children[0]
    return output_dir


def decode_base64_to_file(value: str, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(base64.b64decode(value.encode("ascii")))


def apply_patch(source: Path, destination: Path, file_change: dict) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)
    with destination.open("r+b") as file:
        for block in file_change["patch"]["blocks"]:
            file.seek(int(block["offset"]))
            file.write(base64.b64decode(block["data_base64"].encode("ascii")))
        file.truncate(int(file_change["to_size"]))


def apply_delta(root: Path, delta: dict) -> None:
    for change in delta["files"]:
        relative = Path(change["path"])
        target = root / relative
        action = change["action"]

        if action == "remove":
            if target.exists():
                target.unlink()
        elif action == "add":
            decode_base64_to_file(change["content_base64"], target)
        elif action == "patch":
            staged = target.with_suffix(target.suffix + ".patched")
            apply_patch(target, staged, change)
            staged.replace(target)
        else:
            raise ValueError(f"unsupported delta action: {action}")


def assert_manifest(root: Path, manifest: dict) -> None:
    expected = {item["path"]: item for item in manifest["files"]}
    actual_paths = sorted(path.relative_to(root).as_posix() for path in root.rglob("*") if path.is_file())

    missing = sorted(set(expected) - set(actual_paths))
    extra = sorted(set(actual_paths) - set(expected))
    if missing or extra:
        raise AssertionError(f"file set mismatch missing={missing} extra={extra}")

    for path, record in expected.items():
        actual = root / path
        actual_size = actual.stat().st_size
        actual_hash = sha256_file(actual)
        if actual_size != record["size"] or actual_hash != record["sha256"]:
            raise AssertionError(
                f"file mismatch for {path}: size {actual_size}/{record['size']}, "
                f"sha256 {actual_hash}/{record['sha256']}"
            )


def main() -> int:
    parser = argparse.ArgumentParser(description="Verify an AutoAim Windows delta package.")
    parser.add_argument("--old-package", required=True)
    parser.add_argument("--delta", required=True)
    parser.add_argument("--new-manifest", required=True)
    args = parser.parse_args()

    old_package = Path(args.old_package).resolve()
    delta = json.loads(Path(args.delta).read_text(encoding="utf-8"))
    new_manifest = json.loads(Path(args.new_manifest).read_text(encoding="utf-8"))

    with tempfile.TemporaryDirectory(prefix="autoaim-delta-verify-") as temp:
        root = extract_package(old_package, Path(temp) / "old")
        apply_delta(root, delta)
        assert_manifest(root, new_manifest)

    summary = delta.get("summary", {})
    print(
        "delta verification passed: "
        f"changed={summary.get('changed_files', 0)} "
        f"added={summary.get('added_files', 0)} "
        f"removed={summary.get('removed_files', 0)}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
