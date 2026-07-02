from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from .models import BBox, FrameRecord

CLASS_MAP = {"person": 0, "head": 1}


@dataclass(frozen=True)
class ExportResult:
    label_files: int
    labels: list[str]

    def to_dict(self) -> dict[str, object]:
        return {"label_files": self.label_files, "labels": self.labels}


def _normalize_bbox(bbox: BBox, width: int, height: int) -> tuple[float, float, float, float]:
    x, y, w, h = bbox
    center_x = (x + w / 2.0) / width
    center_y = (y + h / 2.0) / height
    return center_x, center_y, w / width, h / height


def _format_yolo_line(class_id: int, bbox: BBox, width: int, height: int) -> str:
    values = _normalize_bbox(bbox, width, height)
    return f"{class_id} " + " ".join(f"{value:.6f}" for value in values)


def export_yolo(records: list[FrameRecord], output_dir: str | Path, include_head: bool = True) -> ExportResult:
    labels_dir = Path(output_dir)
    labels_dir.mkdir(parents=True, exist_ok=True)

    written: list[str] = []
    for record in records:
        if record.resolution is None:
            raise ValueError(f"frame {record.frame_id} is missing resolution; YOLO export requires image dimensions")
        width, height = record.resolution
        lines: list[str] = []
        for obj in record.objects:
            if obj.class_name == "person":
                lines.append(_format_yolo_line(CLASS_MAP["person"], obj.bbox, width, height))
            if include_head and obj.head_bbox is not None:
                lines.append(_format_yolo_line(CLASS_MAP["head"], obj.head_bbox, width, height))

        label_name = Path(record.image).with_suffix(".txt").name or f"{record.frame_id:08d}.txt"
        label_path = labels_dir / label_name
        label_path.write_text("\n".join(lines) + ("\n" if lines else ""), encoding="utf-8")
        written.append(str(label_path))

    return ExportResult(label_files=len(written), labels=written)
