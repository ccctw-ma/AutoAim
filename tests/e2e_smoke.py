from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path


def run(command: list[str], cwd: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(command, cwd=cwd, check=True, capture_output=True, text=True)


def main() -> int:
    repo = Path(__file__).resolve().parents[1]
    sample = repo / "examples" / "sample_frames.jsonl"
    output_dir = repo / ".e2e-output"
    labels_dir = output_dir / "labels"
    cli = Path(sys.executable).parent / ("autoaim-review.exe" if sys.platform == "win32" else "autoaim-review")
    output_dir.mkdir(exist_ok=True)
    labels_dir.mkdir(exist_ok=True)

    validate = run([str(cli), "validate", str(sample)], repo)
    validate_payload = json.loads(validate.stdout)
    assert validate_payload["ok"] is True
    assert validate_payload["issue_count"] == 0

    evaluate = run([str(cli), "evaluate", str(sample)], repo)
    evaluate_payload = json.loads(evaluate.stdout)
    assert evaluate_payload["safety"]["mouse_control"] is False
    assert evaluate_payload["safety"]["third_party_game_control"] is False
    assert evaluate_payload["summary"]["frame_count"] == 2
    assert len(evaluate_payload["suggestions"]) == 2

    export = run([str(cli), "export-yolo", str(sample), str(labels_dir)], repo)
    export_payload = json.loads(export.stdout)
    assert export_payload["label_files"] == 2
    label_files = sorted(labels_dir.glob("*.txt"))
    assert len(label_files) == 2
    assert label_files[0].read_text(encoding="utf-8").splitlines()[0].startswith("0 ")

    split = run([str(cli), "split", str(sample)], repo)
    split_payload = json.loads(split.stdout)
    assert split_payload["safety"]["random_frame_split"] is False
    assert sum(split_payload["counts"].values()) == 2

    print("e2e smoke passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
