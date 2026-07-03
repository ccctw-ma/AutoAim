use autoaim_capture::CapturedFrame;
use autoaim_core::{DetectionObject, Point};
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

#[cfg(all(feature = "directml", feature = "movenet"))]
use std::sync::Mutex;

#[cfg(all(feature = "directml", feature = "movenet"))]
use ort::{ep, session::Session, value::Tensor as OrtTensor};

#[cfg(feature = "movenet")]
use tract_onnx::prelude::*;

pub const MOVENET_KEYPOINT_COUNT: usize = 17;
pub const MOVENET_LIGHTNING_INPUT_SIZE: u32 = 192;
pub const MOVENET_THUNDER_INPUT_SIZE: u32 = 256;
pub const MOVENET_KEYPOINT_NAMES: [&str; MOVENET_KEYPOINT_COUNT] = [
    "nose",
    "left_eye",
    "right_eye",
    "left_ear",
    "right_ear",
    "left_shoulder",
    "right_shoulder",
    "left_elbow",
    "right_elbow",
    "left_wrist",
    "right_wrist",
    "left_hip",
    "right_hip",
    "left_knee",
    "right_knee",
    "left_ankle",
    "right_ankle",
];

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum NativeInferenceProvider {
    #[serde(rename = "cpu")]
    Cpu,
    #[serde(rename = "cuda")]
    Cuda,
    #[serde(rename = "tensorrt")]
    TensorRt,
    #[serde(rename = "directml")]
    DirectMl,
}

impl Default for NativeInferenceProvider {
    fn default() -> Self {
        Self::Cpu
    }
}

impl NativeInferenceProvider {
    pub fn from_name(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "cpu" => Some(Self::Cpu),
            "cuda" => Some(Self::Cuda),
            "tensorrt" | "tensor_rt" | "tensor-rt" => Some(Self::TensorRt),
            "directml" | "direct_ml" | "direct-ml" => Some(Self::DirectMl),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Cuda => "cuda",
            Self::TensorRt => "tensorrt",
            Self::DirectMl => "directml",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct PoseKeypoint {
    pub index: usize,
    pub name: &'static str,
    pub point: Point,
    pub score: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct PoseEstimate {
    pub keypoints: Vec<PoseKeypoint>,
    pub bbox: [f32; 4],
    pub head_point: Point,
    pub confidence: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MoveNetInput {
    pub size: u32,
    pub rgb: Vec<f32>,
    pub transform: FrameToModelTransform,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameToModelTransform {
    pub frame_size: [u32; 2],
    pub screen_origin: [i32; 2],
    pub screen_size: [u32; 2],
    pub scale: f32,
    pub pad_x: f32,
    pub pad_y: f32,
}

impl FrameToModelTransform {
    pub fn model_point_to_screen(&self, model_x: f32, model_y: f32) -> Point {
        let frame_x = ((model_x - self.pad_x) / self.scale)
            .clamp(0.0, self.frame_size[0].saturating_sub(1) as f32);
        let frame_y = ((model_y - self.pad_y) / self.scale)
            .clamp(0.0, self.frame_size[1].saturating_sub(1) as f32);
        let screen_scale_x = self.screen_size[0] as f32 / self.frame_size[0].max(1) as f32;
        let screen_scale_y = self.screen_size[1] as f32 / self.frame_size[1].max(1) as f32;

        [
            self.screen_origin[0] as f32 + frame_x * screen_scale_x,
            self.screen_origin[1] as f32 + frame_y * screen_scale_y,
        ]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MoveNetRawOutput {
    pub values: Vec<f32>,
}

pub trait PoseModel: fmt::Debug {
    fn input_size(&self) -> u32;
    fn runtime_name(&self) -> &'static str {
        "custom pose model"
    }
    fn infer(&self, input: &MoveNetInput) -> Result<MoveNetRawOutput, InferenceError>;
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct InferenceConfig {
    pub provider: NativeInferenceProvider,
    pub model_path: Option<String>,
    pub confidence_threshold: f32,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            provider: NativeInferenceProvider::Cpu,
            model_path: None,
            confidence_threshold: 0.25,
        }
    }
}

impl InferenceConfig {
    pub fn new(
        provider: NativeInferenceProvider,
        model_path: Option<String>,
        confidence_threshold: f32,
    ) -> Self {
        Self {
            provider,
            model_path: model_path.filter(|value| !value.trim().is_empty()),
            confidence_threshold: confidence_threshold.clamp(0.0, 1.0),
        }
    }

    pub fn model_configured(&self) -> bool {
        self.model_path
            .as_ref()
            .map(|value| Path::new(value).is_file())
            .unwrap_or(false)
    }

    pub fn movenet_input_size(&self) -> u32 {
        self.model_path
            .as_deref()
            .map(|value| {
                if value.to_ascii_lowercase().contains("thunder") {
                    MOVENET_THUNDER_INPUT_SIZE
                } else {
                    MOVENET_LIGHTNING_INPUT_SIZE
                }
            })
            .unwrap_or(MOVENET_LIGHTNING_INPUT_SIZE)
    }

    pub fn model_path_buf(&self) -> Option<PathBuf> {
        self.model_path.as_ref().map(PathBuf::from)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct InferenceOutput {
    pub objects: Vec<DetectionObject>,
    pub poses: Vec<PoseEstimate>,
    pub provider: NativeInferenceProvider,
    pub model_status: String,
}

#[derive(Debug)]
pub enum InferenceError {
    InvalidFrame(&'static str),
    InvalidMoveNetOutput(String),
    ModelLoad(String),
    ModelRun(String),
}

impl fmt::Display for InferenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InferenceError::InvalidFrame(message) => formatter.write_str(message),
            InferenceError::InvalidMoveNetOutput(message) => formatter.write_str(message),
            InferenceError::ModelLoad(message) => {
                write!(formatter, "failed to load MoveNet: {message}")
            }
            InferenceError::ModelRun(message) => {
                write!(formatter, "failed to run MoveNet: {message}")
            }
        }
    }
}

impl Error for InferenceError {}

pub trait PersonDetector {
    fn detect(&self, frame: &CapturedFrame) -> Result<InferenceOutput, InferenceError>;
}

#[derive(Debug)]
pub struct NativePersonDetector {
    config: InferenceConfig,
    pose_model: Option<Box<dyn PoseModel>>,
}

impl NativePersonDetector {
    pub fn new(config: InferenceConfig) -> Self {
        Self {
            config,
            pose_model: None,
        }
    }

    #[cfg(test)]
    fn with_pose_model(config: InferenceConfig, pose_model: Box<dyn PoseModel>) -> Self {
        Self {
            config,
            pose_model: Some(pose_model),
        }
    }

    pub fn from_config(config: InferenceConfig) -> Result<Self, InferenceError> {
        let pose_model = load_pose_model(&config)?;
        Ok(Self { config, pose_model })
    }
}

impl PersonDetector for NativePersonDetector {
    fn detect(&self, frame: &CapturedFrame) -> Result<InferenceOutput, InferenceError> {
        if let Some(pose_model) = self.pose_model.as_deref() {
            let mut output =
                detect_movenet_poses(frame, pose_model, self.config.confidence_threshold)?;
            output.provider = self.config.provider;
            output.model_status = format!(
                "MoveNet {} backend active; requested provider {}; {} pose(s) and {} person object(s) produced",
                pose_model.runtime_name(),
                self.config.provider.as_str(),
                output.poses.len(),
                output.objects.len()
            );
            return Ok(output);
        }

        let objects = detect_visual_person_candidates(frame, self.config.confidence_threshold)?;
        let model_status = if self.config.model_configured() {
            format!(
                "{} model configured but MoveNet backend is unavailable; visual detector produced {} person candidate(s)",
                self.config.provider.as_str(),
                objects.len()
            )
        } else {
            format!(
                "native Windows frame capture is active; no model file configured, visual detector produced {} person candidate(s)",
                objects.len()
            )
        };

        Ok(InferenceOutput {
            objects,
            poses: Vec::new(),
            provider: self.config.provider,
            model_status,
        })
    }
}

fn load_pose_model(config: &InferenceConfig) -> Result<Option<Box<dyn PoseModel>>, InferenceError> {
    let Some(model_path) = config.model_path_buf().filter(|path| path.is_file()) else {
        return Ok(None);
    };

    #[cfg(feature = "movenet")]
    {
        #[cfg(feature = "directml")]
        if config.provider == NativeInferenceProvider::DirectMl
            && has_extension(&model_path, "onnx")
        {
            let model = OrtMoveNetModel::from_path(&model_path, config.movenet_input_size())?;
            return Ok(Some(Box::new(model)));
        }

        let model = TractMoveNetModel::from_path(model_path, config.movenet_input_size())?;
        Ok(Some(Box::new(model)))
    }

    #[cfg(not(feature = "movenet"))]
    {
        let _ = model_path;
        Err(InferenceError::ModelLoad(
            "binary was built without the movenet feature".to_string(),
        ))
    }
}

#[cfg(feature = "directml")]
fn has_extension(path: &Path, extension: &str) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case(extension))
        .unwrap_or(false)
}

#[cfg(feature = "movenet")]
#[derive(Debug)]
pub struct TractMoveNetModel {
    plan: Arc<TypedRunnableModel>,
    input_size: u32,
    input_kind: TractInputKind,
    runtime: TractRuntime,
}

#[cfg(feature = "movenet")]
#[derive(Clone, Copy, Debug, PartialEq)]
enum TractInputKind {
    F32,
    I32,
    U8,
}

#[cfg(feature = "movenet")]
#[derive(Clone, Copy, Debug, PartialEq)]
enum TractRuntime {
    OnnxCpu,
    TfliteCpu,
}

#[cfg(feature = "movenet")]
impl TractRuntime {
    fn name(self) -> &'static str {
        match self {
            Self::OnnxCpu => "tract-onnx CPU",
            Self::TfliteCpu => "tract-tflite CPU",
        }
    }
}

#[cfg(feature = "movenet")]
impl TractMoveNetModel {
    pub fn from_path(path: impl AsRef<Path>, input_size: u32) -> Result<Self, InferenceError> {
        let path = path.as_ref();
        let input_size_usize = input_size as usize;
        let is_tflite = path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.eq_ignore_ascii_case("tflite"))
            .unwrap_or(false);
        let (plan, input_kind, runtime) = if is_tflite {
            let model = tract_tflite::tflite()
                .model_for_path(path)
                .map_err(|error| InferenceError::ModelLoad(error.to_string()))?;
            let input_kind = TractInputKind::from_datum_type(
                model
                    .input_fact(0)
                    .map_err(|error| InferenceError::ModelLoad(error.to_string()))?
                    .datum_type,
            )?;
            let plan = model
                .into_optimized()
                .map_err(|error| InferenceError::ModelLoad(error.to_string()))?
                .into_runnable()
                .map_err(|error| InferenceError::ModelLoad(error.to_string()))?;
            (plan, input_kind, TractRuntime::TfliteCpu)
        } else {
            let input_fact = InferenceFact::dt_shape(
                f32::datum_type(),
                tvec![1, input_size_usize, input_size_usize, 3],
            );
            let plan = tract_onnx::onnx()
                .model_for_path(path)
                .map_err(|error| InferenceError::ModelLoad(error.to_string()))?
                .with_input_fact(0, input_fact)
                .map_err(|error| InferenceError::ModelLoad(error.to_string()))?
                .into_optimized()
                .map_err(|error| InferenceError::ModelLoad(error.to_string()))?
                .into_runnable()
                .map_err(|error| InferenceError::ModelLoad(error.to_string()))?;
            (plan, TractInputKind::F32, TractRuntime::OnnxCpu)
        };

        Ok(Self {
            plan,
            input_size,
            input_kind,
            runtime,
        })
    }
}

#[cfg(feature = "movenet")]
impl TractInputKind {
    fn from_datum_type(datum_type: DatumType) -> Result<Self, InferenceError> {
        if datum_type == f32::datum_type() {
            Ok(Self::F32)
        } else if datum_type == i32::datum_type() {
            Ok(Self::I32)
        } else if datum_type == u8::datum_type() {
            Ok(Self::U8)
        } else {
            Err(InferenceError::ModelLoad(format!(
                "unsupported MoveNet input datum type: {datum_type:?}"
            )))
        }
    }
}

#[cfg(feature = "movenet")]
impl PoseModel for TractMoveNetModel {
    fn input_size(&self) -> u32 {
        self.input_size
    }

    fn runtime_name(&self) -> &'static str {
        self.runtime.name()
    }

    fn infer(&self, input: &MoveNetInput) -> Result<MoveNetRawOutput, InferenceError> {
        let tensor = movenet_input_tensor(input, self.input_kind)?;
        let outputs = self
            .plan
            .run(tvec![tensor.into()])
            .map_err(|error| InferenceError::ModelRun(error.to_string()))?;
        let output = outputs.first().ok_or_else(|| {
            InferenceError::InvalidMoveNetOutput("model returned no output".to_string())
        })?;
        let values = output
            .to_plain_array_view::<f32>()
            .map_err(|error| InferenceError::InvalidMoveNetOutput(error.to_string()))?
            .iter()
            .copied()
            .collect();

        Ok(MoveNetRawOutput { values })
    }
}

#[cfg(all(feature = "directml", feature = "movenet"))]
struct OrtMoveNetModel {
    session: Mutex<Session>,
    input_size: u32,
}

#[cfg(all(feature = "directml", feature = "movenet"))]
impl fmt::Debug for OrtMoveNetModel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OrtMoveNetModel")
            .field("input_size", &self.input_size)
            .field("runtime", &self.runtime_name())
            .finish_non_exhaustive()
    }
}

#[cfg(all(feature = "directml", feature = "movenet"))]
impl OrtMoveNetModel {
    fn from_path(path: impl AsRef<Path>, input_size: u32) -> Result<Self, InferenceError> {
        let session = Session::builder()
            .map_err(|error| InferenceError::ModelLoad(error.to_string()))?
            .with_execution_providers([ep::DirectML::default().build()])
            .map_err(|error| InferenceError::ModelLoad(error.to_string()))?
            .commit_from_file(path.as_ref())
            .map_err(|error| InferenceError::ModelLoad(error.to_string()))?;

        Ok(Self {
            session: Mutex::new(session),
            input_size,
        })
    }
}

#[cfg(all(feature = "directml", feature = "movenet"))]
impl PoseModel for OrtMoveNetModel {
    fn input_size(&self) -> u32 {
        self.input_size
    }

    fn runtime_name(&self) -> &'static str {
        "onnxruntime DirectML"
    }

    fn infer(&self, input: &MoveNetInput) -> Result<MoveNetRawOutput, InferenceError> {
        let input_size = input.size as usize;
        let input_tensor = OrtTensor::from_array((
            [1_usize, input_size, input_size, 3_usize],
            input.rgb.clone(),
        ))
        .map_err(|error| InferenceError::ModelRun(error.to_string()))?;
        let mut session = self
            .session
            .lock()
            .map_err(|_| InferenceError::ModelRun("DirectML session lock poisoned".to_string()))?;
        let outputs = session
            .run(ort::inputs![input_tensor])
            .map_err(|error| InferenceError::ModelRun(error.to_string()))?;
        if outputs.len() == 0 {
            return Err(InferenceError::InvalidMoveNetOutput(
                "model returned no output".to_string(),
            ));
        }
        let output = &outputs[0];
        let (_shape, values) = output
            .try_extract_tensor::<f32>()
            .map_err(|error| InferenceError::InvalidMoveNetOutput(error.to_string()))?;

        Ok(MoveNetRawOutput {
            values: values.to_vec(),
        })
    }
}

#[cfg(feature = "movenet")]
fn movenet_input_tensor(
    input: &MoveNetInput,
    input_kind: TractInputKind,
) -> Result<Tensor, InferenceError> {
    let input_size = input.size as usize;
    match input_kind {
        TractInputKind::F32 => Tensor::from_shape(&[1, input_size, input_size, 3], &input.rgb)
            .map_err(|error| InferenceError::ModelRun(error.to_string())),
        TractInputKind::I32 => {
            let values = input
                .rgb
                .iter()
                .map(|value| (value * 255.0).round().clamp(0.0, 255.0) as i32)
                .collect::<Vec<_>>();
            Tensor::from_shape(&[1, input_size, input_size, 3], &values)
                .map_err(|error| InferenceError::ModelRun(error.to_string()))
        }
        TractInputKind::U8 => {
            let values = input
                .rgb
                .iter()
                .map(|value| (value * 255.0).round().clamp(0.0, 255.0) as u8)
                .collect::<Vec<_>>();
            Tensor::from_shape(&[1, input_size, input_size, 3], &values)
                .map_err(|error| InferenceError::ModelRun(error.to_string()))
        }
    }
}

pub fn detect_movenet_poses<M: PoseModel + ?Sized>(
    frame: &CapturedFrame,
    model: &M,
    threshold: f32,
) -> Result<InferenceOutput, InferenceError> {
    let input = prepare_movenet_input(frame, model.input_size())?;
    let raw = model.infer(&input)?;
    let pose = decode_movenet_output(&raw, &input.transform, input.size, threshold)?;
    let poses = pose.into_iter().collect::<Vec<_>>();
    let objects = poses
        .iter()
        .enumerate()
        .map(|(index, pose)| pose_to_detection_object(pose, index))
        .collect::<Vec<_>>();

    Ok(InferenceOutput {
        objects,
        poses,
        provider: NativeInferenceProvider::Cpu,
        model_status: String::new(),
    })
}

pub fn prepare_movenet_input(
    frame: &CapturedFrame,
    input_size: u32,
) -> Result<MoveNetInput, InferenceError> {
    let [frame_width, frame_height] = frame.frame_size;
    if input_size == 0
        || frame_width == 0
        || frame_height == 0
        || frame.rgba.len() != frame_width as usize * frame_height as usize * 4
    {
        return Err(InferenceError::InvalidFrame(
            "captured frame dimensions do not match RGBA buffer length",
        ));
    }

    let scale =
        (input_size as f32 / frame_width as f32).min(input_size as f32 / frame_height as f32);
    let resized_width = frame_width as f32 * scale;
    let resized_height = frame_height as f32 * scale;
    let pad_x = (input_size as f32 - resized_width) / 2.0;
    let pad_y = (input_size as f32 - resized_height) / 2.0;
    let mut rgb = vec![0.0_f32; input_size as usize * input_size as usize * 3];

    for dst_y in 0..input_size as usize {
        for dst_x in 0..input_size as usize {
            let src_x = ((dst_x as f32 + 0.5 - pad_x) / scale).floor();
            let src_y = ((dst_y as f32 + 0.5 - pad_y) / scale).floor();
            if src_x < 0.0
                || src_y < 0.0
                || src_x >= frame_width as f32
                || src_y >= frame_height as f32
            {
                continue;
            }

            let src_x = src_x as usize;
            let src_y = src_y as usize;
            let src_index = (src_y * frame_width as usize + src_x) * 4;
            let dst_index = (dst_y * input_size as usize + dst_x) * 3;
            rgb[dst_index] = frame.rgba[src_index] as f32 / 255.0;
            rgb[dst_index + 1] = frame.rgba[src_index + 1] as f32 / 255.0;
            rgb[dst_index + 2] = frame.rgba[src_index + 2] as f32 / 255.0;
        }
    }

    Ok(MoveNetInput {
        size: input_size,
        rgb,
        transform: FrameToModelTransform {
            frame_size: frame.frame_size,
            screen_origin: frame.screen_origin,
            screen_size: frame.screen_size,
            scale,
            pad_x,
            pad_y,
        },
    })
}

pub fn decode_movenet_output(
    raw: &MoveNetRawOutput,
    transform: &FrameToModelTransform,
    input_size: u32,
    threshold: f32,
) -> Result<Option<PoseEstimate>, InferenceError> {
    let expected_len = MOVENET_KEYPOINT_COUNT * 3;
    if raw.values.len() < expected_len {
        return Err(InferenceError::InvalidMoveNetOutput(format!(
            "MoveNet output has {} values, expected at least {expected_len}",
            raw.values.len()
        )));
    }

    let mut keypoints = Vec::with_capacity(MOVENET_KEYPOINT_COUNT);
    for index in 0..MOVENET_KEYPOINT_COUNT {
        let y = raw.values[index * 3].clamp(0.0, 1.0);
        let x = raw.values[index * 3 + 1].clamp(0.0, 1.0);
        let score = raw.values[index * 3 + 2].clamp(0.0, 1.0);
        let point = transform.model_point_to_screen(x * input_size as f32, y * input_size as f32);
        keypoints.push(PoseKeypoint {
            index,
            name: MOVENET_KEYPOINT_NAMES[index],
            point,
            score,
        });
    }

    Ok(pose_from_keypoints(keypoints, threshold))
}

fn pose_from_keypoints(keypoints: Vec<PoseKeypoint>, threshold: f32) -> Option<PoseEstimate> {
    let keypoint_threshold = threshold.clamp(0.15, 0.65);
    let visible = keypoints
        .iter()
        .filter(|keypoint| keypoint.score >= keypoint_threshold)
        .collect::<Vec<_>>();

    if visible.len() < 3 {
        return None;
    }

    let min_x = visible
        .iter()
        .map(|keypoint| keypoint.point[0])
        .fold(f32::INFINITY, f32::min);
    let min_y = visible
        .iter()
        .map(|keypoint| keypoint.point[1])
        .fold(f32::INFINITY, f32::min);
    let max_x = visible
        .iter()
        .map(|keypoint| keypoint.point[0])
        .fold(f32::NEG_INFINITY, f32::max);
    let max_y = visible
        .iter()
        .map(|keypoint| keypoint.point[1])
        .fold(f32::NEG_INFINITY, f32::max);
    let width = (max_x - min_x).max(1.0);
    let height = (max_y - min_y).max(1.0);
    let pad_x = (width * 0.12).max(8.0);
    let pad_y = (height * 0.12).max(8.0);
    let bbox = [
        min_x - pad_x,
        min_y - pad_y,
        width + pad_x * 2.0,
        height + pad_y * 2.0,
    ];
    let head_point = head_point_from_keypoints(&keypoints)
        .unwrap_or([bbox[0] + bbox[2] / 2.0, bbox[1] + bbox[3] * 0.18]);
    let confidence =
        visible.iter().map(|keypoint| keypoint.score).sum::<f32>() / visible.len() as f32;

    if confidence < threshold {
        return None;
    }

    Some(PoseEstimate {
        keypoints,
        bbox,
        head_point,
        confidence,
    })
}

fn head_point_from_keypoints(keypoints: &[PoseKeypoint]) -> Option<Point> {
    let head = keypoints
        .iter()
        .take(5)
        .filter(|keypoint| keypoint.score >= 0.20)
        .collect::<Vec<_>>();
    if head.is_empty() {
        return None;
    }

    let x = head.iter().map(|keypoint| keypoint.point[0]).sum::<f32>() / head.len() as f32;
    let y = head.iter().map(|keypoint| keypoint.point[1]).sum::<f32>() / head.len() as f32;
    Some([x, y])
}

fn pose_to_detection_object(pose: &PoseEstimate, index: usize) -> DetectionObject {
    DetectionObject {
        class_name: "person".to_string(),
        bbox: pose.bbox,
        head_bbox: None,
        head_point: Some(pose.head_point),
        confidence: pose.confidence,
        track_id: Some(index as u64 + 1),
    }
}

fn detect_visual_person_candidates(
    frame: &CapturedFrame,
    threshold: f32,
) -> Result<Vec<DetectionObject>, InferenceError> {
    let [frame_width, frame_height] = frame.frame_size;
    if frame_width == 0
        || frame_height == 0
        || frame.rgba.len() != frame_width as usize * frame_height as usize * 4
    {
        return Err(InferenceError::InvalidFrame(
            "captured frame dimensions do not match RGBA buffer length",
        ));
    }

    if threshold > 0.95 {
        return Ok(Vec::new());
    }

    let cell_size = 12_usize;
    let grid_width = (frame_width as usize).div_ceil(cell_size);
    let grid_height = (frame_height as usize).div_ceil(cell_size);
    let mut active = vec![false; grid_width * grid_height];

    for gy in 0..grid_height {
        for gx in 0..grid_width {
            let x0 = gx * cell_size;
            let y0 = gy * cell_size;
            let x1 = ((gx + 1) * cell_size).min(frame_width as usize - 1);
            let y1 = ((gy + 1) * cell_size).min(frame_height as usize - 1);
            let mut samples = 0_u32;
            let mut edge_hits = 0_u32;

            let mut y = y0;
            while y < y1 {
                let mut x = x0;
                while x < x1 {
                    let edge = local_edge_score(
                        &frame.rgba,
                        frame_width as usize,
                        frame_height as usize,
                        x,
                        y,
                    );
                    let brightness = pixel_brightness(&frame.rgba, frame_width as usize, x, y);
                    if edge > 75 && (18..=238).contains(&brightness) {
                        edge_hits += 1;
                    }
                    samples += 1;
                    x += 3;
                }
                y += 3;
            }

            if samples > 0 && edge_hits as f32 / samples as f32 > 0.18 {
                active[gy * grid_width + gx] = true;
            }
        }
    }

    let mut components = collect_components(&active, grid_width, grid_height);
    components.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let scale_x = frame.screen_size[0] as f32 / frame.frame_size[0] as f32;
    let scale_y = frame.screen_size[1] as f32 / frame.frame_size[1] as f32;
    let mut objects = Vec::new();

    for component in components.into_iter().take(8) {
        let width_cells = component.max_x - component.min_x + 1;
        let height_cells = component.max_y - component.min_y + 1;
        let component_width = (width_cells * cell_size) as f32;
        let component_height = (height_cells * cell_size) as f32;
        let area = component_width * component_height;
        let frame_area = (frame_width as f32 * frame_height as f32).max(1.0);
        let aspect = component_height / component_width.max(1.0);

        if component.cells < 3 || area / frame_area < 0.003 || !(1.05..=5.8).contains(&aspect) {
            continue;
        }

        let confidence = (0.30 + component.score * 0.58).clamp(0.0, 0.94);
        if confidence < threshold {
            continue;
        }

        let frame_x = (component.min_x * cell_size) as f32;
        let frame_y = (component.min_y * cell_size) as f32;
        let frame_w = component_width.min(frame_width as f32 - frame_x);
        let frame_h = component_height.min(frame_height as f32 - frame_y);
        let screen_x = frame.screen_origin[0] as f32 + frame_x * scale_x;
        let screen_y = frame.screen_origin[1] as f32 + frame_y * scale_y;
        let screen_w = frame_w * scale_x;
        let screen_h = frame_h * scale_y;

        objects.push(DetectionObject {
            class_name: "person".to_string(),
            bbox: [screen_x, screen_y, screen_w, screen_h],
            head_bbox: None,
            head_point: Some([screen_x + screen_w / 2.0, screen_y + screen_h * 0.18]),
            confidence,
            track_id: Some(objects.len() as u64 + 1),
        });
    }

    Ok(objects)
}

#[derive(Clone, Copy, Debug)]
struct Component {
    min_x: usize,
    min_y: usize,
    max_x: usize,
    max_y: usize,
    cells: usize,
    score: f32,
}

fn collect_components(active: &[bool], grid_width: usize, grid_height: usize) -> Vec<Component> {
    let mut visited = vec![false; active.len()];
    let mut components = Vec::new();

    for y in 0..grid_height {
        for x in 0..grid_width {
            let index = y * grid_width + x;
            if !active[index] || visited[index] {
                continue;
            }

            let mut stack = vec![(x, y)];
            let mut min_x = x;
            let mut max_x = x;
            let mut min_y = y;
            let mut max_y = y;
            let mut cells = 0_usize;
            visited[index] = true;

            while let Some((cx, cy)) = stack.pop() {
                cells += 1;
                min_x = min_x.min(cx);
                max_x = max_x.max(cx);
                min_y = min_y.min(cy);
                max_y = max_y.max(cy);

                let neighbors = [
                    (cx.wrapping_sub(1), cy, cx > 0),
                    (cx + 1, cy, cx + 1 < grid_width),
                    (cx, cy.wrapping_sub(1), cy > 0),
                    (cx, cy + 1, cy + 1 < grid_height),
                ];

                for (nx, ny, valid) in neighbors {
                    if !valid {
                        continue;
                    }
                    let neighbor_index = ny * grid_width + nx;
                    if active[neighbor_index] && !visited[neighbor_index] {
                        visited[neighbor_index] = true;
                        stack.push((nx, ny));
                    }
                }
            }

            let width = (max_x - min_x + 1) as f32;
            let height = (max_y - min_y + 1) as f32;
            let fill = cells as f32 / (width * height).max(1.0);
            let aspect = height / width.max(1.0);
            let aspect_score = (1.0 - ((aspect - 2.6).abs() / 2.6)).clamp(0.0, 1.0);
            let score = (fill * 0.55 + aspect_score * 0.45).clamp(0.0, 1.0);

            components.push(Component {
                min_x,
                min_y,
                max_x,
                max_y,
                cells,
                score,
            });
        }
    }

    components
}

fn local_edge_score(rgba: &[u8], width: usize, height: usize, x: usize, y: usize) -> u16 {
    if x + 1 >= width || y + 1 >= height {
        return 0;
    }

    let current = pixel_rgb(rgba, width, x, y);
    let right = pixel_rgb(rgba, width, x + 1, y);
    let down = pixel_rgb(rgba, width, x, y + 1);
    rgb_distance(current, right).max(rgb_distance(current, down))
}

fn pixel_brightness(rgba: &[u8], width: usize, x: usize, y: usize) -> u8 {
    let [r, g, b] = pixel_rgb(rgba, width, x, y);
    ((r as u16 * 30 + g as u16 * 59 + b as u16 * 11) / 100) as u8
}

fn pixel_rgb(rgba: &[u8], width: usize, x: usize, y: usize) -> [u8; 3] {
    let index = (y * width + x) * 4;
    [rgba[index], rgba[index + 1], rgba[index + 2]]
}

fn rgb_distance(left: [u8; 3], right: [u8; 3]) -> u16 {
    (left[0].abs_diff(right[0]) as u16)
        + (left[1].abs_diff(right[1]) as u16)
        + (left[2].abs_diff(right[2]) as u16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct MockPoseModel {
        input_size: u32,
        output: MoveNetRawOutput,
    }

    impl PoseModel for MockPoseModel {
        fn input_size(&self) -> u32 {
            self.input_size
        }

        fn infer(&self, input: &MoveNetInput) -> Result<MoveNetRawOutput, InferenceError> {
            assert_eq!(input.size, self.input_size);
            Ok(self.output.clone())
        }
    }

    fn frame_with_blob(width: u32, height: u32) -> CapturedFrame {
        let mut rgba = vec![20_u8; width as usize * height as usize * 4];
        for y in height as usize / 5..height as usize * 4 / 5 {
            for x in width as usize * 2 / 5..width as usize * 3 / 5 {
                let index = (y * width as usize + x) * 4;
                let value = if (x + y) % 2 == 0 { 230 } else { 55 };
                rgba[index] = value;
                rgba[index + 1] = value;
                rgba[index + 2] = value;
                rgba[index + 3] = 255;
            }
        }

        CapturedFrame {
            screen_origin: [100, 50],
            screen_size: [960, 720],
            frame_size: [width, height],
            rgba,
            cursor: [400.0, 300.0],
            cursor_on_screen: true,
            timestamp_millis: 1,
        }
    }

    fn movenet_raw_output(score: f32) -> MoveNetRawOutput {
        let mut values = Vec::with_capacity(MOVENET_KEYPOINT_COUNT * 3);
        for index in 0..MOVENET_KEYPOINT_COUNT {
            let row = index / 4;
            let col = index % 4;
            values.push((0.35 + row as f32 * 0.03).clamp(0.0, 1.0));
            values.push((0.40 + col as f32 * 0.04).clamp(0.0, 1.0));
            values.push(score);
        }
        MoveNetRawOutput { values }
    }

    #[test]
    fn provider_names_parse_and_format_all_supported_backends() {
        assert_eq!(
            NativeInferenceProvider::default(),
            NativeInferenceProvider::Cpu
        );
        assert_eq!(
            NativeInferenceProvider::from_name("cpu"),
            Some(NativeInferenceProvider::Cpu)
        );
        assert_eq!(
            NativeInferenceProvider::from_name("CUDA"),
            Some(NativeInferenceProvider::Cuda)
        );
        assert_eq!(
            NativeInferenceProvider::from_name("tensor-rt"),
            Some(NativeInferenceProvider::TensorRt)
        );
        assert_eq!(
            NativeInferenceProvider::from_name("direct_ml"),
            Some(NativeInferenceProvider::DirectMl)
        );
        assert_eq!(NativeInferenceProvider::from_name("unknown"), None);
        assert_eq!(NativeInferenceProvider::Cpu.as_str(), "cpu");
        assert_eq!(NativeInferenceProvider::Cuda.as_str(), "cuda");
        assert_eq!(NativeInferenceProvider::TensorRt.as_str(), "tensorrt");
        assert_eq!(NativeInferenceProvider::DirectMl.as_str(), "directml");
    }

    #[test]
    fn inference_config_clamps_threshold_and_selects_movenet_size() {
        let lightning = InferenceConfig::new(
            NativeInferenceProvider::Cpu,
            Some("models/movenet_lightning.onnx".to_string()),
            -1.0,
        );
        let thunder = InferenceConfig::new(
            NativeInferenceProvider::Cpu,
            Some("models/movenet_thunder.onnx".to_string()),
            2.0,
        );
        let empty_path =
            InferenceConfig::new(NativeInferenceProvider::Cpu, Some("".to_string()), 0.5);

        assert_eq!(lightning.confidence_threshold, 0.0);
        assert_eq!(lightning.movenet_input_size(), MOVENET_LIGHTNING_INPUT_SIZE);
        assert_eq!(thunder.confidence_threshold, 1.0);
        assert_eq!(thunder.movenet_input_size(), MOVENET_THUNDER_INPUT_SIZE);
        assert!(empty_path.model_path.is_none());
        assert!(InferenceConfig::default().model_path_buf().is_none());
    }

    #[test]
    fn detector_reports_movenet_status_when_pose_model_is_injected() {
        let frame = frame_with_blob(96, 72);
        let config = InferenceConfig::new(NativeInferenceProvider::Cpu, None, 0.25);
        let detector = NativePersonDetector::with_pose_model(
            config,
            Box::new(MockPoseModel {
                input_size: MOVENET_LIGHTNING_INPUT_SIZE,
                output: movenet_raw_output(0.90),
            }),
        );

        let output = detector.detect(&frame).unwrap();

        assert_eq!(output.objects.len(), 1);
        assert_eq!(output.poses.len(), 1);
        assert_eq!(output.provider, NativeInferenceProvider::Cpu);
        assert!(output
            .model_status
            .contains("MoveNet custom pose model backend active; requested provider cpu; 1 pose(s) and 1 person object(s) produced"));
    }

    #[test]
    fn inference_error_display_is_specific() {
        assert_eq!(
            InferenceError::InvalidFrame("bad frame").to_string(),
            "bad frame"
        );
        assert_eq!(
            InferenceError::InvalidMoveNetOutput("bad output".to_string()).to_string(),
            "bad output"
        );
        assert!(InferenceError::ModelLoad("x".to_string())
            .to_string()
            .contains("failed to load MoveNet"));
        assert!(InferenceError::ModelRun("x".to_string())
            .to_string()
            .contains("failed to run MoveNet"));
    }

    #[test]
    fn visual_detector_returns_screen_space_candidates() {
        let frame = frame_with_blob(240, 180);
        let detector = NativePersonDetector::new(InferenceConfig::default());
        let output = detector.detect(&frame).unwrap();

        assert!(!output.objects.is_empty());
        assert!(output.poses.is_empty());
        assert_eq!(output.objects[0].class_name, "person");
        assert!(output.objects[0].bbox[0] >= 100.0);
        assert!(output.objects[0].bbox[1] >= 50.0);
    }

    #[test]
    fn visual_detector_status_mentions_configured_model_when_backend_is_absent() {
        let frame = frame_with_blob(240, 180);
        let path = std::env::temp_dir().join(format!(
            "autoaim-configured-movenet-{}.tflite",
            std::process::id()
        ));
        std::fs::write(&path, b"placeholder model path").unwrap();
        let config = InferenceConfig::new(
            NativeInferenceProvider::Cpu,
            Some(path.to_string_lossy().to_string()),
            0.25,
        );
        let detector = NativePersonDetector::new(config);
        let output = detector.detect(&frame).unwrap();
        let _ = std::fs::remove_file(path);

        assert!(!output.objects.is_empty());
        assert!(output
            .model_status
            .contains("model configured but MoveNet backend is unavailable"));
    }

    #[test]
    fn visual_detector_handles_empty_high_threshold_and_invalid_frame() {
        let frame = frame_with_blob(80, 60);
        assert!(detect_visual_person_candidates(&frame, 0.99)
            .unwrap()
            .is_empty());

        let mut invalid = frame;
        invalid.rgba.truncate(3);
        assert!(detect_visual_person_candidates(&invalid, 0.25)
            .unwrap_err()
            .to_string()
            .contains("RGBA buffer"));
    }

    #[test]
    fn component_collection_groups_connected_cells() {
        let active = vec![
            true, true, false, false, false, true, false, true, false, false, false, true,
        ];
        let components = collect_components(&active, 4, 3);

        assert_eq!(components.len(), 2);
        assert!(components.iter().any(|component| component.cells == 3));
        assert!(components.iter().any(|component| component.cells == 2));
    }

    #[test]
    fn local_edge_score_and_brightness_cover_boundaries() {
        let rgba = vec![0, 0, 0, 255, 255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255];

        assert_eq!(local_edge_score(&rgba, 2, 2, 1, 1), 0);
        assert!(local_edge_score(&rgba, 2, 2, 0, 0) > 0);
        assert!(pixel_brightness(&rgba, 2, 1, 0) > 40);
        assert_eq!(rgb_distance([0, 0, 0], [1, 2, 3]), 6);
    }

    #[cfg(feature = "movenet")]
    #[test]
    fn tract_input_kind_maps_supported_types_and_builds_tensors() {
        assert_eq!(
            TractInputKind::from_datum_type(f32::datum_type()).unwrap(),
            TractInputKind::F32
        );
        assert_eq!(
            TractInputKind::from_datum_type(i32::datum_type()).unwrap(),
            TractInputKind::I32
        );
        assert_eq!(
            TractInputKind::from_datum_type(u8::datum_type()).unwrap(),
            TractInputKind::U8
        );
        assert!(matches!(
            TractInputKind::from_datum_type(i8::datum_type()),
            Err(InferenceError::ModelLoad(_))
        ));

        let input = MoveNetInput {
            size: 2,
            rgb: vec![
                -1.0, 0.0, 0.25, 0.5, 0.75, 1.0, 1.25, 0.4, 0.6, 0.8, 0.2, 0.1,
            ],
            transform: FrameToModelTransform {
                frame_size: [2, 2],
                screen_origin: [0, 0],
                screen_size: [2, 2],
                scale: 1.0,
                pad_x: 0.0,
                pad_y: 0.0,
            },
        };

        let f32_tensor = movenet_input_tensor(&input, TractInputKind::F32).unwrap();
        assert_eq!(f32_tensor.datum_type(), f32::datum_type());
        assert_eq!(f32_tensor.shape(), &[1, 2, 2, 3]);
        assert_eq!(
            f32_tensor
                .to_plain_array_view::<f32>()
                .unwrap()
                .iter()
                .copied()
                .collect::<Vec<_>>()[0],
            -1.0
        );

        let i32_values = movenet_input_tensor(&input, TractInputKind::I32)
            .unwrap()
            .to_plain_array_view::<i32>()
            .unwrap()
            .iter()
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(&i32_values[0..6], &[0, 0, 64, 128, 191, 255]);

        let u8_values = movenet_input_tensor(&input, TractInputKind::U8)
            .unwrap()
            .to_plain_array_view::<u8>()
            .unwrap()
            .iter()
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(&u8_values[0..6], &[0, 0, 64, 128, 191, 255]);
    }

    #[cfg(feature = "movenet")]
    #[test]
    fn configured_invalid_movenet_file_reports_model_load_error() {
        for extension in ["onnx", "tflite"] {
            let path = std::env::temp_dir().join(format!(
                "autoaim-invalid-movenet-{}.{}",
                std::process::id(),
                extension
            ));
            std::fs::write(&path, b"not a model file").unwrap();
            let config = InferenceConfig::new(
                NativeInferenceProvider::Cpu,
                Some(path.to_string_lossy().to_string()),
                0.25,
            );

            let error = NativePersonDetector::from_config(config).unwrap_err();
            let _ = std::fs::remove_file(path);

            assert!(matches!(error, InferenceError::ModelLoad(_)));
        }
    }
}
