use autoaim_capture::CapturedFrame;
use autoaim_core::{DetectionObject, Point};
use serde::{Deserialize, Serialize};
use std::{
    error::Error,
    fmt,
    path::{Path, PathBuf},
    sync::Arc,
};

#[cfg(all(feature = "ort-backend", feature = "movenet"))]
use std::sync::Mutex;

#[cfg(all(feature = "ort-backend", feature = "movenet"))]
use ort::{ep, session::Session, value::Tensor as OrtTensor};

#[cfg(feature = "movenet")]
use tract_onnx::prelude::*;

pub const MOVENET_KEYPOINT_COUNT: usize = 17;
pub const MOVENET_LIGHTNING_INPUT_SIZE: u32 = 192;
pub const MOVENET_THUNDER_INPUT_SIZE: u32 = 256;
pub const YOLOV8_INPUT_SIZE: u32 = 640;
const YOLOV8_DETECT_OUTPUT_CHANNELS: usize = 84;
const YOLOV8_POSE_OUTPUT_CHANNELS: usize = 56;
const YOLOV8_POSE_KEYPOINT_OFFSET: usize = 5;
const YOLOV8_PERSON_CLASS_ID: usize = 0;
const YOLOV8_NMS_IOU_THRESHOLD: f32 = 0.50;
const YOLOV8_MAX_OBJECTS: usize = 32;
const YOLOV8_MAX_SCAN_REGIONS: usize = 8;
const MOVENET_MAX_SCAN_POSES: usize = 4;
const MOVENET_DUPLICATE_IOU_THRESHOLD: f32 = 0.35;
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
    pub source_origin: [f32; 2],
    pub source_size: [f32; 2],
    pub screen_origin: [i32; 2],
    pub screen_size: [u32; 2],
    pub scale: f32,
    pub pad_x: f32,
    pub pad_y: f32,
}

impl FrameToModelTransform {
    pub fn model_point_to_screen(&self, model_x: f32, model_y: f32) -> Point {
        let source_x =
            ((model_x - self.pad_x) / self.scale).clamp(0.0, (self.source_size[0] - 1.0).max(0.0));
        let source_y =
            ((model_y - self.pad_y) / self.scale).clamp(0.0, (self.source_size[1] - 1.0).max(0.0));
        let frame_x = (self.source_origin[0] + source_x)
            .clamp(0.0, self.frame_size[0].saturating_sub(1) as f32);
        let frame_y = (self.source_origin[1] + source_y)
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

pub trait PoseModel: fmt::Debug + Send {
    fn input_size(&self) -> u32;
    fn runtime_name(&self) -> &'static str {
        "custom pose model"
    }
    fn infer(&self, input: &MoveNetInput) -> Result<MoveNetRawOutput, InferenceError>;
}

trait ObjectModel: fmt::Debug + Send {
    fn input_size(&self) -> u32;
    fn runtime_name(&self) -> &'static str;
    fn infer(&self, input: &YoloV8Input) -> Result<YoloV8RawOutput, InferenceError>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct YoloV8Input {
    pub size: u32,
    pub rgb: Vec<f32>,
    pub transform: FrameToModelTransform,
}

#[derive(Clone, Debug, PartialEq)]
pub struct YoloV8RawOutput {
    pub shape: Vec<usize>,
    pub values: Vec<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct YoloV8DecodedOutput {
    pub objects: Vec<DetectionObject>,
    pub poses: Vec<PoseEstimate>,
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
    object_model: Option<Box<dyn ObjectModel>>,
    pose_model: Option<Box<dyn PoseModel>>,
}

impl NativePersonDetector {
    pub fn new(config: InferenceConfig) -> Self {
        Self {
            config,
            object_model: None,
            pose_model: None,
        }
    }

    #[cfg(test)]
    fn with_pose_model(config: InferenceConfig, pose_model: Box<dyn PoseModel>) -> Self {
        Self {
            config,
            object_model: None,
            pose_model: Some(pose_model),
        }
    }

    pub fn from_config(config: InferenceConfig) -> Result<Self, InferenceError> {
        let object_model = load_object_model(&config)?;
        let pose_model = load_pose_model(&config)?;
        Ok(Self {
            config,
            object_model,
            pose_model,
        })
    }
}

impl PersonDetector for NativePersonDetector {
    fn detect(&self, frame: &CapturedFrame) -> Result<InferenceOutput, InferenceError> {
        if let Some(object_model) = self.object_model.as_deref() {
            let mut output =
                detect_yolov8_people(frame, object_model, self.config.confidence_threshold)?;
            let scan_status = output.model_status.clone();
            output.provider = self.config.provider;
            output.model_status = format!(
                "YOLOv8 {} backend active; requested provider {}; {} pose(s) and {} person object(s) produced; {}",
                object_model.runtime_name(),
                self.config.provider.as_str(),
                output.poses.len(),
                output.objects.len(),
                scan_status
            );
            return Ok(output);
        }

        if let Some(pose_model) = self.pose_model.as_deref() {
            let mut output =
                detect_movenet_poses(frame, pose_model, self.config.confidence_threshold)?;
            let scan_status = output.model_status.clone();
            output.provider = self.config.provider;
            output.model_status = format!(
                "MoveNet {} backend active; requested provider {}; {} pose(s) and {} person object(s) produced; {}",
                pose_model.runtime_name(),
                self.config.provider.as_str(),
                output.poses.len(),
                output.objects.len(),
                scan_status
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

fn load_object_model(
    config: &InferenceConfig,
) -> Result<Option<Box<dyn ObjectModel>>, InferenceError> {
    let Some(model_path) = config.model_path_buf().filter(|path| path.is_file()) else {
        return Ok(None);
    };
    if !is_yolo_model_path(&model_path) {
        return Ok(None);
    }

    #[cfg(feature = "ort-backend")]
    {
        if is_ort_provider(config.provider) && has_extension(&model_path, "onnx") {
            let model = OrtYoloV8Model::from_path(&model_path, config.provider)?;
            return Ok(Some(Box::new(model)));
        }
    }

    Err(InferenceError::ModelLoad(
        "YOLOv8 requires an ONNX model and ONNX Runtime provider".to_string(),
    ))
}

fn is_yolo_model_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase().contains("yolo"))
        .unwrap_or(false)
}

fn load_pose_model(config: &InferenceConfig) -> Result<Option<Box<dyn PoseModel>>, InferenceError> {
    let Some(model_path) = config.model_path_buf().filter(|path| path.is_file()) else {
        return Ok(None);
    };

    #[cfg(feature = "movenet")]
    {
        #[cfg(feature = "ort-backend")]
        if is_ort_provider(config.provider) && has_extension(&model_path, "onnx") {
            let model = OrtMoveNetModel::from_path(
                &model_path,
                config.provider,
                config.movenet_input_size(),
            )?;
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

#[cfg(feature = "ort-backend")]
fn has_extension(path: &Path, extension: &str) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case(extension))
        .unwrap_or(false)
}

#[cfg(feature = "ort-backend")]
fn is_ort_provider(provider: NativeInferenceProvider) -> bool {
    matches!(
        provider,
        NativeInferenceProvider::DirectMl
            | NativeInferenceProvider::Cuda
            | NativeInferenceProvider::TensorRt
    )
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

#[cfg(all(feature = "ort-backend", feature = "movenet"))]
struct OrtMoveNetModel {
    session: Mutex<Session>,
    input_size: u32,
    provider: NativeInferenceProvider,
}

#[cfg(all(feature = "ort-backend", feature = "movenet"))]
impl fmt::Debug for OrtMoveNetModel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OrtMoveNetModel")
            .field("input_size", &self.input_size)
            .field("runtime", &self.runtime_name())
            .finish_non_exhaustive()
    }
}

#[cfg(all(feature = "ort-backend", feature = "movenet"))]
impl OrtMoveNetModel {
    fn from_path(
        path: impl AsRef<Path>,
        provider: NativeInferenceProvider,
        input_size: u32,
    ) -> Result<Self, InferenceError> {
        let execution_provider = ort_execution_provider(provider, path.as_ref());
        let session = Session::builder()
            .map_err(|error| InferenceError::ModelLoad(error.to_string()))?
            .with_execution_providers([execution_provider])
            .map_err(|error| InferenceError::ModelLoad(error.to_string()))?
            .commit_from_file(path.as_ref())
            .map_err(|error| InferenceError::ModelLoad(error.to_string()))?;

        Ok(Self {
            session: Mutex::new(session),
            input_size,
            provider,
        })
    }
}

#[cfg(all(feature = "ort-backend", feature = "movenet"))]
impl PoseModel for OrtMoveNetModel {
    fn input_size(&self) -> u32 {
        self.input_size
    }

    fn runtime_name(&self) -> &'static str {
        match self.provider {
            NativeInferenceProvider::DirectMl => "onnxruntime DirectML",
            NativeInferenceProvider::Cuda => "onnxruntime CUDA",
            NativeInferenceProvider::TensorRt => "onnxruntime TensorRT",
            NativeInferenceProvider::Cpu => "onnxruntime CPU",
        }
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

#[cfg(all(feature = "ort-backend", feature = "movenet"))]
struct OrtYoloV8Model {
    session: Mutex<Session>,
    provider: NativeInferenceProvider,
}

#[cfg(all(feature = "ort-backend", feature = "movenet"))]
impl fmt::Debug for OrtYoloV8Model {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OrtYoloV8Model")
            .field("runtime", &self.runtime_name())
            .finish_non_exhaustive()
    }
}

#[cfg(all(feature = "ort-backend", feature = "movenet"))]
impl OrtYoloV8Model {
    fn from_path(
        path: impl AsRef<Path>,
        provider: NativeInferenceProvider,
    ) -> Result<Self, InferenceError> {
        let execution_provider = ort_execution_provider(provider, path.as_ref());
        let session = Session::builder()
            .map_err(|error| InferenceError::ModelLoad(error.to_string()))?
            .with_execution_providers([execution_provider])
            .map_err(|error| InferenceError::ModelLoad(error.to_string()))?
            .commit_from_file(path.as_ref())
            .map_err(|error| InferenceError::ModelLoad(error.to_string()))?;

        Ok(Self {
            session: Mutex::new(session),
            provider,
        })
    }
}

#[cfg(all(feature = "ort-backend", feature = "movenet"))]
impl ObjectModel for OrtYoloV8Model {
    fn input_size(&self) -> u32 {
        YOLOV8_INPUT_SIZE
    }

    fn runtime_name(&self) -> &'static str {
        match self.provider {
            NativeInferenceProvider::DirectMl => "onnxruntime DirectML",
            NativeInferenceProvider::Cuda => "onnxruntime CUDA",
            NativeInferenceProvider::TensorRt => "onnxruntime TensorRT",
            NativeInferenceProvider::Cpu => "onnxruntime CPU",
        }
    }

    fn infer(&self, input: &YoloV8Input) -> Result<YoloV8RawOutput, InferenceError> {
        let input_size = input.size as usize;
        let input_tensor = OrtTensor::from_array((
            [1_usize, 3_usize, input_size, input_size],
            input.rgb.clone(),
        ))
        .map_err(|error| InferenceError::ModelRun(error.to_string()))?;
        let mut session = self
            .session
            .lock()
            .map_err(|_| InferenceError::ModelRun("YOLO session lock poisoned".to_string()))?;
        let outputs = session
            .run(ort::inputs![input_tensor])
            .map_err(|error| InferenceError::ModelRun(error.to_string()))?;
        if outputs.len() == 0 {
            return Err(InferenceError::InvalidMoveNetOutput(
                "YOLO model returned no output".to_string(),
            ));
        }
        let output = &outputs[0];
        let (shape, values) = output
            .try_extract_tensor::<f32>()
            .map_err(|error| InferenceError::InvalidMoveNetOutput(error.to_string()))?;
        let mut shape_dims = Vec::with_capacity(shape.len());
        for dim in shape.iter() {
            if *dim < 0 {
                return Err(InferenceError::InvalidMoveNetOutput(format!(
                    "YOLO output has dynamic dimension {dim}"
                )));
            }
            shape_dims.push(*dim as usize);
        }

        Ok(YoloV8RawOutput {
            shape: shape_dims,
            values: values.to_vec(),
        })
    }
}

#[cfg(all(feature = "ort-backend", feature = "movenet"))]
fn ort_execution_provider(
    provider: NativeInferenceProvider,
    model_path: &Path,
) -> ort::execution_providers::ExecutionProviderDispatch {
    match provider {
        NativeInferenceProvider::DirectMl => ep::DirectML::default().build(),
        NativeInferenceProvider::Cuda => ep::CUDA::default().with_device_id(0).build(),
        NativeInferenceProvider::TensorRt => {
            let cache_dir = model_path
                .parent()
                .map(|parent| parent.join("trt_cache"))
                .unwrap_or_else(|| PathBuf::from("trt_cache"));
            ep::TensorRT::default()
                .with_device_id(0)
                .with_fp16(true)
                .with_engine_cache(true)
                .with_engine_cache_path(cache_dir.to_string_lossy())
                .with_timing_cache(true)
                .with_timing_cache_path(cache_dir.to_string_lossy())
                .build()
        }
        NativeInferenceProvider::Cpu => unreachable!("CPU provider does not use ONNX Runtime EPs"),
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

fn detect_yolov8_people<M: ObjectModel + ?Sized>(
    frame: &CapturedFrame,
    model: &M,
    threshold: f32,
) -> Result<InferenceOutput, InferenceError> {
    let regions = yolov8_scan_regions(frame);
    let region_count = regions.len();
    let mut outputs = Vec::with_capacity(region_count);
    for region in regions {
        let input = prepare_yolov8_input_region(frame, model.input_size(), region)?;
        let raw = model.infer(&input)?;
        outputs.push(decode_yolov8_people_output(
            &raw,
            &input.transform,
            input.size,
            threshold,
        )?);
    }
    let decoded = merge_yolov8_decoded_outputs(outputs);
    Ok(InferenceOutput {
        objects: decoded.objects,
        poses: decoded.poses,
        provider: NativeInferenceProvider::Cpu,
        model_status: format!("scanned {region_count} YOLOv8 region(s)"),
    })
}

pub fn prepare_yolov8_input(
    frame: &CapturedFrame,
    input_size: u32,
) -> Result<YoloV8Input, InferenceError> {
    let [frame_width, frame_height] = frame.frame_size;
    prepare_yolov8_input_region(
        frame,
        input_size,
        YoloV8ScanRegion {
            origin: [0.0, 0.0],
            size: [frame_width as f32, frame_height as f32],
        },
    )
}

fn prepare_yolov8_input_region(
    frame: &CapturedFrame,
    input_size: u32,
    region: YoloV8ScanRegion,
) -> Result<YoloV8Input, InferenceError> {
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

    let source_width = region.size[0].clamp(1.0, frame_width as f32);
    let source_height = region.size[1].clamp(1.0, frame_height as f32);
    let source_origin_x = region.origin[0].clamp(0.0, frame_width.saturating_sub(1) as f32);
    let source_origin_y = region.origin[1].clamp(0.0, frame_height.saturating_sub(1) as f32);
    let max_source_width = frame_width as f32 - source_origin_x;
    let max_source_height = frame_height as f32 - source_origin_y;
    let source_width = source_width.min(max_source_width.max(1.0));
    let source_height = source_height.min(max_source_height.max(1.0));
    let scale = (input_size as f32 / source_width).min(input_size as f32 / source_height);
    let resized_width = source_width * scale;
    let resized_height = source_height * scale;
    let pad_x = (input_size as f32 - resized_width) / 2.0;
    let pad_y = (input_size as f32 - resized_height) / 2.0;
    let input_size = input_size as usize;
    let plane = input_size * input_size;
    let mut rgb = vec![114.0_f32 / 255.0; plane * 3];

    for dst_y in 0..input_size {
        for dst_x in 0..input_size {
            let src_x = source_origin_x + ((dst_x as f32 + 0.5 - pad_x) / scale).floor();
            let src_y = source_origin_y + ((dst_y as f32 + 0.5 - pad_y) / scale).floor();
            if src_x < 0.0
                || src_y < 0.0
                || src_x >= frame_width as f32
                || src_y >= frame_height as f32
                || src_x >= source_origin_x + source_width
                || src_y >= source_origin_y + source_height
            {
                continue;
            }

            let src_x = src_x as usize;
            let src_y = src_y as usize;
            let src_index = (src_y * frame_width as usize + src_x) * 4;
            let dst_index = dst_y * input_size + dst_x;
            rgb[dst_index] = frame.rgba[src_index] as f32 / 255.0;
            rgb[plane + dst_index] = frame.rgba[src_index + 1] as f32 / 255.0;
            rgb[plane * 2 + dst_index] = frame.rgba[src_index + 2] as f32 / 255.0;
        }
    }

    Ok(YoloV8Input {
        size: input_size as u32,
        rgb,
        transform: FrameToModelTransform {
            frame_size: frame.frame_size,
            source_origin: [source_origin_x, source_origin_y],
            source_size: [source_width, source_height],
            screen_origin: frame.screen_origin,
            screen_size: frame.screen_size,
            scale,
            pad_x,
            pad_y,
        },
    })
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct YoloV8ScanRegion {
    origin: [f32; 2],
    size: [f32; 2],
}

fn yolov8_scan_regions(frame: &CapturedFrame) -> Vec<YoloV8ScanRegion> {
    let [frame_width, frame_height] = frame.frame_size;
    let width = frame_width as f32;
    let height = frame_height as f32;
    let mut regions = Vec::new();
    push_yolov8_region(&mut regions, width, height, 0.0, 0.0, width, height);
    push_center_yolov8_region(&mut regions, width, height, 0.70, 0.70);
    push_yolov8_grid(&mut regions, width, height, 3, 2, 0.46, 0.62);
    regions.truncate(YOLOV8_MAX_SCAN_REGIONS);
    regions
}

fn push_center_yolov8_region(
    regions: &mut Vec<YoloV8ScanRegion>,
    frame_width: f32,
    frame_height: f32,
    width_ratio: f32,
    height_ratio: f32,
) {
    let width = frame_width * width_ratio;
    let height = frame_height * height_ratio;
    let origin_x = (frame_width - width) / 2.0;
    let origin_y = (frame_height - height) / 2.0;
    push_yolov8_region(
        regions,
        frame_width,
        frame_height,
        origin_x,
        origin_y,
        width,
        height,
    );
}

fn push_yolov8_grid(
    regions: &mut Vec<YoloV8ScanRegion>,
    frame_width: f32,
    frame_height: f32,
    columns: usize,
    rows: usize,
    width_ratio: f32,
    height_ratio: f32,
) {
    let width = frame_width * width_ratio;
    let height = frame_height * height_ratio;
    let max_x = (frame_width - width).max(0.0);
    let max_y = (frame_height - height).max(0.0);
    for row in 0..rows {
        let y = if rows <= 1 {
            max_y / 2.0
        } else {
            max_y * row as f32 / (rows - 1) as f32
        };
        for column in 0..columns {
            let x = if columns <= 1 {
                max_x / 2.0
            } else {
                max_x * column as f32 / (columns - 1) as f32
            };
            push_yolov8_region(regions, frame_width, frame_height, x, y, width, height);
        }
    }
}

fn push_yolov8_region(
    regions: &mut Vec<YoloV8ScanRegion>,
    frame_width: f32,
    frame_height: f32,
    origin_x: f32,
    origin_y: f32,
    width: f32,
    height: f32,
) {
    if width < 8.0 || height < 8.0 {
        return;
    }
    let origin_x = origin_x.clamp(0.0, (frame_width - 1.0).max(0.0));
    let origin_y = origin_y.clamp(0.0, (frame_height - 1.0).max(0.0));
    let width = width.min(frame_width - origin_x).max(1.0);
    let height = height.min(frame_height - origin_y).max(1.0);
    let duplicate = regions.iter().any(|existing| {
        (existing.origin[0] - origin_x).abs() < 1.0
            && (existing.origin[1] - origin_y).abs() < 1.0
            && (existing.size[0] - width).abs() < 1.0
            && (existing.size[1] - height).abs() < 1.0
    });
    if !duplicate {
        regions.push(YoloV8ScanRegion {
            origin: [origin_x, origin_y],
            size: [width, height],
        });
    }
}

pub fn decode_yolov8_people_output(
    raw: &YoloV8RawOutput,
    transform: &FrameToModelTransform,
    input_size: u32,
    threshold: f32,
) -> Result<YoloV8DecodedOutput, InferenceError> {
    let layout = yolo_output_layout(raw)?;
    let threshold = threshold.clamp(0.01, 0.95);
    let mut candidates = Vec::new();
    for anchor in 0..layout.anchors {
        let confidence_channel = match layout.kind {
            YoloV8OutputKind::Detect => 4 + YOLOV8_PERSON_CLASS_ID,
            YoloV8OutputKind::Pose => 4,
        };
        let confidence = yolo_value(raw, layout, confidence_channel, anchor).clamp(0.0, 1.0);
        if confidence < threshold {
            continue;
        }
        let center_x = yolo_value(raw, layout, 0, anchor).clamp(0.0, input_size as f32);
        let center_y = yolo_value(raw, layout, 1, anchor).clamp(0.0, input_size as f32);
        let width = yolo_value(raw, layout, 2, anchor).max(1.0);
        let height = yolo_value(raw, layout, 3, anchor).max(1.0);
        let top_left =
            transform.model_point_to_screen(center_x - width / 2.0, center_y - height / 2.0);
        let bottom_right =
            transform.model_point_to_screen(center_x + width / 2.0, center_y + height / 2.0);
        let x1 = top_left[0].min(bottom_right[0]);
        let y1 = top_left[1].min(bottom_right[1]);
        let x2 = top_left[0].max(bottom_right[0]);
        let y2 = top_left[1].max(bottom_right[1]);
        let bbox = [x1, y1, (x2 - x1).max(1.0), (y2 - y1).max(1.0)];
        let object = DetectionObject {
            class_name: "person".to_string(),
            bbox,
            head_bbox: None,
            head_point: Some([bbox[0] + bbox[2] * 0.5, bbox[1] + bbox[3] * 0.18]),
            confidence,
            track_id: None,
        };
        let pose = if layout.kind == YoloV8OutputKind::Pose {
            Some(yolov8_pose_from_anchor(
                raw, layout, anchor, transform, input_size, bbox, confidence,
            ))
        } else {
            None
        };
        candidates.push(YoloV8Candidate { object, pose });
    }

    Ok(select_yolov8_candidates(candidates))
}

fn merge_yolov8_decoded_outputs(outputs: Vec<YoloV8DecodedOutput>) -> YoloV8DecodedOutput {
    let mut candidates = Vec::new();
    for output in outputs {
        let poses = output.poses;
        for (index, mut object) in output.objects.into_iter().enumerate() {
            object.track_id = None;
            candidates.push(YoloV8Candidate {
                object,
                pose: poses.get(index).cloned(),
            });
        }
    }
    select_yolov8_candidates(candidates)
}

fn select_yolov8_candidates(mut candidates: Vec<YoloV8Candidate>) -> YoloV8DecodedOutput {
    candidates.sort_by(|left, right| {
        right
            .object
            .confidence
            .partial_cmp(&left.object.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut selected = Vec::new();
    for candidate in candidates {
        if selected.iter().any(|existing: &YoloV8Candidate| {
            bbox_iou(existing.object.bbox, candidate.object.bbox) >= YOLOV8_NMS_IOU_THRESHOLD
        }) {
            continue;
        }
        selected.push(candidate);
        if selected.len() >= YOLOV8_MAX_OBJECTS {
            break;
        }
    }
    for (index, candidate) in selected.iter_mut().enumerate() {
        candidate.object.track_id = Some(index as u64 + 1);
    }
    let mut objects = Vec::with_capacity(selected.len());
    let mut poses = Vec::new();
    for candidate in selected {
        objects.push(candidate.object);
        if let Some(pose) = candidate.pose {
            poses.push(pose);
        }
    }
    YoloV8DecodedOutput { objects, poses }
}

#[derive(Clone, Debug, PartialEq)]
struct YoloV8Candidate {
    object: DetectionObject,
    pose: Option<PoseEstimate>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum YoloV8OutputKind {
    Detect,
    Pose,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct YoloV8OutputLayout {
    kind: YoloV8OutputKind,
    channels: usize,
    anchors: usize,
    channels_first: bool,
}

fn yolo_output_layout(raw: &YoloV8RawOutput) -> Result<YoloV8OutputLayout, InferenceError> {
    let values_len = raw.values.len();
    if values_len == 0 {
        return Err(InferenceError::InvalidMoveNetOutput(
            "YOLOv8 output is empty".to_string(),
        ));
    }

    let dims = raw
        .shape
        .iter()
        .copied()
        .filter(|dim| *dim > 1)
        .collect::<Vec<_>>();
    for pair in dims.windows(2).rev() {
        if let Some(layout) = yolo_layout_from_dims(pair[0], pair[1], values_len) {
            return Ok(layout);
        }
    }

    let detect_match = values_len % YOLOV8_DETECT_OUTPUT_CHANNELS == 0;
    let pose_match = values_len % YOLOV8_POSE_OUTPUT_CHANNELS == 0;
    match (detect_match, pose_match) {
        (true, false) => Ok(YoloV8OutputLayout {
            kind: YoloV8OutputKind::Detect,
            channels: YOLOV8_DETECT_OUTPUT_CHANNELS,
            anchors: values_len / YOLOV8_DETECT_OUTPUT_CHANNELS,
            channels_first: true,
        }),
        (false, true) => Ok(YoloV8OutputLayout {
            kind: YoloV8OutputKind::Pose,
            channels: YOLOV8_POSE_OUTPUT_CHANNELS,
            anchors: values_len / YOLOV8_POSE_OUTPUT_CHANNELS,
            channels_first: true,
        }),
        _ => Err(InferenceError::InvalidMoveNetOutput(format!(
            "YOLOv8 output has shape {:?} and {} values; expected 84-channel detect or 56-channel pose output",
            raw.shape, values_len
        ))),
    }
}

fn yolo_layout_from_dims(
    left: usize,
    right: usize,
    values_len: usize,
) -> Option<YoloV8OutputLayout> {
    if left.checked_mul(right)? != values_len {
        return None;
    }
    if let Some(kind) = yolo_kind_from_channels(left) {
        return Some(YoloV8OutputLayout {
            kind,
            channels: left,
            anchors: right,
            channels_first: true,
        });
    }
    if let Some(kind) = yolo_kind_from_channels(right) {
        return Some(YoloV8OutputLayout {
            kind,
            channels: right,
            anchors: left,
            channels_first: false,
        });
    }
    None
}

fn yolo_kind_from_channels(channels: usize) -> Option<YoloV8OutputKind> {
    match channels {
        YOLOV8_DETECT_OUTPUT_CHANNELS => Some(YoloV8OutputKind::Detect),
        YOLOV8_POSE_OUTPUT_CHANNELS => Some(YoloV8OutputKind::Pose),
        _ => None,
    }
}

fn yolo_value(
    raw: &YoloV8RawOutput,
    layout: YoloV8OutputLayout,
    channel: usize,
    anchor: usize,
) -> f32 {
    if layout.channels_first {
        raw.values[channel * layout.anchors + anchor]
    } else {
        raw.values[anchor * layout.channels + channel]
    }
}

fn yolov8_pose_from_anchor(
    raw: &YoloV8RawOutput,
    layout: YoloV8OutputLayout,
    anchor: usize,
    transform: &FrameToModelTransform,
    input_size: u32,
    bbox: [f32; 4],
    confidence: f32,
) -> PoseEstimate {
    let mut keypoints = Vec::with_capacity(MOVENET_KEYPOINT_COUNT);
    for index in 0..MOVENET_KEYPOINT_COUNT {
        let offset = YOLOV8_POSE_KEYPOINT_OFFSET + index * 3;
        let x = yolo_value(raw, layout, offset, anchor).clamp(0.0, input_size as f32);
        let y = yolo_value(raw, layout, offset + 1, anchor).clamp(0.0, input_size as f32);
        let score = yolo_value(raw, layout, offset + 2, anchor).clamp(0.0, 1.0);
        keypoints.push(PoseKeypoint {
            index,
            name: MOVENET_KEYPOINT_NAMES[index],
            point: transform.model_point_to_screen(x, y),
            score,
        });
    }
    let head_point = head_point_from_keypoints(&keypoints)
        .unwrap_or([bbox[0] + bbox[2] * 0.5, bbox[1] + bbox[3] * 0.18]);
    PoseEstimate {
        keypoints,
        bbox,
        head_point,
        confidence,
    }
}

pub fn detect_movenet_poses<M: PoseModel + ?Sized>(
    frame: &CapturedFrame,
    model: &M,
    threshold: f32,
) -> Result<InferenceOutput, InferenceError> {
    let regions = movenet_scan_regions(frame);
    let region_count = regions.len();
    let mut poses = Vec::new();
    for region in regions {
        let input = prepare_movenet_input_region(frame, model.input_size(), region)?;
        let raw = model.infer(&input)?;
        if let Some(pose) = decode_movenet_output(&raw, &input.transform, input.size, threshold)? {
            poses.push(pose);
        }
    }
    let poses = dedupe_movenet_poses(poses);
    let objects = poses
        .iter()
        .enumerate()
        .map(|(index, pose)| pose_to_detection_object(pose, index))
        .collect::<Vec<_>>();

    Ok(InferenceOutput {
        objects,
        poses,
        provider: NativeInferenceProvider::Cpu,
        model_status: format!("scanned {region_count} MoveNet region(s)"),
    })
}

pub fn prepare_movenet_input(
    frame: &CapturedFrame,
    input_size: u32,
) -> Result<MoveNetInput, InferenceError> {
    let [frame_width, frame_height] = frame.frame_size;
    prepare_movenet_input_region(
        frame,
        input_size,
        MoveNetScanRegion {
            origin: [0.0, 0.0],
            size: [frame_width as f32, frame_height as f32],
        },
    )
}

fn prepare_movenet_input_region(
    frame: &CapturedFrame,
    input_size: u32,
    region: MoveNetScanRegion,
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

    let source_width = region.size[0].clamp(1.0, frame_width as f32);
    let source_height = region.size[1].clamp(1.0, frame_height as f32);
    let source_origin_x = region.origin[0].clamp(0.0, frame_width.saturating_sub(1) as f32);
    let source_origin_y = region.origin[1].clamp(0.0, frame_height.saturating_sub(1) as f32);
    let max_source_width = frame_width as f32 - source_origin_x;
    let max_source_height = frame_height as f32 - source_origin_y;
    let source_width = source_width.min(max_source_width.max(1.0));
    let source_height = source_height.min(max_source_height.max(1.0));
    let scale = (input_size as f32 / source_width).min(input_size as f32 / source_height);
    let resized_width = source_width * scale;
    let resized_height = source_height * scale;
    let pad_x = (input_size as f32 - resized_width) / 2.0;
    let pad_y = (input_size as f32 - resized_height) / 2.0;
    let mut rgb = vec![0.0_f32; input_size as usize * input_size as usize * 3];

    for dst_y in 0..input_size as usize {
        for dst_x in 0..input_size as usize {
            let src_x = source_origin_x + ((dst_x as f32 + 0.5 - pad_x) / scale).floor();
            let src_y = source_origin_y + ((dst_y as f32 + 0.5 - pad_y) / scale).floor();
            if src_x < 0.0
                || src_y < 0.0
                || src_x >= frame_width as f32
                || src_y >= frame_height as f32
                || src_x >= source_origin_x + source_width
                || src_y >= source_origin_y + source_height
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
            source_origin: [source_origin_x, source_origin_y],
            source_size: [source_width, source_height],
            screen_origin: frame.screen_origin,
            screen_size: frame.screen_size,
            scale,
            pad_x,
            pad_y,
        },
    })
}

#[derive(Clone, Copy, Debug)]
struct MoveNetScanRegion {
    origin: [f32; 2],
    size: [f32; 2],
}

fn movenet_scan_regions(frame: &CapturedFrame) -> Vec<MoveNetScanRegion> {
    let [frame_width, frame_height] = frame.frame_size;
    let width = frame_width as f32;
    let height = frame_height as f32;
    let mut regions = Vec::new();
    push_movenet_region(&mut regions, width, height, 0.0, 0.0, width, height);
    push_center_movenet_region(&mut regions, width, height, 0.72, 0.72);
    push_center_movenet_region(&mut regions, width, height, 0.46, 0.56);
    push_movenet_grid(&mut regions, width, height, 2, 2, 0.64, 0.64);
    push_movenet_grid(&mut regions, width, height, 3, 2, 0.48, 0.62);
    regions
}

fn push_center_movenet_region(
    regions: &mut Vec<MoveNetScanRegion>,
    frame_width: f32,
    frame_height: f32,
    width_ratio: f32,
    height_ratio: f32,
) {
    let width = frame_width * width_ratio;
    let height = frame_height * height_ratio;
    let origin_x = (frame_width - width) / 2.0;
    let origin_y = (frame_height - height) / 2.0;
    push_movenet_region(
        regions,
        frame_width,
        frame_height,
        origin_x,
        origin_y,
        width,
        height,
    );
}

fn push_movenet_grid(
    regions: &mut Vec<MoveNetScanRegion>,
    frame_width: f32,
    frame_height: f32,
    columns: usize,
    rows: usize,
    width_ratio: f32,
    height_ratio: f32,
) {
    let width = frame_width * width_ratio;
    let height = frame_height * height_ratio;
    let max_x = (frame_width - width).max(0.0);
    let max_y = (frame_height - height).max(0.0);
    for row in 0..rows {
        let y = if rows <= 1 {
            max_y / 2.0
        } else {
            max_y * row as f32 / (rows - 1) as f32
        };
        for column in 0..columns {
            let x = if columns <= 1 {
                max_x / 2.0
            } else {
                max_x * column as f32 / (columns - 1) as f32
            };
            push_movenet_region(regions, frame_width, frame_height, x, y, width, height);
        }
    }
}

fn push_movenet_region(
    regions: &mut Vec<MoveNetScanRegion>,
    frame_width: f32,
    frame_height: f32,
    origin_x: f32,
    origin_y: f32,
    width: f32,
    height: f32,
) {
    if width < 8.0 || height < 8.0 {
        return;
    }
    let origin_x = origin_x.clamp(0.0, (frame_width - 1.0).max(0.0));
    let origin_y = origin_y.clamp(0.0, (frame_height - 1.0).max(0.0));
    let width = width.min(frame_width - origin_x).max(1.0);
    let height = height.min(frame_height - origin_y).max(1.0);
    let duplicate = regions.iter().any(|existing| {
        (existing.origin[0] - origin_x).abs() < 1.0
            && (existing.origin[1] - origin_y).abs() < 1.0
            && (existing.size[0] - width).abs() < 1.0
            && (existing.size[1] - height).abs() < 1.0
    });
    if !duplicate {
        regions.push(MoveNetScanRegion {
            origin: [origin_x, origin_y],
            size: [width, height],
        });
    }
}

fn dedupe_movenet_poses(mut poses: Vec<PoseEstimate>) -> Vec<PoseEstimate> {
    poses.sort_by(|left, right| {
        right
            .confidence
            .partial_cmp(&left.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut selected: Vec<PoseEstimate> = Vec::new();
    for pose in poses {
        if selected
            .iter()
            .any(|existing| bbox_iou(existing.bbox, pose.bbox) >= MOVENET_DUPLICATE_IOU_THRESHOLD)
        {
            continue;
        }
        selected.push(pose);
        if selected.len() >= MOVENET_MAX_SCAN_POSES {
            break;
        }
    }
    selected
}

fn bbox_iou(left: [f32; 4], right: [f32; 4]) -> f32 {
    let left_x2 = left[0] + left[2];
    let left_y2 = left[1] + left[3];
    let right_x2 = right[0] + right[2];
    let right_y2 = right[1] + right[3];
    let intersection_width = (left_x2.min(right_x2) - left[0].max(right[0])).max(0.0);
    let intersection_height = (left_y2.min(right_y2) - left[1].max(right[1])).max(0.0);
    let intersection = intersection_width * intersection_height;
    let left_area = (left[2] * left[3]).max(0.0);
    let right_area = (right[2] * right[3]).max(0.0);
    let union = left_area + right_area - intersection;
    if union <= 0.0 {
        0.0
    } else {
        intersection / union
    }
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
            capture_backend: autoaim_capture::CaptureBackend::Gdi,
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

    fn identity_yolo_transform(input_size: u32) -> FrameToModelTransform {
        FrameToModelTransform {
            frame_size: [input_size, input_size],
            source_origin: [0.0, 0.0],
            source_size: [input_size as f32, input_size as f32],
            screen_origin: [0, 0],
            screen_size: [input_size, input_size],
            scale: 1.0,
            pad_x: 0.0,
            pad_y: 0.0,
        }
    }

    fn assert_near(left: f32, right: f32) {
        assert!(
            (left - right).abs() < 0.01,
            "expected {left} to be near {right}"
        );
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

        assert!(!output.objects.is_empty());
        assert!(output.objects.len() <= MOVENET_MAX_SCAN_POSES);
        assert_eq!(output.objects.len(), output.poses.len());
        assert_eq!(output.provider, NativeInferenceProvider::Cpu);
        assert!(output
            .model_status
            .contains("MoveNet custom pose model backend active; requested provider cpu"));
        assert!(output.model_status.contains("scanned"));
    }

    #[test]
    fn yolov8_detect_output_decodes_person_objects_from_shape() {
        let anchor_count = 2;
        let mut values = vec![0.0; YOLOV8_DETECT_OUTPUT_CHANNELS * anchor_count];
        values[0] = 50.0;
        values[anchor_count] = 60.0;
        values[anchor_count * 2] = 20.0;
        values[anchor_count * 3] = 40.0;
        values[(4 + YOLOV8_PERSON_CLASS_ID) * anchor_count] = 0.80;
        values[(4 + YOLOV8_PERSON_CLASS_ID) * anchor_count + 1] = 0.10;

        let decoded = decode_yolov8_people_output(
            &YoloV8RawOutput {
                shape: vec![1, YOLOV8_DETECT_OUTPUT_CHANNELS, anchor_count],
                values,
            },
            &identity_yolo_transform(100),
            100,
            0.25,
        )
        .unwrap();

        assert_eq!(decoded.objects.len(), 1);
        assert!(decoded.poses.is_empty());
        assert_near(decoded.objects[0].bbox[0], 40.0);
        assert_near(decoded.objects[0].bbox[1], 40.0);
        assert_near(decoded.objects[0].bbox[2], 20.0);
        assert_near(decoded.objects[0].bbox[3], 40.0);
        assert_eq!(decoded.objects[0].track_id, Some(1));
    }

    #[test]
    fn yolov8_pose_output_decodes_keypoints_from_shape() {
        let anchor_count = 1;
        let mut values = vec![0.0; YOLOV8_POSE_OUTPUT_CHANNELS * anchor_count];
        values[0] = 50.0;
        values[1] = 60.0;
        values[2] = 20.0;
        values[3] = 40.0;
        values[4] = 0.90;
        for index in 0..MOVENET_KEYPOINT_COUNT {
            let offset = YOLOV8_POSE_KEYPOINT_OFFSET + index * 3;
            values[offset] = 50.0 + index as f32;
            values[offset + 1] = 40.0 + index as f32;
            values[offset + 2] = 0.80;
        }

        let decoded = decode_yolov8_people_output(
            &YoloV8RawOutput {
                shape: vec![1, YOLOV8_POSE_OUTPUT_CHANNELS, anchor_count],
                values,
            },
            &identity_yolo_transform(100),
            100,
            0.25,
        )
        .unwrap();

        assert_eq!(decoded.objects.len(), 1);
        assert_eq!(decoded.poses.len(), 1);
        assert_eq!(decoded.poses[0].keypoints.len(), MOVENET_KEYPOINT_COUNT);
        assert_eq!(decoded.poses[0].keypoints[0].name, "nose");
        assert_near(decoded.poses[0].keypoints[0].point[0], 50.0);
        assert_near(decoded.poses[0].keypoints[0].point[1], 40.0);
        assert_near(decoded.poses[0].keypoints[0].score, 0.80);
        assert_near(decoded.poses[0].bbox[0], decoded.objects[0].bbox[0]);
        assert_near(decoded.poses[0].bbox[1], decoded.objects[0].bbox[1]);
    }

    #[test]
    fn yolov8_scan_regions_include_full_frame_and_zoomed_tiles() {
        let frame = frame_with_blob(640, 360);
        let regions = yolov8_scan_regions(&frame);

        assert_eq!(regions.len(), YOLOV8_MAX_SCAN_REGIONS);
        assert_eq!(regions[0].origin, [0.0, 0.0]);
        assert_eq!(regions[0].size, [640.0, 360.0]);
        assert!(regions
            .iter()
            .skip(1)
            .any(|region| region.size[0] < 640.0 && region.size[1] < 360.0));
    }

    #[test]
    fn yolov8_region_preprocess_maps_crop_back_to_screen_space() {
        let frame = frame_with_blob(640, 360);
        let input = prepare_yolov8_input_region(
            &frame,
            YOLOV8_INPUT_SIZE,
            YoloV8ScanRegion {
                origin: [160.0, 90.0],
                size: [320.0, 180.0],
            },
        )
        .unwrap();

        assert_eq!(input.transform.source_origin, [160.0, 90.0]);
        assert_eq!(input.transform.source_size, [320.0, 180.0]);
        assert_near(input.transform.scale, 2.0);
        assert_near(input.transform.pad_x, 0.0);
        assert_near(input.transform.pad_y, 140.0);

        let top_left = input.transform.model_point_to_screen(0.0, 140.0);
        assert_near(top_left[0], 340.0);
        assert_near(top_left[1], 230.0);
    }

    #[test]
    fn yolov8_merge_dedupes_objects_across_scan_regions() {
        let first = DetectionObject {
            class_name: "person".to_string(),
            bbox: [10.0, 20.0, 40.0, 80.0],
            head_bbox: None,
            head_point: Some([30.0, 34.0]),
            confidence: 0.80,
            track_id: Some(99),
        };
        let second = DetectionObject {
            class_name: "person".to_string(),
            bbox: [12.0, 22.0, 40.0, 80.0],
            head_bbox: None,
            head_point: Some([32.0, 36.0]),
            confidence: 0.95,
            track_id: Some(100),
        };

        let decoded = merge_yolov8_decoded_outputs(vec![
            YoloV8DecodedOutput {
                objects: vec![first],
                poses: Vec::new(),
            },
            YoloV8DecodedOutput {
                objects: vec![second],
                poses: Vec::new(),
            },
        ]);

        assert_eq!(decoded.objects.len(), 1);
        assert_near(decoded.objects[0].confidence, 0.95);
        assert_eq!(decoded.objects[0].track_id, Some(1));
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
                source_origin: [0.0, 0.0],
                source_size: [2.0, 2.0],
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
