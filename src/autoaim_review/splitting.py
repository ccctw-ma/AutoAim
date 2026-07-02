from __future__ import annotations

import hashlib
from collections import defaultdict
from dataclasses import dataclass

from .models import FrameRecord


@dataclass(frozen=True)
class SplitResult:
    train: list[FrameRecord]
    val: list[FrameRecord]
    test: list[FrameRecord]
    groups: dict[str, str]

    def counts(self) -> dict[str, int]:
        return {"train": len(self.train), "val": len(self.val), "test": len(self.test)}


def group_key(record: FrameRecord) -> str:
    if record.session_id:
        return f"session:{record.session_id}"
    if record.scene_id:
        return f"scene:{record.scene_id}"
    return f"window:{record.window_handle or 'unknown'}"


def _bucket_for_group(key: str, train_ratio: float, val_ratio: float) -> str:
    digest = hashlib.sha256(key.encode("utf-8")).hexdigest()
    value = int(digest[:8], 16) / 0xFFFFFFFF
    if value < train_ratio:
        return "train"
    if value < train_ratio + val_ratio:
        return "val"
    return "test"


def split_by_group(records: list[FrameRecord], train_ratio: float = 0.8, val_ratio: float = 0.1) -> SplitResult:
    if not 0 < train_ratio < 1:
        raise ValueError("train_ratio must be between 0 and 1")
    if not 0 <= val_ratio < 1:
        raise ValueError("val_ratio must be between 0 and 1")
    if train_ratio + val_ratio >= 1:
        raise ValueError("train_ratio + val_ratio must be < 1")

    grouped: dict[str, list[FrameRecord]] = defaultdict(list)
    for record in records:
        grouped[group_key(record)].append(record)

    splits = {"train": [], "val": [], "test": []}
    assignments: dict[str, str] = {}
    for key in sorted(grouped):
        bucket = _bucket_for_group(key, train_ratio, val_ratio)
        splits[bucket].extend(grouped[key])
        assignments[key] = bucket

    return SplitResult(
        train=splits["train"],
        val=splits["val"],
        test=splits["test"],
        groups=assignments,
    )
