from autoaim_review.dataset import read_jsonl
from autoaim_review.evaluation import suggest_frames, summarize
from autoaim_review.exporters import export_yolo
from autoaim_review.models import DetectionObject, FrameInput, FrameRecord
from autoaim_review.scoring import choose_target
from autoaim_review.splitting import split_by_group
from autoaim_review.validation import validate_records


def test_choose_target_prefers_head_point_near_cursor():
    frame = FrameRecord(
        frame_id=1,
        timestamp_qpc=10,
        image="frame.jpg",
        input=FrameInput(cursor=(500, 300), mouse_down=False),
        objects=(
            DetectionObject(
                class_name="person",
                bbox=(100, 100, 100, 300),
                head_point=(150, 140),
                confidence=0.99,
            ),
            DetectionObject(
                class_name="person",
                bbox=(470, 260, 80, 220),
                head_point=(505, 295),
                confidence=0.75,
            ),
        ),
    )

    suggestion = choose_target(frame)

    assert suggestion.target_index == 1
    assert suggestion.suggested_point == (505, 295)
    assert suggestion.dx == 5
    assert suggestion.dy == -5


def test_dataset_summary_uses_sample_file():
    records = read_jsonl("examples/sample_frames.jsonl")
    suggestions = suggest_frames(records)
    summary = summarize(records, suggestions)

    assert summary.frame_count == 2
    assert summary.object_count == 3
    assert summary.target_count == 2
    assert summary.mean_confidence > 0.8


def test_validate_records_accepts_sample_file():
    records = read_jsonl("examples/sample_frames.jsonl")

    assert validate_records(records) == []


def test_export_yolo_writes_person_and_head_labels(tmp_path):
    records = read_jsonl("examples/sample_frames.jsonl")

    result = export_yolo(records[:1], tmp_path)

    assert result.label_files == 1
    label_text = (tmp_path / "00010231.txt").read_text(encoding="utf-8")
    lines = label_text.strip().splitlines()
    assert len(lines) == 4
    assert lines[0].startswith("0 ")
    assert lines[1].startswith("1 ")


def test_split_by_group_keeps_session_together():
    records = read_jsonl("examples/sample_frames.jsonl")

    result = split_by_group(records)

    counts = result.counts()
    assert sum(counts.values()) == 2
    assert len([value for value in counts.values() if value > 0]) == 1
    assert result.groups == {"session:session-001": next(key for key, value in counts.items() if value > 0)}
