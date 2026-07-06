# AutoAim Review Docs

This folder contains architecture documentation for the AutoAim Review runtime.

## Architecture

- English: [`architecture.md`](architecture.md)
- 中文：[`architecture.zh.md`](architecture.zh.md)

The two architecture documents cover the same runtime areas:

- product boundary and safety constraints,
- Rust workspace layout,
- live runtime topology,
- threading model,
- capture pipeline,
- inference providers,
- YOLOv8 scan strategy,
- candidate filtering,
- tracking and prediction,
- bounded relative cursor movement,
- overlay and UI rendering,
- dataset recording,
- packaging and update,
- performance budget,
- failure modes,
- testing,
- roadmap.

Keep both language versions aligned when changing runtime architecture.
