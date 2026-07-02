from __future__ import annotations

from dataclasses import dataclass, field
from math import hypot
from typing import Any

BBox = tuple[float, float, float, float]
Point = tuple[float, float]


def _point(value: list[float] | tuple[float, float] | None, default: Point = (0.0, 0.0)) -> Point:
    if value is None:
        return default
    if len(value) != 2:
        raise ValueError(f"point must contain 2 values, got {value!r}")
    return float(value[0]), float(value[1])


def _bbox(value: list[float] | tuple[float, float, float, float] | None) -> BBox | None:
    if value is None:
        return None
    if len(value) != 4:
        raise ValueError(f"bbox must contain 4 values, got {value!r}")
    return float(value[0]), float(value[1]), float(value[2]), float(value[3])


def bbox_center(bbox: BBox) -> Point:
    x, y, w, h = bbox
    return x + w / 2.0, y + h / 2.0


@dataclass(frozen=True)
class FrameInput:
    cursor: Point = (0.0, 0.0)
    mouse_down: bool = False

    @classmethod
    def from_dict(cls, data: dict[str, Any] | None) -> "FrameInput":
        data = data or {}
        return cls(cursor=_point(data.get("cursor")), mouse_down=bool(data.get("mouse_down", False)))

    def to_dict(self) -> dict[str, Any]:
        return {"cursor": [self.cursor[0], self.cursor[1]], "mouse_down": self.mouse_down}


@dataclass(frozen=True)
class DetectionObject:
    class_name: str
    bbox: BBox
    head_bbox: BBox | None = None
    head_point: Point | None = None
    confidence: float = 0.0
    track_id: int | None = None
    attributes: dict[str, Any] = field(default_factory=dict)

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "DetectionObject":
        class_name = str(data.get("class") or data.get("class_name") or "person")
        bbox = _bbox(data.get("bbox"))
        if bbox is None:
            raise ValueError("object.bbox is required")
        return cls(
            class_name=class_name,
            bbox=bbox,
            head_bbox=_bbox(data.get("head_bbox")),
            head_point=_point(data["head_point"]) if data.get("head_point") is not None else None,
            confidence=float(data.get("confidence", 0.0)),
            track_id=data.get("track_id"),
            attributes=dict(data.get("attributes") or {}),
        )

    def aim_point(self) -> Point:
        if self.head_point is not None:
            return self.head_point
        if self.head_bbox is not None:
            return bbox_center(self.head_bbox)
        x, y, w, h = self.bbox
        return x + w / 2.0, y + h * 0.18

    def distance_to_cursor(self, cursor: Point) -> float:
        point = self.aim_point()
        return hypot(point[0] - cursor[0], point[1] - cursor[1])

    def to_dict(self) -> dict[str, Any]:
        result: dict[str, Any] = {
            "class": self.class_name,
            "bbox": list(self.bbox),
            "confidence": self.confidence,
        }
        if self.head_bbox is not None:
            result["head_bbox"] = list(self.head_bbox)
        if self.head_point is not None:
            result["head_point"] = list(self.head_point)
        if self.track_id is not None:
            result["track_id"] = self.track_id
        if self.attributes:
            result["attributes"] = self.attributes
        return result


@dataclass(frozen=True)
class FrameRecord:
    frame_id: int
    timestamp_qpc: int
    image: str
    objects: tuple[DetectionObject, ...] = ()
    input: FrameInput = field(default_factory=FrameInput)
    resolution: tuple[int, int] | None = None
    window_handle: str | None = None
    session_id: str | None = None
    scene_id: str | None = None

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "FrameRecord":
        resolution = data.get("resolution")
        parsed_resolution = None
        if resolution is not None:
            if len(resolution) != 2:
                raise ValueError("resolution must contain 2 values")
            parsed_resolution = (int(resolution[0]), int(resolution[1]))
        return cls(
            frame_id=int(data["frame_id"]),
            timestamp_qpc=int(data["timestamp_qpc"]),
            image=str(data.get("image", "")),
            objects=tuple(DetectionObject.from_dict(item) for item in data.get("objects", [])),
            input=FrameInput.from_dict(data.get("input")),
            resolution=parsed_resolution,
            window_handle=data.get("window_handle"),
            session_id=data.get("session_id"),
            scene_id=data.get("scene_id"),
        )

    def to_dict(self) -> dict[str, Any]:
        result: dict[str, Any] = {
            "frame_id": self.frame_id,
            "timestamp_qpc": self.timestamp_qpc,
            "image": self.image,
            "objects": [item.to_dict() for item in self.objects],
            "input": self.input.to_dict(),
        }
        if self.resolution is not None:
            result["resolution"] = list(self.resolution)
        if self.window_handle:
            result["window_handle"] = self.window_handle
        if self.session_id:
            result["session_id"] = self.session_id
        if self.scene_id:
            result["scene_id"] = self.scene_id
        return result


@dataclass(frozen=True)
class AimSuggestion:
    frame_id: int
    suggested_point: Point | None
    confidence: float
    target_index: int | None
    dx: float | None
    dy: float | None
    score: float

    def to_dict(self) -> dict[str, Any]:
        return {
            "frame_id": self.frame_id,
            "suggested_point": list(self.suggested_point) if self.suggested_point is not None else None,
            "confidence": self.confidence,
            "target_index": self.target_index,
            "dx": self.dx,
            "dy": self.dy,
            "score": self.score,
        }


@dataclass(frozen=True)
class MetricsSummary:
    frame_count: int
    object_count: int
    target_count: int
    mean_confidence: float
    mean_abs_dx: float
    mean_abs_dy: float
    mean_distance: float

    def to_dict(self) -> dict[str, Any]:
        return {
            "frame_count": self.frame_count,
            "object_count": self.object_count,
            "target_count": self.target_count,
            "mean_confidence": self.mean_confidence,
            "mean_abs_dx": self.mean_abs_dx,
            "mean_abs_dy": self.mean_abs_dy,
            "mean_distance": self.mean_distance,
        }
