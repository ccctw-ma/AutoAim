from __future__ import annotations

import argparse
import hashlib
import os
import shutil
import sys
import tempfile
import urllib.request
from pathlib import Path
from zipfile import ZipFile


REQUIRED_MODELS = {
    "yolov8n.onnx": {
        "sha256": "18e69ffef4db38463f7b0c12f66aca599f3ec0d062675af536a25ea1860601d0",
        "env_url": "AUTOAIM_YOLOV8_ONNX_URL",
        "default_url": "https://www.modelscope.cn/models/cix/ai_model_hub_25_Q3/resolve/master/models/ComputeVision/Object_Detection/onnx_yolov8_n/model/yolov8n.onnx",
    },
    "movenet_lightning.onnx": {
        "sha256": "65ae8c693f8c4649101255221f66f3fed7b7ba977a3dce0bbd68663ce035982e",
        "env_url": "AUTOAIM_MOVENET_ONNX_URL",
    },
    "movenet_lightning.tflite": {
        "sha256": "0fac2226112d0371903ca86e3853cec24ef603a0b2f96f589b180f0ebdd135ab",
        "env_url": "AUTOAIM_MOVENET_TFLITE_URL",
    },
}


def sha256_file(path: Path) -> str:
    hasher = hashlib.sha256()
    with path.open("rb") as file:
        for chunk in iter(lambda: file.read(1024 * 1024), b""):
            hasher.update(chunk)
    return hasher.hexdigest()


def is_valid_model(path: Path, expected_sha256: str) -> bool:
    return path.is_file() and sha256_file(path) == expected_sha256


def download(url: str, output_path: Path) -> None:
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with urllib.request.urlopen(url) as response, output_path.open("wb") as file:
        shutil.copyfileobj(response, file)


def copy_checked(source: Path, destination: Path, expected_sha256: str) -> None:
    actual = sha256_file(source)
    if actual != expected_sha256:
        raise ValueError(
            f"{source} has sha256 {actual}, expected {expected_sha256}"
        )
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)


def extract_from_package(package_path: Path, models_dir: Path) -> set[str]:
    extracted: set[str] = set()
    if not package_path.is_file():
        return extracted

    with ZipFile(package_path) as archive:
        names = set(archive.namelist())
        for model_name, metadata in REQUIRED_MODELS.items():
            candidates = [
                f"models/{model_name}",
                f"AutoAimReview/models/{model_name}",
                model_name,
            ]
            archive_name = next((name for name in candidates if name in names), None)
            if archive_name is None:
                continue

            target = models_dir / model_name
            target.parent.mkdir(parents=True, exist_ok=True)
            with archive.open(archive_name) as source, target.open("wb") as output:
                shutil.copyfileobj(source, output)
            if not is_valid_model(target, metadata["sha256"]):
                target.unlink(missing_ok=True)
                raise ValueError(f"{archive_name} in {package_path} failed sha256 check")
            extracted.add(model_name)
    return extracted


def prepare_models(args: argparse.Namespace) -> None:
    models_dir = Path(args.models_dir).resolve()
    models_dir.mkdir(parents=True, exist_ok=True)

    missing = {
        name
        for name, metadata in REQUIRED_MODELS.items()
        if not is_valid_model(models_dir / name, metadata["sha256"])
    }
    if not missing:
        print(f"models ready: {models_dir}")
        return

    package = args.package or os.environ.get("AUTOAIM_MODEL_PACKAGE")
    if package:
        extracted = extract_from_package(Path(package).resolve(), models_dir)
        missing -= extracted

    package_url = args.package_url or os.environ.get("AUTOAIM_MODEL_PACKAGE_URL")
    if missing and package_url:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_path = Path(temp_dir) / "AutoAimReview-windows-x64.zip"
            download(package_url, package_path)
            extracted = extract_from_package(package_path, models_dir)
            missing -= extracted

    direct_urls = {
        "yolov8n.onnx": args.yolo_url or os.environ.get("AUTOAIM_YOLOV8_ONNX_URL") or REQUIRED_MODELS["yolov8n.onnx"]["default_url"],
        "movenet_lightning.onnx": args.onnx_url or os.environ.get("AUTOAIM_MOVENET_ONNX_URL"),
        "movenet_lightning.tflite": args.tflite_url or os.environ.get("AUTOAIM_MOVENET_TFLITE_URL"),
    }
    for model_name in list(missing):
        url = direct_urls.get(model_name)
        if not url:
            continue
        target = models_dir / model_name
        download(url, target)
        if not is_valid_model(target, REQUIRED_MODELS[model_name]["sha256"]):
            target.unlink(missing_ok=True)
            raise ValueError(f"downloaded {model_name} failed sha256 check")
        missing.remove(model_name)

    if missing:
        names = ", ".join(sorted(missing))
        raise FileNotFoundError(
            "missing model files: "
            f"{names}. Copy them into models/, pass --package with a previous "
            "AutoAimReview-windows-x64.zip, or set AUTOAIM_MOVENET_ONNX_URL and "
            "AUTOAIM_MOVENET_TFLITE_URL."
        )

    print(f"models ready: {models_dir}")


def main() -> int:
    parser = argparse.ArgumentParser(description="Prepare local AutoAim model files.")
    parser.add_argument("--models-dir", default="models")
    parser.add_argument("--package", help="Existing AutoAimReview-windows-x64.zip to extract models from")
    parser.add_argument("--package-url", help="URL of an AutoAimReview-windows-x64.zip release asset")
    parser.add_argument("--yolo-url", help="Direct URL for yolov8n.onnx")
    parser.add_argument("--onnx-url", help="Direct URL for movenet_lightning.onnx")
    parser.add_argument("--tflite-url", help="Direct URL for movenet_lightning.tflite")
    args = parser.parse_args()

    prepare_models(args)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
