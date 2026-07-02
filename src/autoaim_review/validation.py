from __future__ import annotations

from dataclasses import dataclass

from .models import BBox, FrameRecord, Point


@dataclass(frozen=True)
class ValidationIssue:
    frame_id: int
    level: str
    code: str
    message: str

    def to_dict(self) -> dict[str, object]:
        return {
            "frame_id": self.frame_id,
            "level": self.level,
            "code": self.code,
            "message": self.message,
        }


def _bbox_in_bounds(bbox: BBox, width: int, height: int) -> bool:
    x, y, w, h = bbox
    return x >= 0 and y >= 0 and w > 0 and h > 0 and x + w <= width and y + h <= height


def _point_in_bounds(point: Point, width: int, height: int) -> bool:
    x, y = point
    return 0 <= x <= width and 0 <= y <= height


def validate_records(records: list[FrameRecord]) -> list[ValidationIssue]:
    issues: list[ValidationIssue] = []
    seen_frame_ids: set[int] = set()
    for record in records:
        if record.frame_id in seen_frame_ids:
            issues.append(
                ValidationIssue(record.frame_id, "error", "duplicate_frame_id", "frame_id appears more than once")
            )
        seen_frame_ids.add(record.frame_id)

        if not record.image:
            issues.append(ValidationIssue(record.frame_id, "error", "missing_image", "image path is required"))

        if record.timestamp_qpc < 0:
            issues.append(ValidationIssue(record.frame_id, "error", "negative_timestamp", "timestamp_qpc must be >= 0"))

        width = height = None
        if record.resolution is not None:
            width, height = record.resolution
            if width <= 0 or height <= 0:
                issues.append(
                    ValidationIssue(record.frame_id, "error", "invalid_resolution", "resolution must be positive")
                )

        for index, obj in enumerate(record.objects):
            prefix = f"objects[{index}]"
            if obj.class_name != "person":
                issues.append(
                    ValidationIssue(record.frame_id, "warning", "non_person_object", f"{prefix}.class is not person")
                )
            if not 0.0 <= obj.confidence <= 1.0:
                issues.append(
                    ValidationIssue(
                        record.frame_id,
                        "error",
                        "invalid_confidence",
                        f"{prefix}.confidence must be between 0 and 1",
                    )
                )
            x, y, w, h = obj.bbox
            if w <= 0 or h <= 0:
                issues.append(ValidationIssue(record.frame_id, "error", "invalid_bbox", f"{prefix}.bbox size must be positive"))
            if width is not None and height is not None and not _bbox_in_bounds(obj.bbox, width, height):
                issues.append(
                    ValidationIssue(record.frame_id, "warning", "bbox_out_of_bounds", f"{prefix}.bbox exceeds resolution")
                )
            if obj.head_bbox is not None and width is not None and height is not None:
                if not _bbox_in_bounds(obj.head_bbox, width, height):
                    issues.append(
                        ValidationIssue(
                            record.frame_id,
                            "warning",
                            "head_bbox_out_of_bounds",
                            f"{prefix}.head_bbox exceeds resolution",
                        )
                    )
            if obj.head_point is not None and width is not None and height is not None:
                if not _point_in_bounds(obj.head_point, width, height):
                    issues.append(
                        ValidationIssue(
                            record.frame_id,
                            "warning",
                            "head_point_out_of_bounds",
                            f"{prefix}.head_point exceeds resolution",
                        )
                    )
            if obj.head_bbox is None and obj.head_point is None:
                issues.append(
                    ValidationIssue(
                        record.frame_id,
                        "warning",
                        "missing_head_annotation",
                        f"{prefix} lacks head_bbox/head_point; fallback aim point will be coarse",
                    )
                )
    return issues
