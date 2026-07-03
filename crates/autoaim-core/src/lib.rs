use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    error::Error,
    fmt,
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::Path,
};

pub type BBox = [f32; 4];
pub type Point = [f32; 2];

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct FrameInput {
    pub cursor: Point,
    pub mouse_down: bool,
}

impl Default for FrameInput {
    fn default() -> Self {
        Self {
            cursor: [0.0, 0.0],
            mouse_down: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DetectionObject {
    #[serde(alias = "class_name", rename = "class")]
    pub class_name: String,
    pub bbox: BBox,
    #[serde(default)]
    pub head_bbox: Option<BBox>,
    #[serde(default)]
    pub head_point: Option<Point>,
    #[serde(default)]
    pub confidence: f32,
    #[serde(default)]
    pub track_id: Option<u64>,
}

impl DetectionObject {
    pub fn person(bbox: BBox) -> Self {
        Self {
            class_name: "person".to_string(),
            bbox,
            head_bbox: None,
            head_point: None,
            confidence: 0.0,
            track_id: None,
        }
    }

    pub fn aim_point(&self) -> Point {
        if let Some(point) = self.head_point {
            return point;
        }

        if let Some(head_bbox) = self.head_bbox {
            return bbox_center(head_bbox);
        }

        let [x, y, w, h] = self.bbox;
        [x + w / 2.0, y + h * 0.18]
    }

    pub fn distance_to_cursor(&self, cursor: Point) -> f32 {
        let point = self.aim_point();
        let dx = point[0] - cursor[0];
        let dy = point[1] - cursor[1];
        (dx * dx + dy * dy).sqrt()
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct FrameRecord {
    pub frame_id: u64,
    pub timestamp_qpc: u64,
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub objects: Vec<DetectionObject>,
    #[serde(default)]
    pub input: FrameInput,
    #[serde(default)]
    pub resolution: Option<[u32; 2]>,
    #[serde(default)]
    pub window_handle: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub scene_id: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct AimSuggestion {
    pub frame_id: u64,
    pub suggested_point: Option<Point>,
    pub confidence: f32,
    pub target_index: Option<usize>,
    pub dx: Option<f32>,
    pub dy: Option<f32>,
    pub score: f32,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct MetricsSummary {
    pub frame_count: usize,
    pub object_count: usize,
    pub target_count: usize,
    pub mean_confidence: f32,
    pub mean_abs_dx: f32,
    pub mean_abs_dy: f32,
    pub mean_distance: f32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ValidationDiagnostic {
    pub frame_id: Option<u64>,
    pub message: String,
}

#[derive(Debug)]
pub enum AutoAimError {
    Io(std::io::Error),
    JsonLine {
        line: usize,
        source: serde_json::Error,
    },
    Json(serde_json::Error),
}

impl fmt::Display for AutoAimError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AutoAimError::Io(error) => write!(formatter, "I/O error: {error}"),
            AutoAimError::JsonLine { line, source } => {
                write!(formatter, "invalid JSONL at line {line}: {source}")
            }
            AutoAimError::Json(error) => write!(formatter, "JSON error: {error}"),
        }
    }
}

impl Error for AutoAimError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            AutoAimError::Io(error) => Some(error),
            AutoAimError::JsonLine { source, .. } => Some(source),
            AutoAimError::Json(error) => Some(error),
        }
    }
}

impl From<std::io::Error> for AutoAimError {
    fn from(error: std::io::Error) -> Self {
        AutoAimError::Io(error)
    }
}

impl From<serde_json::Error> for AutoAimError {
    fn from(error: serde_json::Error) -> Self {
        AutoAimError::Json(error)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct TargetScorer {
    pub confidence_weight: f32,
    pub distance_weight: f32,
    pub stability_weight: f32,
    pub max_distance_px: f32,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct TrackerConfig {
    pub min_iou: f32,
    pub max_center_distance_px: f32,
    pub max_missed_frames: u64,
}

impl Default for TrackerConfig {
    fn default() -> Self {
        Self {
            min_iou: 0.05,
            max_center_distance_px: 180.0,
            max_missed_frames: 8,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct TrackState {
    id: u64,
    class_name: String,
    bbox: BBox,
    aim_point: Point,
    last_seen_frame: u64,
}

#[derive(Debug)]
pub struct ObjectTracker {
    config: TrackerConfig,
    next_track_id: u64,
    frame_index: u64,
    tracks: Vec<TrackState>,
}

impl Default for ObjectTracker {
    fn default() -> Self {
        Self::new(TrackerConfig::default())
    }
}

impl ObjectTracker {
    pub fn new(config: TrackerConfig) -> Self {
        Self {
            config,
            next_track_id: 1,
            frame_index: 0,
            tracks: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.next_track_id = 1;
        self.frame_index = 0;
        self.tracks.clear();
    }

    pub fn assign(&mut self, objects: &mut [DetectionObject]) {
        self.frame_index += 1;
        self.tracks.retain(|track| {
            self.frame_index.saturating_sub(track.last_seen_frame) <= self.config.max_missed_frames
        });

        let mut claimed_tracks = vec![false; self.tracks.len()];
        for object in objects.iter_mut() {
            if object.class_name != "person" {
                continue;
            }

            let aim_point = object.aim_point();
            let mut best_track = None;
            let mut best_score = f32::NEG_INFINITY;

            for (index, track) in self.tracks.iter().enumerate() {
                if claimed_tracks[index] || track.class_name != object.class_name {
                    continue;
                }

                let iou = bbox_iou(track.bbox, object.bbox);
                let center_distance = point_distance(track.aim_point, aim_point);
                let allowed_distance = self
                    .config
                    .max_center_distance_px
                    .max(object.bbox[2].max(object.bbox[3]) * 0.65);

                if iou < self.config.min_iou && center_distance > allowed_distance {
                    continue;
                }

                let distance_score = 1.0 - (center_distance / allowed_distance).min(1.0);
                let score = iou * 0.65 + distance_score * 0.35;
                if score > best_score {
                    best_score = score;
                    best_track = Some(index);
                }
            }

            if let Some(track_index) = best_track {
                let track = &mut self.tracks[track_index];
                track.bbox = object.bbox;
                track.aim_point = aim_point;
                track.last_seen_frame = self.frame_index;
                object.track_id = Some(track.id);
                claimed_tracks[track_index] = true;
            } else {
                let track_id = self.next_track_id;
                self.next_track_id += 1;
                self.tracks.push(TrackState {
                    id: track_id,
                    class_name: object.class_name.clone(),
                    bbox: object.bbox,
                    aim_point,
                    last_seen_frame: self.frame_index,
                });
                claimed_tracks.push(true);
                object.track_id = Some(track_id);
            }
        }
    }
}

impl Default for TargetScorer {
    fn default() -> Self {
        Self {
            confidence_weight: 0.55,
            distance_weight: 0.35,
            stability_weight: 0.10,
            max_distance_px: 900.0,
        }
    }
}

impl TargetScorer {
    pub fn score(
        &self,
        target: &DetectionObject,
        cursor: Point,
        previous_track_id: Option<u64>,
    ) -> f32 {
        let distance = target.distance_to_cursor(cursor).min(self.max_distance_px);
        let distance_score = 1.0 - distance / self.max_distance_px;
        let confidence_score = target.confidence.clamp(0.0, 1.0);
        let stability_score = if previous_track_id.is_some() && target.track_id == previous_track_id
        {
            1.0
        } else {
            0.0
        };

        confidence_score * self.confidence_weight
            + distance_score * self.distance_weight
            + stability_score * self.stability_weight
    }
}

pub fn bbox_center(bbox: BBox) -> Point {
    let [x, y, w, h] = bbox;
    [x + w / 2.0, y + h / 2.0]
}

pub fn bbox_iou(left: BBox, right: BBox) -> f32 {
    let left_x1 = left[0];
    let left_y1 = left[1];
    let left_x2 = left[0] + left[2];
    let left_y2 = left[1] + left[3];
    let right_x1 = right[0];
    let right_y1 = right[1];
    let right_x2 = right[0] + right[2];
    let right_y2 = right[1] + right[3];

    let intersection_w = (left_x2.min(right_x2) - left_x1.max(right_x1)).max(0.0);
    let intersection_h = (left_y2.min(right_y2) - left_y1.max(right_y1)).max(0.0);
    let intersection = intersection_w * intersection_h;
    if intersection <= 0.0 {
        return 0.0;
    }

    let left_area = (left[2].max(0.0)) * (left[3].max(0.0));
    let right_area = (right[2].max(0.0)) * (right[3].max(0.0));
    let union = left_area + right_area - intersection;
    if union <= 0.0 {
        return 0.0;
    }

    intersection / union
}

pub fn point_distance(left: Point, right: Point) -> f32 {
    let dx = left[0] - right[0];
    let dy = left[1] - right[1];
    (dx * dx + dy * dy).sqrt()
}

pub fn choose_target(
    frame: &FrameRecord,
    scorer: TargetScorer,
    previous_track_id: Option<u64>,
) -> AimSuggestion {
    let Some((target_index, target, score)) = frame
        .objects
        .iter()
        .enumerate()
        .filter(|(_, object)| object.class_name == "person")
        .map(|(index, object)| {
            (
                index,
                object,
                scorer.score(object, frame.input.cursor, previous_track_id),
            )
        })
        .max_by(|left, right| left.2.partial_cmp(&right.2).unwrap_or(Ordering::Equal))
    else {
        return AimSuggestion {
            frame_id: frame.frame_id,
            suggested_point: None,
            confidence: 0.0,
            target_index: None,
            dx: None,
            dy: None,
            score: 0.0,
        };
    };

    let point = target.aim_point();
    let [cursor_x, cursor_y] = frame.input.cursor;

    AimSuggestion {
        frame_id: frame.frame_id,
        suggested_point: Some(point),
        confidence: target.confidence,
        target_index: Some(target_index),
        dx: Some(point[0] - cursor_x),
        dy: Some(point[1] - cursor_y),
        score,
    }
}

pub fn choose_target_default(frame: &FrameRecord) -> AimSuggestion {
    choose_target(frame, TargetScorer::default(), None)
}

pub fn suggest_frames(records: &[FrameRecord], scorer: TargetScorer) -> Vec<AimSuggestion> {
    let mut previous_track_id = None;
    let mut suggestions = Vec::with_capacity(records.len());

    for record in records {
        let suggestion = choose_target(record, scorer, previous_track_id);
        previous_track_id = suggestion
            .target_index
            .and_then(|index| record.objects.get(index))
            .and_then(|object| object.track_id);
        suggestions.push(suggestion);
    }

    suggestions
}

pub fn summarize(records: &[FrameRecord], suggestions: &[AimSuggestion]) -> MetricsSummary {
    let frame_count = records.len();
    let object_count = records.iter().map(|record| record.objects.len()).sum();
    let target_count = suggestions
        .iter()
        .filter(|suggestion| suggestion.suggested_point.is_some())
        .count();

    if target_count == 0 {
        return MetricsSummary {
            frame_count,
            object_count,
            target_count,
            mean_confidence: 0.0,
            mean_abs_dx: 0.0,
            mean_abs_dy: 0.0,
            mean_distance: 0.0,
        };
    }

    let mut confidence_sum = 0.0;
    let mut abs_dx_sum = 0.0;
    let mut abs_dy_sum = 0.0;
    let mut distance_sum = 0.0;

    for suggestion in suggestions
        .iter()
        .filter(|suggestion| suggestion.suggested_point.is_some())
    {
        let dx = suggestion.dx.unwrap_or(0.0);
        let dy = suggestion.dy.unwrap_or(0.0);
        confidence_sum += suggestion.confidence;
        abs_dx_sum += dx.abs();
        abs_dy_sum += dy.abs();
        distance_sum += (dx * dx + dy * dy).sqrt();
    }

    let target_count_f32 = target_count as f32;
    MetricsSummary {
        frame_count,
        object_count,
        target_count,
        mean_confidence: confidence_sum / target_count_f32,
        mean_abs_dx: abs_dx_sum / target_count_f32,
        mean_abs_dy: abs_dy_sum / target_count_f32,
        mean_distance: distance_sum / target_count_f32,
    }
}

pub fn validate_records(records: &[FrameRecord]) -> Vec<ValidationDiagnostic> {
    let mut diagnostics = Vec::new();

    for record in records {
        if record.session_id.is_none() && record.scene_id.is_none() {
            diagnostics.push(ValidationDiagnostic {
                frame_id: Some(record.frame_id),
                message: "frame should include session_id or scene_id for grouped splits"
                    .to_string(),
            });
        }

        for (index, object) in record.objects.iter().enumerate() {
            let [_x, _y, width, height] = object.bbox;
            if width <= 0.0 || height <= 0.0 {
                diagnostics.push(ValidationDiagnostic {
                    frame_id: Some(record.frame_id),
                    message: format!("object {index} has non-positive bbox size"),
                });
            }

            if !(0.0..=1.0).contains(&object.confidence) {
                diagnostics.push(ValidationDiagnostic {
                    frame_id: Some(record.frame_id),
                    message: format!("object {index} confidence is outside [0, 1]"),
                });
            }
        }
    }

    diagnostics
}

pub fn read_jsonl_path(path: impl AsRef<Path>) -> Result<Vec<FrameRecord>, AutoAimError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut records = Vec::new();

    for (index, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let record = serde_json::from_str::<FrameRecord>(&line).map_err(|source| {
            AutoAimError::JsonLine {
                line: index + 1,
                source,
            }
        })?;
        records.push(record);
    }

    Ok(records)
}

pub fn write_jsonl_path(
    path: impl AsRef<Path>,
    records: &[FrameRecord],
) -> Result<(), AutoAimError> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);

    for record in records {
        serde_json::to_writer(&mut writer, record)?;
        writer.write_all(b"\n")?;
    }

    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choose_target_prefers_head_point_near_cursor() {
        let frame = FrameRecord {
            frame_id: 1,
            timestamp_qpc: 10,
            image: "frame.jpg".to_string(),
            input: FrameInput {
                cursor: [500.0, 300.0],
                mouse_down: false,
            },
            objects: vec![
                DetectionObject {
                    class_name: "person".to_string(),
                    bbox: [100.0, 100.0, 100.0, 300.0],
                    head_bbox: None,
                    head_point: Some([150.0, 140.0]),
                    confidence: 0.99,
                    track_id: None,
                },
                DetectionObject {
                    class_name: "person".to_string(),
                    bbox: [470.0, 260.0, 80.0, 220.0],
                    head_bbox: None,
                    head_point: Some([505.0, 295.0]),
                    confidence: 0.75,
                    track_id: None,
                },
            ],
            resolution: None,
            window_handle: None,
            session_id: None,
            scene_id: None,
        };

        let suggestion = choose_target(&frame, TargetScorer::default(), None);

        assert_eq!(suggestion.target_index, Some(1));
        assert_eq!(suggestion.suggested_point, Some([505.0, 295.0]));
        assert_eq!(suggestion.dx, Some(5.0));
        assert_eq!(suggestion.dy, Some(-5.0));
    }

    #[test]
    fn aim_point_prefers_head_bbox_over_body_fallback() {
        let object = DetectionObject {
            class_name: "person".to_string(),
            bbox: [100.0, 100.0, 100.0, 300.0],
            head_bbox: Some([120.0, 110.0, 40.0, 50.0]),
            head_point: None,
            confidence: 0.8,
            track_id: None,
        };

        assert_eq!(object.aim_point(), [140.0, 135.0]);
    }

    #[test]
    fn aim_point_falls_back_to_upper_body() {
        let object = DetectionObject::person([100.0, 100.0, 100.0, 300.0]);

        assert_eq!(object.aim_point(), [150.0, 154.0]);
    }

    #[test]
    fn object_tracker_reuses_track_id_for_nearby_person() {
        let mut tracker = ObjectTracker::default();
        let mut frame_one = vec![DetectionObject {
            class_name: "person".to_string(),
            bbox: [100.0, 100.0, 80.0, 220.0],
            head_bbox: None,
            head_point: Some([140.0, 135.0]),
            confidence: 0.9,
            track_id: None,
        }];
        tracker.assign(&mut frame_one);

        let mut frame_two = vec![DetectionObject {
            class_name: "person".to_string(),
            bbox: [106.0, 102.0, 82.0, 218.0],
            head_bbox: None,
            head_point: Some([145.0, 136.0]),
            confidence: 0.88,
            track_id: None,
        }];
        tracker.assign(&mut frame_two);

        assert_eq!(frame_one[0].track_id, Some(1));
        assert_eq!(frame_two[0].track_id, Some(1));
    }

    #[test]
    fn object_tracker_assigns_new_track_id_to_far_person() {
        let mut tracker = ObjectTracker::default();
        let mut frame_one = vec![DetectionObject::person([100.0, 100.0, 80.0, 220.0])];
        tracker.assign(&mut frame_one);

        let mut frame_two = vec![DetectionObject::person([500.0, 200.0, 80.0, 220.0])];
        tracker.assign(&mut frame_two);

        assert_eq!(frame_one[0].track_id, Some(1));
        assert_eq!(frame_two[0].track_id, Some(2));
    }

    #[test]
    fn bbox_iou_and_point_distance_cover_basic_geometry() {
        assert!(bbox_iou([0.0, 0.0, 100.0, 100.0], [50.0, 50.0, 100.0, 100.0]) > 0.1);
        assert_eq!(
            bbox_iou([0.0, 0.0, 10.0, 10.0], [20.0, 20.0, 5.0, 5.0]),
            0.0
        );
        assert_eq!(point_distance([0.0, 0.0], [3.0, 4.0]), 5.0);
    }
}
