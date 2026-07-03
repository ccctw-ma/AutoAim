use autoaim_capture::CapturedFrame;
use autoaim_infer::{
    decode_movenet_output, detect_movenet_poses, prepare_movenet_input, InferenceError,
    MoveNetInput, MoveNetRawOutput, PoseModel, MOVENET_KEYPOINT_COUNT,
};

#[derive(Debug)]
struct MockMoveNetModel {
    input_size: u32,
    output: MoveNetRawOutput,
}

impl PoseModel for MockMoveNetModel {
    fn input_size(&self) -> u32 {
        self.input_size
    }

    fn infer(&self, input: &MoveNetInput) -> Result<MoveNetRawOutput, InferenceError> {
        assert_eq!(input.size, self.input_size);
        assert_eq!(
            input.rgb.len(),
            self.input_size as usize * self.input_size as usize * 3
        );
        Ok(self.output.clone())
    }
}

fn synthetic_frame() -> CapturedFrame {
    let width = 64_u32;
    let height = 48_u32;
    let mut rgba = vec![0_u8; width as usize * height as usize * 4];
    for y in 0..height as usize {
        for x in 0..width as usize {
            let index = (y * width as usize + x) * 4;
            rgba[index] = (x * 3).min(255) as u8;
            rgba[index + 1] = (y * 4).min(255) as u8;
            rgba[index + 2] = 90;
            rgba[index + 3] = 255;
        }
    }

    CapturedFrame {
        screen_origin: [100, 50],
        screen_size: [640, 480],
        frame_size: [width, height],
        rgba,
        cursor: [300.0, 220.0],
        cursor_on_screen: true,
        timestamp_millis: 42,
    }
}

fn movenet_output(center_x: f32, center_y: f32, score: f32) -> MoveNetRawOutput {
    let mut values = Vec::with_capacity(MOVENET_KEYPOINT_COUNT * 3);
    for index in 0..MOVENET_KEYPOINT_COUNT {
        let row = index / 4;
        let col = index % 4;
        values.push((center_y + row as f32 * 0.018).clamp(0.0, 1.0));
        values.push((center_x + col as f32 * 0.025).clamp(0.0, 1.0));
        values.push(score);
    }
    MoveNetRawOutput { values }
}

#[test]
fn e2e_movenet_pose_outputs_screen_space_person_and_keypoints() {
    let frame = synthetic_frame();
    let model = MockMoveNetModel {
        input_size: 192,
        output: movenet_output(0.45, 0.42, 0.88),
    };

    let output = detect_movenet_poses(&frame, &model, 0.25).unwrap();

    assert_eq!(output.objects.len(), 1);
    assert_eq!(output.poses.len(), 1);
    let object = &output.objects[0];
    let pose = &output.poses[0];
    assert_eq!(object.class_name, "person");
    assert_eq!(pose.keypoints.len(), MOVENET_KEYPOINT_COUNT);
    assert!(object.bbox[0] >= frame.screen_origin[0] as f32);
    assert!(object.bbox[1] >= frame.screen_origin[1] as f32);
    assert!(object.bbox[2] > 10.0);
    assert!(object.bbox[3] > 10.0);
    assert_eq!(object.head_point, Some(pose.head_point));
    assert!(pose.confidence > 0.80);
}

#[test]
fn movenet_preprocess_letterboxes_and_keeps_frame_to_screen_transform() {
    let frame = synthetic_frame();
    let input = prepare_movenet_input(&frame, 192).unwrap();

    assert_eq!(input.size, 192);
    assert_eq!(input.rgb.len(), 192 * 192 * 3);
    assert_eq!(input.transform.frame_size, frame.frame_size);
    assert_eq!(input.transform.screen_origin, frame.screen_origin);
    assert_eq!(input.transform.screen_size, frame.screen_size);
    assert!(input.transform.pad_y > 0.0);
    assert_eq!(input.transform.pad_x, 0.0);

    let screen_point = input.transform.model_point_to_screen(96.0, 96.0);
    assert!(screen_point[0] > 415.0 && screen_point[0] < 425.0);
    assert!(screen_point[1] > 285.0 && screen_point[1] < 305.0);
}

#[test]
fn movenet_decode_rejects_short_output() {
    let frame = synthetic_frame();
    let input = prepare_movenet_input(&frame, 192).unwrap();
    let error = decode_movenet_output(
        &MoveNetRawOutput {
            values: vec![0.0; 12],
        },
        &input.transform,
        input.size,
        0.25,
    )
    .unwrap_err();

    assert!(error.to_string().contains("expected at least"));
}

#[test]
fn movenet_high_threshold_filters_low_confidence_pose() {
    let frame = synthetic_frame();
    let model = MockMoveNetModel {
        input_size: 192,
        output: movenet_output(0.45, 0.42, 0.30),
    };

    let output = detect_movenet_poses(&frame, &model, 0.80).unwrap();

    assert!(output.objects.is_empty());
    assert!(output.poses.is_empty());
}

#[test]
fn movenet_decode_filters_pose_when_visible_average_is_below_threshold() {
    let frame = synthetic_frame();
    let input = prepare_movenet_input(&frame, 192).unwrap();
    let pose = decode_movenet_output(
        &movenet_output(0.45, 0.42, 0.70),
        &input.transform,
        input.size,
        0.80,
    )
    .unwrap();

    assert!(pose.is_none());
}

#[test]
fn movenet_decode_uses_bbox_head_fallback_when_face_keypoints_are_hidden() {
    let frame = synthetic_frame();
    let input = prepare_movenet_input(&frame, 192).unwrap();
    let mut raw = movenet_output(0.45, 0.42, 0.80);
    for face_index in 0..5 {
        raw.values[face_index * 3 + 2] = 0.05;
    }

    let pose = decode_movenet_output(&raw, &input.transform, input.size, 0.25)
        .unwrap()
        .expect("body keypoints should produce a pose");

    assert_eq!(pose.keypoints[0].score, 0.05);
    assert!(pose.head_point[0] >= pose.bbox[0]);
    assert!(pose.head_point[0] <= pose.bbox[0] + pose.bbox[2]);
    assert!(pose.head_point[1] >= pose.bbox[1]);
    assert!(pose.head_point[1] <= pose.bbox[1] + pose.bbox[3]);
}

#[test]
fn movenet_preprocess_rejects_invalid_rgba_buffer() {
    let mut frame = synthetic_frame();
    frame.rgba.pop();

    let error = prepare_movenet_input(&frame, 192).unwrap_err();

    assert!(error.to_string().contains("RGBA buffer"));
}
