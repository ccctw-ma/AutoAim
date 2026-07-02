from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path


def run(command: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command, cwd=cwd, check=True, capture_output=True, text=True)


def main() -> int:
    repo = Path(__file__).resolve().parents[1]
    sample = repo / "examples" / "sample_frames.jsonl"
    target = repo / "target" / "debug" / ("autoaim.exe" if os.name == "nt" else "autoaim")
    if not target.exists():
        run(["cargo", "build", "-p", "autoaim-cli"], repo)

    positions = run([str(target), "positions", str(sample)], repo)
    lines = [json.loads(line) for line in positions.stdout.splitlines() if line.strip()]
    assert len(lines) == 3
    assert lines[0]["bbox"] == [420.0, 180.0, 120.0, 360.0]
    assert lines[0]["head_point"] == [479.0, 211.0]
    assert lines[0]["dx"] == -33.0
    assert lines[0]["dy"] == -173.0

    mousedown = repo / ".e2e-output" / "mousedown_frames.jsonl"
    mousedown.parent.mkdir(exist_ok=True)
    records = sample.read_text(encoding="utf-8").splitlines()
    first = json.loads(records[0])
    first["input"]["mouse_down"] = True
    mousedown.write_text(json.dumps(first) + "\n" + "\n".join(records[1:]) + "\n", encoding="utf-8")

    assist = run([str(target), "positions", str(mousedown), "--assist-events"], repo)
    events = [json.loads(line) for line in assist.stdout.splitlines() if line.strip()]
    assist_events = [event for event in events if event.get("type") == "assist.suggestion"]
    assert len(assist_events) == 1
    assert assist_events[0]["review_only"] is True
    assert assist_events[0]["suggestion"]["suggested_point"] == [479.0, 211.0]

    print("rust cli e2e passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
