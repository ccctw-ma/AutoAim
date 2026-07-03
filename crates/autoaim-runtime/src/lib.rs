use autoaim_core::{choose_target, AutoAimError, FrameRecord, ObjectTracker, Point, TargetScorer};
use autoaim_ipc::{encode_json_line, AssistSuggestionEvent, InferenceResult};
use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::Path,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RuntimeConfig {
    pub scorer: TargetScorer,
    pub default_latency_ms: f32,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            scorer: TargetScorer::default(),
            default_latency_ms: 0.0,
        }
    }
}

#[derive(Debug)]
pub struct ReviewPipeline {
    config: RuntimeConfig,
    previous_track_id: Option<u64>,
    tracker: ObjectTracker,
}

impl ReviewPipeline {
    pub fn new(config: RuntimeConfig) -> Self {
        Self {
            config,
            previous_track_id: None,
            tracker: ObjectTracker::default(),
        }
    }

    pub fn process_frame(&mut self, frame: &FrameRecord) -> InferenceResult {
        self.process_frame_with_latency(frame, self.config.default_latency_ms)
    }

    pub fn process_frame_with_assist(
        &mut self,
        frame: &FrameRecord,
    ) -> (InferenceResult, Option<AssistSuggestionEvent>) {
        self.process_frame_with_assist_and_latency(frame, self.config.default_latency_ms)
    }

    pub fn process_frame_with_latency(
        &mut self,
        frame: &FrameRecord,
        latency_ms: f32,
    ) -> InferenceResult {
        let tracked_frame = self.tracked_frame(frame);
        let suggestion = choose_target(&tracked_frame, self.config.scorer, self.previous_track_id);
        self.previous_track_id = suggestion
            .target_index
            .and_then(|index| tracked_frame.objects.get(index))
            .and_then(|object| object.track_id);

        InferenceResult::new(
            tracked_frame.frame_id,
            latency_ms,
            tracked_frame.objects,
            suggestion,
        )
    }

    pub fn process_frame_with_assist_and_latency(
        &mut self,
        frame: &FrameRecord,
        latency_ms: f32,
    ) -> (InferenceResult, Option<AssistSuggestionEvent>) {
        let tracked_frame = self.tracked_frame(frame);
        let suggestion = choose_target(&tracked_frame, self.config.scorer, self.previous_track_id);
        self.previous_track_id = suggestion
            .target_index
            .and_then(|index| tracked_frame.objects.get(index))
            .and_then(|object| object.track_id);

        let assist_event = if tracked_frame.input.mouse_down && suggestion.suggested_point.is_some()
        {
            Some(AssistSuggestionEvent::left_mouse_review(
                tracked_frame.frame_id,
                suggestion,
            ))
        } else {
            None
        };

        let result = InferenceResult::new(
            tracked_frame.frame_id,
            latency_ms,
            tracked_frame.objects,
            suggestion,
        );

        (result, assist_event)
    }

    pub fn process_records(&mut self, records: &[FrameRecord]) -> Vec<InferenceResult> {
        records
            .iter()
            .map(|record| self.process_frame(record))
            .collect()
    }

    fn tracked_frame(&mut self, frame: &FrameRecord) -> FrameRecord {
        let mut tracked = frame.clone();
        self.tracker.assign(&mut tracked.objects);
        tracked
    }
}

impl Default for ReviewPipeline {
    fn default() -> Self {
        Self::new(RuntimeConfig::default())
    }
}

pub trait FrameSource {
    fn next_frame(&mut self) -> Result<Option<FrameRecord>, AutoAimError>;
}

#[derive(Debug)]
pub struct JsonlFrameSource {
    records: Vec<FrameRecord>,
    cursor: usize,
}

impl JsonlFrameSource {
    pub fn from_records(records: Vec<FrameRecord>) -> Self {
        Self { records, cursor: 0 }
    }
}

impl FrameSource for JsonlFrameSource {
    fn next_frame(&mut self) -> Result<Option<FrameRecord>, AutoAimError> {
        let Some(record) = self.records.get(self.cursor).cloned() else {
            return Ok(None);
        };

        self.cursor += 1;
        Ok(Some(record))
    }
}

pub struct JsonlEventWriter {
    writer: BufWriter<File>,
}

impl JsonlEventWriter {
    pub fn create(path: impl AsRef<Path>) -> Result<Self, AutoAimError> {
        let file = File::create(path)?;
        Ok(Self {
            writer: BufWriter::new(file),
        })
    }

    pub fn append(path: impl AsRef<Path>) -> Result<Self, AutoAimError> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            writer: BufWriter::new(file),
        })
    }

    pub fn write_inference_result(&mut self, result: &InferenceResult) -> Result<(), AutoAimError> {
        let line = encode_json_line(result)?;
        self.writer.write_all(line.as_bytes())?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), AutoAimError> {
        self.writer.flush()?;
        Ok(())
    }
}

pub struct DatasetLogger {
    writer: BufWriter<File>,
}

impl DatasetLogger {
    pub fn create(path: impl AsRef<Path>) -> Result<Self, AutoAimError> {
        let file = File::create(path)?;
        Ok(Self {
            writer: BufWriter::new(file),
        })
    }

    pub fn write_record(&mut self, record: &FrameRecord) -> Result<(), AutoAimError> {
        serde_json::to_writer(&mut self.writer, record)?;
        self.writer.write_all(b"\n")?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), AutoAimError> {
        self.writer.flush()?;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LiveDetectionInput {
    pub screen_origin: [i32; 2],
    pub screen_size: [u32; 2],
    pub frame_size: [u32; 2],
    pub cursor: Point,
    pub confidence_threshold: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LiveDetection {
    pub object_index: usize,
    pub bbox: [f32; 4],
    pub head_point: Point,
    pub confidence: f32,
    pub track_id: Option<u64>,
    pub dx: f32,
    pub dy: f32,
}

pub trait LiveDetector {
    fn detect(&self, input: &LiveDetectionInput) -> Vec<LiveDetection>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct MockNativeDetector;

impl LiveDetector for MockNativeDetector {
    fn detect(&self, input: &LiveDetectionInput) -> Vec<LiveDetection> {
        mock_native_detections(input)
    }
}

pub fn mock_native_detections(input: &LiveDetectionInput) -> Vec<LiveDetection> {
    if input.confidence_threshold > 0.90 {
        return Vec::new();
    }

    let [origin_x, origin_y] = input.screen_origin;
    let [screen_w, screen_h] = input.screen_size;
    let [_frame_w, _frame_h] = input.frame_size;
    let width = (screen_w as f32 * 0.14).clamp(72.0, 220.0);
    let height = (screen_h as f32 * 0.38).clamp(160.0, 520.0);
    let x = origin_x as f32 + screen_w as f32 * 0.5 - width / 2.0;
    let y = origin_y as f32 + screen_h as f32 * 0.42 - height / 2.0;
    let head_point = [x + width / 2.0, y + height * 0.18];

    vec![LiveDetection {
        object_index: 0,
        bbox: [x, y, width, height],
        head_point,
        confidence: 0.91,
        track_id: Some(1),
        dx: head_point[0] - input.cursor[0],
        dy: head_point[1] - input.cursor[1],
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use autoaim_core::{DetectionObject, FrameInput};

    #[test]
    fn pipeline_emits_inference_result_for_frame() {
        let frame = FrameRecord {
            frame_id: 7,
            timestamp_qpc: 100,
            image: "frame.jpg".to_string(),
            objects: vec![DetectionObject {
                class_name: "person".to_string(),
                bbox: [470.0, 260.0, 80.0, 220.0],
                head_bbox: None,
                head_point: Some([505.0, 295.0]),
                confidence: 0.75,
                track_id: Some(9),
            }],
            input: FrameInput {
                cursor: [500.0, 300.0],
                mouse_down: false,
            },
            resolution: None,
            window_handle: None,
            session_id: Some("session-001".to_string()),
            scene_id: None,
        };

        let mut pipeline = ReviewPipeline::default();
        let result = pipeline.process_frame_with_latency(&frame, 4.2);

        assert_eq!(result.frame_id, 7);
        assert_eq!(result.latency_ms, 4.2);
        assert_eq!(result.suggestion.suggested_point, Some([505.0, 295.0]));
    }

    #[test]
    fn pipeline_emits_review_only_assist_event_when_left_mouse_is_down() {
        let frame = FrameRecord {
            frame_id: 8,
            timestamp_qpc: 101,
            image: "frame.jpg".to_string(),
            objects: vec![DetectionObject {
                class_name: "person".to_string(),
                bbox: [470.0, 260.0, 80.0, 220.0],
                head_bbox: None,
                head_point: Some([505.0, 295.0]),
                confidence: 0.75,
                track_id: Some(9),
            }],
            input: FrameInput {
                cursor: [500.0, 300.0],
                mouse_down: true,
            },
            resolution: None,
            window_handle: None,
            session_id: Some("session-001".to_string()),
            scene_id: None,
        };

        let mut pipeline = ReviewPipeline::default();
        let (_result, assist) = pipeline.process_frame_with_assist_and_latency(&frame, 3.0);
        let assist = assist.expect("mouse down should create a review-only assist event");

        assert_eq!(assist.frame_id, 8);
        assert_eq!(assist.trigger, "mouse_left_down");
        assert!(assist.review_only);
        assert_eq!(assist.suggestion.suggested_point, Some([505.0, 295.0]));
    }

    #[test]
    fn pipeline_does_not_emit_assist_event_when_mouse_is_up() {
        let frame = FrameRecord {
            frame_id: 9,
            timestamp_qpc: 102,
            image: "frame.jpg".to_string(),
            objects: vec![DetectionObject {
                class_name: "person".to_string(),
                bbox: [470.0, 260.0, 80.0, 220.0],
                head_bbox: None,
                head_point: Some([505.0, 295.0]),
                confidence: 0.75,
                track_id: Some(9),
            }],
            input: FrameInput {
                cursor: [500.0, 300.0],
                mouse_down: false,
            },
            resolution: None,
            window_handle: None,
            session_id: Some("session-001".to_string()),
            scene_id: None,
        };

        let mut pipeline = ReviewPipeline::default();
        let (_result, assist) = pipeline.process_frame_with_assist(&frame);

        assert!(assist.is_none());
    }

    #[test]
    fn mock_native_detector_returns_screen_space_person() {
        let input = LiveDetectionInput {
            screen_origin: [100, 50],
            screen_size: [1920, 1080],
            frame_size: [960, 540],
            cursor: [960.0, 540.0],
            confidence_threshold: 0.25,
        };

        let detector = MockNativeDetector;
        let people = detector.detect(&input);

        assert_eq!(people.len(), 1);
        assert_eq!(people[0].object_index, 0);
        assert!(people[0].bbox[0] >= 100.0);
        assert!(people[0].head_point[1] >= 50.0);
        assert_eq!(people[0].track_id, Some(1));
    }

    #[test]
    fn mock_native_detector_honors_high_threshold() {
        let input = LiveDetectionInput {
            screen_origin: [0, 0],
            screen_size: [1920, 1080],
            frame_size: [960, 540],
            cursor: [960.0, 540.0],
            confidence_threshold: 0.95,
        };

        assert!(mock_native_detections(&input).is_empty());
    }
}
