from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from .dataset import read_jsonl
from .evaluation import suggest_frames, summarize
from .exporters import export_yolo
from .splitting import split_by_group
from .validation import validate_records


def _cmd_evaluate(args: argparse.Namespace) -> int:
    records = read_jsonl(args.input)
    suggestions = suggest_frames(records)
    summary = summarize(records, suggestions)
    result = {
        "summary": summary.to_dict(),
        "suggestions": [item.to_dict() for item in suggestions],
        "safety": {
            "mode": "visualization_only",
            "mouse_control": False,
            "third_party_game_control": False,
        },
    }
    if args.output:
        Path(args.output).write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
    else:
        json.dump(result, sys.stdout, ensure_ascii=False, indent=2)
        sys.stdout.write("\n")
    return 0


def _cmd_validate(args: argparse.Namespace) -> int:
    records = read_jsonl(args.input)
    issues = validate_records(records)
    result = {
        "ok": not any(item.level == "error" for item in issues),
        "issue_count": len(issues),
        "issues": [item.to_dict() for item in issues],
    }
    json.dump(result, sys.stdout, ensure_ascii=False, indent=2)
    sys.stdout.write("\n")
    return 0 if result["ok"] else 1


def _cmd_export_yolo(args: argparse.Namespace) -> int:
    records = read_jsonl(args.input)
    result = export_yolo(records, args.output, include_head=not args.no_head)
    json.dump(result.to_dict(), sys.stdout, ensure_ascii=False, indent=2)
    sys.stdout.write("\n")
    return 0


def _cmd_split(args: argparse.Namespace) -> int:
    records = read_jsonl(args.input)
    result = split_by_group(records, train_ratio=args.train_ratio, val_ratio=args.val_ratio)
    payload = {
        "counts": result.counts(),
        "groups": result.groups,
        "safety": {
            "split_policy": "grouped_by_session_scene_or_window",
            "random_frame_split": False,
        },
    }
    json.dump(payload, sys.stdout, ensure_ascii=False, indent=2)
    sys.stdout.write("\n")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="autoaim-review",
        description="Visualization-only AutoAim review and evaluation tools.",
    )
    sub = parser.add_subparsers(dest="command", required=True)

    evaluate = sub.add_parser("evaluate", help="Evaluate frame JSONL and emit suggested aim points.")
    evaluate.add_argument("input", help="Frame annotation JSONL input.")
    evaluate.add_argument("-o", "--output", help="Optional JSON output path.")
    evaluate.set_defaults(func=_cmd_evaluate)

    validate = sub.add_parser("validate", help="Validate frame JSONL quality and schema-level invariants.")
    validate.add_argument("input", help="Frame annotation JSONL input.")
    validate.set_defaults(func=_cmd_validate)

    export_yolo_parser = sub.add_parser("export-yolo", help="Export person/head boxes as YOLO label files.")
    export_yolo_parser.add_argument("input", help="Frame annotation JSONL input.")
    export_yolo_parser.add_argument("output", help="Output labels directory.")
    export_yolo_parser.add_argument("--no-head", action="store_true", help="Export person boxes only.")
    export_yolo_parser.set_defaults(func=_cmd_export_yolo)

    split = sub.add_parser("split", help="Plan grouped dataset split without random frame leakage.")
    split.add_argument("input", help="Frame annotation JSONL input.")
    split.add_argument("--train-ratio", type=float, default=0.8)
    split.add_argument("--val-ratio", type=float, default=0.1)
    split.set_defaults(func=_cmd_split)
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
