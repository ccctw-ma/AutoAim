use autoaim_core::{AimSuggestion, DetectionObject, Point};
use serde::{Deserialize, Serialize};

pub const CAPTURE_CONTROL_PIPE: &str = r"\\.\pipe\autoaim.capture.control";
pub const INFERENCE_EVENTS_PIPE: &str = r"\\.\pipe\autoaim.inference.events";

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum MessageType {
    #[serde(rename = "capture.frame")]
    CaptureFrame,
    #[serde(rename = "inference.result")]
    InferenceResult,
    #[serde(rename = "assist.suggestion")]
    AssistSuggestion,
    #[serde(rename = "runtime.config")]
    RuntimeConfig,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum InferenceProvider {
    #[serde(rename = "cpu")]
    Cpu,
    #[serde(rename = "cuda")]
    Cuda,
    #[serde(rename = "tensorrt")]
    TensorRt,
    #[serde(rename = "directml")]
    DirectMl,
}

impl Default for InferenceProvider {
    fn default() -> Self {
        Self::Cpu
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct InferenceRuntimeConfig {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub provider: InferenceProvider,
    pub model_path: Option<String>,
    pub device_id: Option<u32>,
    pub confidence_threshold: f32,
    pub review_only: bool,
}

impl Default for InferenceRuntimeConfig {
    fn default() -> Self {
        Self {
            message_type: MessageType::RuntimeConfig,
            provider: InferenceProvider::Cpu,
            model_path: None,
            device_id: None,
            confidence_threshold: 0.25,
            review_only: true,
        }
    }
}

impl InferenceRuntimeConfig {
    pub fn new(
        provider: InferenceProvider,
        model_path: Option<String>,
        device_id: Option<u32>,
        confidence_threshold: f32,
    ) -> Self {
        Self {
            provider,
            model_path,
            device_id,
            confidence_threshold,
            ..Self::default()
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CaptureFrameMeta {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub frame_id: u64,
    pub timestamp_qpc: u64,
    pub resolution: [u32; 2],
    pub window_handle: String,
    pub cursor: Point,
    pub mouse_down: bool,
}

impl CaptureFrameMeta {
    pub fn new(
        frame_id: u64,
        timestamp_qpc: u64,
        resolution: [u32; 2],
        window_handle: impl Into<String>,
        cursor: Point,
        mouse_down: bool,
    ) -> Self {
        Self {
            message_type: MessageType::CaptureFrame,
            frame_id,
            timestamp_qpc,
            resolution,
            window_handle: window_handle.into(),
            cursor,
            mouse_down,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct RuntimeSuggestion {
    pub suggested_point: Option<Point>,
    pub dx: Option<f32>,
    pub dy: Option<f32>,
    pub score: f32,
}

impl From<AimSuggestion> for RuntimeSuggestion {
    fn from(value: AimSuggestion) -> Self {
        Self {
            suggested_point: value.suggested_point,
            dx: value.dx,
            dy: value.dy,
            score: value.score,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct InferenceResult {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub frame_id: u64,
    pub latency_ms: f32,
    pub objects: Vec<DetectionObject>,
    pub suggestion: RuntimeSuggestion,
}

impl InferenceResult {
    pub fn new(
        frame_id: u64,
        latency_ms: f32,
        objects: Vec<DetectionObject>,
        suggestion: AimSuggestion,
    ) -> Self {
        Self {
            message_type: MessageType::InferenceResult,
            frame_id,
            latency_ms,
            objects,
            suggestion: suggestion.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AssistSuggestionEvent {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub frame_id: u64,
    pub trigger: String,
    pub target_index: Option<usize>,
    pub confidence: f32,
    pub suggestion: RuntimeSuggestion,
    pub review_only: bool,
}

impl AssistSuggestionEvent {
    pub fn left_mouse_review(frame_id: u64, suggestion: AimSuggestion) -> Self {
        Self {
            message_type: MessageType::AssistSuggestion,
            frame_id,
            trigger: "mouse_left_down".to_string(),
            target_index: suggestion.target_index,
            confidence: suggestion.confidence,
            suggestion: suggestion.into(),
            review_only: true,
        }
    }
}

pub fn encode_json_line<T: Serialize>(message: &T) -> Result<String, serde_json::Error> {
    let mut line = serde_json::to_string(message)?;
    line.push('\n');
    Ok(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_frame_serializes_contract_type() {
        let message = CaptureFrameMeta::new(
            10231,
            123456789,
            [1280, 720],
            "0x0000000000120A4E",
            [512.0, 384.0],
            false,
        );

        let json = serde_json::to_value(message).unwrap();

        assert_eq!(json["type"], "capture.frame");
        assert_eq!(json["frame_id"], 10231);
    }

    #[test]
    fn inference_result_serializes_contract_type() {
        let suggestion = AimSuggestion {
            frame_id: 10231,
            suggested_point: Some([479.0, 211.0]),
            confidence: 0.91,
            target_index: Some(0),
            dx: Some(-33.0),
            dy: Some(-173.0),
            score: 0.82,
        };
        let message = InferenceResult::new(10231, 8.3, Vec::new(), suggestion);

        let json = serde_json::to_value(message).unwrap();

        assert_eq!(json["type"], "inference.result");
        assert_eq!(json["suggestion"]["suggested_point"][0], 479.0);
    }

    #[test]
    fn assist_suggestion_serializes_review_only_contract() {
        let suggestion = AimSuggestion {
            frame_id: 10231,
            suggested_point: Some([479.0, 211.0]),
            confidence: 0.91,
            target_index: Some(0),
            dx: Some(-33.0),
            dy: Some(-173.0),
            score: 0.82,
        };
        let message = AssistSuggestionEvent::left_mouse_review(10231, suggestion);

        let json = serde_json::to_value(message).unwrap();

        assert_eq!(json["type"], "assist.suggestion");
        assert_eq!(json["trigger"], "mouse_left_down");
        assert_eq!(json["review_only"], true);
        assert_eq!(json["suggestion"]["dx"], -33.0);
    }

    #[test]
    fn runtime_config_serializes_gpu_provider() {
        let config = InferenceRuntimeConfig::new(
            InferenceProvider::Cuda,
            Some("models/person_head.onnx".to_string()),
            Some(0),
            0.35,
        );

        let json = serde_json::to_value(config).unwrap();

        assert_eq!(json["type"], "runtime.config");
        assert_eq!(json["provider"], "cuda");
        assert_eq!(json["review_only"], true);
    }
}
