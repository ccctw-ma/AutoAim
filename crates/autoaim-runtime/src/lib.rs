use autoaim_core::{choose_target, AutoAimError, FrameRecord, TargetScorer};
use autoaim_ipc::{encode_json_line, InferenceResult};
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
}

impl ReviewPipeline {
    pub fn new(config: RuntimeConfig) -> Self {
        Self {
            config,
            previous_track_id: None,
        }
    }

    pub fn process_frame(&mut self, frame: &FrameRecord) -> InferenceResult {
        self.process_frame_with_latency(frame, self.config.default_latency_ms)
    }

    pub fn process_frame_with_latency(
        &mut self,
        frame: &FrameRecord,
        latency_ms: f32,
    ) -> InferenceResult {
        let suggestion = choose_target(frame, self.config.scorer, self.previous_track_id);
        self.previous_track_id = suggestion
            .target_index
            .and_then(|index| frame.objects.get(index))
            .and_then(|object| object.track_id);

        InferenceResult::new(
            frame.frame_id,
            latency_ms,
            frame.objects.clone(),
            suggestion,
        )
    }

    pub fn process_records(&mut self, records: &[FrameRecord]) -> Vec<InferenceResult> {
        records
            .iter()
            .map(|record| self.process_frame(record))
            .collect()
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
}
