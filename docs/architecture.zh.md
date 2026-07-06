# AutoAim Review 架构说明

中文版本。英文版本见 [`architecture.md`](architecture.md)。

AutoAim Review 是一个 Rust 优先的 Windows 屏幕捕获、推理回放和数据集工具。它面向可控的评测、训练和排障场景，要求所有运行时副作用都必须显式、可观察、可关闭。Python 仍保留在仓库中，用于离线数据处理；实时运行链路以 Rust 为主。

## 设计目标

- 让实时链路足够低延迟，尽量支撑 60 FPS 级别的 UI 反馈。
- 用清晰的所有权边界拆开捕获、推理、跟踪、Overlay、遥测和数据集录制。
- 避免隐藏副作用：捕获需要用户选择屏幕，数据集录制需要显式开启，鼠标移动需要同时开启 `Auto move mouse` 并按住激活键。
- 优先复用长生命周期的原生资源，避免每帧重建捕获或 GPU 对象。
- 让性能敏感步骤都能通过日志和 UI latency 指标定位。

## 非目标与安全边界

应用可以做：

- 捕获用户选择的屏幕像素；
- 执行原生人体或姿态推理；
- 绘制 Overlay 和预览；
- 记录帧元数据、推理结果和延迟；
- 在用户开启开关并按住激活键时，发送有界的相对鼠标移动。

应用不能做：

- 自动点击或自动开火；
- 向第三方进程安装键鼠 Hook；
- 修改进程内存；
- 检查或篡改游戏网络流量；
- 每帧重建 GPU 或捕获 session；
- 在 UI 线程阻塞执行捕获、推理、磁盘 IO 或遥测采样。

## 工作区结构

```text
crates/
  autoaim-core/        # 共享 DTO、几何计算、JSONL、校验、打分
  autoaim-ipc/         # JSON IPC 事件结构和 JSON line 编码
  autoaim-runtime/     # frame -> inference event 管线和事件日志
  autoaim-cli/         # validate / evaluate / suggest / replay 命令
  autoaim-capture/     # Windows 原生屏幕和光标捕获
  autoaim-infer/       # MoveNet 与 YOLOv8/YOLOv8-pose 推理适配
  autoaim-app/         # Tauri 桌面 UI、实时监控、Overlay、更新器

src/autoaim_review/    # Python 离线数据集与评估工具
schemas/               # 运行时与数据集 JSON schema
contracts/             # IPC 合约
windows/               # Windows 安装器与更新脚本
docs/                  # 架构文档
```

## 运行时拓扑

```text
Tauri UI
 -> live monitor command
 -> 带缓存 ScreenCapturer 的捕获 worker
 -> 带缓存 NativePersonDetector 的推理 worker
 -> 目标跟踪与头部预测
 -> 可选的有界相对鼠标移动
 -> Overlay 窗口事件
 -> UI 预览与遥测更新
 -> 可选的数据集录制器
```

实时监控命令使用 in-flight 标记保护。如果上一帧快照仍在执行，下一次请求会返回 `live snapshot busy`，而不是继续堆积 GPU 或捕获任务。这可以避免慢推理帧造成无限排队。

## 线程模型

- UI 线程：Tauri 窗口、命令分发、WebView 事件处理。
- Snapshot worker：通过 `spawn_blocking` 执行捕获、推理、跟踪、目标选择和快照构建。
- Capture worker：`ScreenCapturer` 持有原生捕获 session，响应帧请求，避免重复重建。
- Detector cache：`LiveDetectorState` 按当前 provider、模型路径和阈值缓存 detector，仅在配置变化时重建。
- Dataset writer：大尺寸 RGBA 帧写盘不走热路径。
- Telemetry loop：低频采样 CPU/GPU 遥测并缓存。
- Overlay window：浏览器 Canvas 使用 `requestAnimationFrame` 合并绘制，把高频光标层和低频姿态层分开。

## 捕获链路

`autoaim-capture` 提供原生屏幕与光标捕获。实时运行时的重要规则：

- 使用长生命周期 `ScreenCapturer`，不要每帧创建 DXGI/session 对象。
- 遇到 `WouldBlock` 时尽量返回缓存帧，而不是阻塞实时循环。
- 推理帧受 live capture 最大尺寸约束，目前为 1920x1080。
- UI 预览单独限制尺寸，目前为 640x360，避免 IPC 和 base64 预览传输成为主耗时。
- 只有用户显式开启数据集录制时才保存完整 RGBA 帧，并且写盘应放在后台路径。

每个捕获帧包含：

- 屏幕原点和屏幕尺寸；
- 捕获缩放后的帧尺寸；
- 捕获后端名称；
- RGBA 像素；
- 光标坐标；
- 光标是否在当前屏幕内；
- 毫秒级时间戳。

## 推理 Provider

训练和实时运行明确分离：

- 训练：Python、PyTorch、Ultralytics YOLO 或 RT-DETR；
- 导出：ONNX；
- 运行时：Rust 推理适配层；
- Provider：CPU、DirectML、CUDA、TensorRT；
- fallback：未配置模型时使用确定性的视觉候选，保证 UI 和管线可验证。

当模型路径看起来是 YOLO 模型时，运行时会走 YOLO 路径。YOLOv8 detect 模型输出人体框；YOLOv8-pose 模型输出人体框和关键点。MoveNet 仍支持姿态模型，但对于当前类似游戏场景的数据集，YOLOv8-pose 对小目标和部分遮挡目标更稳。

## YOLOv8 扫描策略

YOLOv8 预处理会进行 letterbox resize，并通过 `FrameToModelTransform` 将模型坐标映射回屏幕坐标。

实时扫描目前包含：

- 全屏区域，用于保留全局感知；
- 准星附近的小型放大区域，用于远距离或开镜目标；
- 轮转网格扫描，在主扫描为空或固定间隔时补充覆盖。

这能在不每帧执行完整多尺度网格扫描的前提下，提高准星附近的有效像素密度。

## 候选过滤

YOLO 原始输出进入 tracker 前会经过多层过滤：

- live 配置中的置信度阈值；
- pre-NMS 候选数量限制；
- bbox 最小边长和最小面积；
- bbox 宽高比保护；
- 屏幕底部自身角色框过滤；
- 基于 IoU 的 NMS；
- pose 关键点分数阈值；
- 最小可见关键点数量；
- 可见关键点平均分；
- 身体锚点或“脸部 + 肩部”结构要求；
- 关键点必须落在扩展 bbox 内；
- 可见关键点必须有最小空间展开；
- 头部点必须位于身体框上半部分。

这些过滤偏保守，目的是降低背景 UI、武器/角色 HUD、脸部纹理误匹配导致的误报。

## 跟踪与预测

`autoaim-core::ObjectTracker` 使用 IoU 和 aim point 距离为 person 对象分配 track id。实时 App 再按屏幕维护头部预测状态：

- 预测窗口：120 ms；
- 速度平滑：指数滑动平均；
- 最大接受跟踪间隔：250 ms；
- 最大头部速度钳制：5000 px/s。

预测只有在用户开启 prediction 时才生效。它影响 Overlay 显示和可选 auto-mouse 目标选择；原始检测结果仍然保留用于回放和排障。

## 目标选择

Auto mouse 目标选择按以下因素打分：

- 目标置信度；
- 目标点到当前 aim anchor 的距离；
- 开启预测时使用预测头部点。

如果光标在选中屏幕内，anchor 使用光标坐标；否则回退到选中屏幕中心。这样即使光标捕获短暂不可用，目标选择仍是确定性的。

## 有界鼠标移动

鼠标路径只在 Windows 上启用，并受用户控制：

- UI 中必须开启 `Auto move mouse`；
- 必须按住配置的激活键；
- 目标类别必须是 `person`；
- 相对移动量必须超过很小的 dead zone。

运行时发送的是相对 `SendInput` 移动，不是绝对坐标跳转。移动量由以下因素约束：

- 基础相对增益；
- 距离增益：越靠近目标越慢；
- 目标宽度增益：小目标或远目标可以更积极接近；
- 最大相对步长；
- 最小取整阈值。

运行时不点击。它会周期性记录目标 bbox、目标得分、aim delta 和 input delta，方便根据日志调参。

## Overlay 与 UI

UI 有两个视觉面：

- 主 Tauri 窗口：控制项、预览帧、遥测、延迟、模型状态、数据集控制；
- Overlay 窗口：透明穿透绘制层，用于实时人体、骨架、头部点、预测点和光标准星。

Overlay 绘制必须批处理。姿态层随 snapshot 事件更新；光标层可以更高频更新。Canvas 分层可以避免高频光标刷新导致整张姿态层反复清空和重绘。

## 数据集录制

运行时数据集记录写入用户本地应用目录。Windows 日志路径：

```text
%LOCALAPPDATA%\AutoAimReview\logs\autoaim-review.log
```

数据集记录包含：

- schema version；
- sequence id；
- timestamp；
- screen id；
- frame 文件路径；
- frame 和 screen 尺寸；
- capture backend；
- cursor 坐标；
- provider 和 model status；
- capture、detect、tracking、total 延迟；
- people：bbox、head point、predicted head point、keypoints、confidence、track id。

完整 1080p RGBA 帧约 8 MB，同步写盘会产生明显卡顿，因此大帧写入必须从实时路径移走。

## 离线工具

Rust 负责运行时录制和事件生成。Python 适合继续承担：

- 标注转换；
- 数据集校验；
- split 规划；
- 离线评估；
- 模型训练。

数据集切分必须按录制 session、地图、场景或 capture source 分组。禁止随机 frame-level split，因为相邻帧高度相似，会把几乎重复的数据泄漏到验证集。

推荐工具：

- 标注：CVAT 或 Label Studio；
- 质量复核：FiftyOne；
- 本地索引：SQLite；
- 大数据集版本管理：DVC + S3 或 MinIO。

## 打包与更新

Windows 打包位于 `windows/`：

- Inno Setup 生成安装包；
- `install.ps1` 将 release zip 安装到 `%LOCALAPPDATA%\AutoAimReview`；
- `update.ps1` 按 manifest 校验并应用增量 block update；
- Windows 上启动遥测和更新等后台子进程时必须隐藏窗口，避免终端闪烁。

安装后的应用不应该依赖 Rust、Cargo、Git 或 Python。

## 配置面

当前用户可见的运行时配置包括：

- screen selection；
- provider selection；
- model path；
- confidence threshold；
- activation key；
- prediction overlay toggle；
- auto-mouse toggle；
- preview frame inclusion；
- live dataset recording。

激活键支持 Alt 变体、右键、Mouse4、Mouse5 和 always-on。侧键通常能绕过部分游戏对普通键盘修饰键的拦截。

## 性能预算

实时路径应优先满足：

- 尽量稳定的 16 ms UI 轮询节奏；
- 不每帧重建 capture session；
- 不产生无限推理队列；
- 低频遥测采样；
- 预览帧传输节流；
- 数据集后台写盘；
- Overlay 合并绘制。

慢帧会写日志。推荐调参流程是：录制一小段 live dataset，离线 replay，检查 p95 延迟和失败样本，再决定是否调整阈值、扫描区域或模型。

## 故障模式

已知故障类型与对应策略：

- Capture `WouldBlock`：返回缓存帧；只有硬错误后才重建 capturer；
- GPU/provider 卡住：in-flight guard 防止请求堆积；
- 模型误报：bbox 几何、关键点质量、自身角色过滤；
- UI 预览压力过大：下采样预览，必要时关闭 frame transfer；
- 数据集 IO 峰值：从实时路径移出大帧写盘；
- Windows 终端闪烁：遥测和更新 helper 使用隐藏进程；
- 激活键被游戏吞掉：支持 Mouse4/Mouse5 和 always-on。

## 测试

常用本地检查：

```bash
cargo fmt
cargo test -p autoaim-infer
cargo test -p autoaim-app
cargo test --workspace
```

推理相关测试覆盖：

- YOLO 输出 layout 检测；
- detect 与 pose 解码；
- 低质量关键点过滤；
- face-only 误报过滤；
- 屏幕底部自身角色框过滤；
- 扫描区域坐标映射回屏幕；
- merge 和 NMS 行为。

## 路线图

- 用 replay 统计继续收紧 YOLOv8-pose 阈值；
- 增加更明确的 provider 级延迟统计；
- 增加 rejected candidates 面板，展示候选被拒原因；
- 增加扫描 preset：全屏、开镜中心、低延迟模式；
- release 流程稳定后增加包签名；
- 驱动级或硬件 HID 鼠标路径不进入默认运行时；如果未来需要，必须保持可选并显式文档化。
