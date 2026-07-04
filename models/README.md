# Model Files

Large model weights are intentionally not tracked in Git.

Expected local files:

- `yolov8n-pose.onnx`
- `yolov8n.onnx`
- `movenet_lightning.onnx`
- `movenet_lightning.tflite`

Prepare them from a previous release package:

```bash
python scripts/prepare_models.py --package path/to/AutoAimReview-windows-x64.zip
```

Or provide direct download URLs:

```bash
AUTOAIM_MOVENET_ONNX_URL=https://example/movenet_lightning.onnx \
AUTOAIM_MOVENET_TFLITE_URL=https://example/movenet_lightning.tflite \
AUTOAIM_YOLOV8_POSE_ONNX_URL=https://example/yolov8n-pose.onnx \
python scripts/prepare_models.py
```

`yolov8n-pose.onnx` is the default live detector when available and provides
person boxes plus skeleton keypoints. `yolov8n.onnx` remains the bbox-only
fallback. `prepare_models.py` can download `yolov8n.onnx` from the configured
default URL, or you can override it with `AUTOAIM_YOLOV8_ONNX_URL`.
