from __future__ import annotations

from math import hypot

from .models import AimSuggestion, FrameRecord, MetricsSummary
from .scoring import TargetScorer, choose_target


def suggest_frames(records: list[FrameRecord], scorer: TargetScorer | None = None) -> list[AimSuggestion]:
    suggestions: list[AimSuggestion] = []
    previous_track_id: int | None = None
    for record in records:
        suggestion = choose_target(record, scorer=scorer, previous_track_id=previous_track_id)
        suggestions.append(suggestion)
        if suggestion.target_index is not None:
            selected = record.objects[suggestion.target_index]
            previous_track_id = selected.track_id
    return suggestions


def summarize(records: list[FrameRecord], suggestions: list[AimSuggestion] | None = None) -> MetricsSummary:
    suggestions = suggestions if suggestions is not None else suggest_frames(records)
    target_suggestions = [item for item in suggestions if item.suggested_point is not None]
    object_count = sum(len(item.objects) for item in records)
    if not target_suggestions:
        return MetricsSummary(
            frame_count=len(records),
            object_count=object_count,
            target_count=0,
            mean_confidence=0.0,
            mean_abs_dx=0.0,
            mean_abs_dy=0.0,
            mean_distance=0.0,
        )

    count = len(target_suggestions)
    confidence = sum(item.confidence for item in target_suggestions) / count
    mean_abs_dx = sum(abs(item.dx or 0.0) for item in target_suggestions) / count
    mean_abs_dy = sum(abs(item.dy or 0.0) for item in target_suggestions) / count
    mean_distance = sum(hypot(item.dx or 0.0, item.dy or 0.0) for item in target_suggestions) / count
    return MetricsSummary(
        frame_count=len(records),
        object_count=object_count,
        target_count=count,
        mean_confidence=confidence,
        mean_abs_dx=mean_abs_dx,
        mean_abs_dy=mean_abs_dy,
        mean_distance=mean_distance,
    )
