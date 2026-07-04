# Model Files

Large model weights are intentionally not tracked in Git.

Expected local files:

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
python scripts/prepare_models.py
```

`yolov8n.onnx` is the default live detector. `prepare_models.py` can download it
from the configured default URL, or you can override it with
`AUTOAIM_YOLOV8_ONNX_URL`.
