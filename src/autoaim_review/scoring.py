from __future__ import annotations

from dataclasses import dataclass

from .models import AimSuggestion, DetectionObject, FrameRecord, Point


@dataclass(frozen=True)
class TargetScorer:
    confidence_weight: float = 0.55
    distance_weight: float = 0.35
    stability_weight: float = 0.10
    max_distance_px: float = 900.0

    def score(self, target: DetectionObject, cursor: Point, previous_track_id: int | None = None) -> float:
        distance = min(target.distance_to_cursor(cursor), self.max_distance_px)
        distance_score = 1.0 - distance / self.max_distance_px
        confidence_score = max(0.0, min(1.0, target.confidence))
        stability_score = 1.0 if previous_track_id is not None and target.track_id == previous_track_id else 0.0
        return (
            confidence_score * self.confidence_weight
            + distance_score * self.distance_weight
            + stability_score * self.stability_weight
        )


def choose_target(
    frame: FrameRecord,
    scorer: TargetScorer | None = None,
    previous_track_id: int | None = None,
) -> AimSuggestion:
    scorer = scorer or TargetScorer()
    candidates = [item for item in frame.objects if item.class_name == "person"]
    if not candidates:
        return AimSuggestion(
            frame_id=frame.frame_id,
            suggested_point=None,
            confidence=0.0,
            target_index=None,
            dx=None,
            dy=None,
            score=0.0,
        )

    scored = [
        (index, item, scorer.score(item, frame.input.cursor, previous_track_id))
        for index, item in enumerate(frame.objects)
        if item in candidates
    ]
    target_index, target, score = max(scored, key=lambda item: item[2])
    point = target.aim_point()
    cursor_x, cursor_y = frame.input.cursor
    return AimSuggestion(
        frame_id=frame.frame_id,
        suggested_point=point,
        confidence=target.confidence,
        target_index=target_index,
        dx=point[0] - cursor_x,
        dy=point[1] - cursor_y,
        score=score,
    )
