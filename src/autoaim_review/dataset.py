from __future__ import annotations

import json
from collections.abc import Iterable
from pathlib import Path
from typing import Any

from .models import FrameRecord


def read_jsonl(path: str | Path) -> list[FrameRecord]:
    records: list[FrameRecord] = []
    with Path(path).open("r", encoding="utf-8") as source:
        for line_number, line in enumerate(source, start=1):
            stripped = line.strip()
            if not stripped:
                continue
            try:
                records.append(FrameRecord.from_dict(json.loads(stripped)))
            except (json.JSONDecodeError, KeyError, TypeError, ValueError) as exc:
                raise ValueError(f"invalid record at {path}:{line_number}: {exc}") from exc
    return records


def write_jsonl(path: str | Path, records: Iterable[FrameRecord]) -> None:
    with Path(path).open("w", encoding="utf-8") as target:
        for record in records:
            target.write(json.dumps(record.to_dict(), ensure_ascii=False, separators=(",", ":")))
            target.write("\n")


def read_json(path: str | Path) -> Any:
    with Path(path).open("r", encoding="utf-8") as source:
        return json.load(source)
