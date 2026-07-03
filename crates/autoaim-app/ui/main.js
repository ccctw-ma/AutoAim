const tauriApi = window.__TAURI__ || {};
const invoke = tauriApi.tauri?.invoke;
const dialog = tauriApi.dialog;

const i18n = {
  en: {
    eyebrow: "Windows review utility",
    title: "AutoAim Review",
    subtitle: "Select a screen and run visual monitoring.",
    languageLabel: "Language",
    liveKicker: "Live",
    liveTitle: "Screen monitor",
    screen: "Screen",
    refreshScreens: "Refresh",
    startLive: "Start",
    stopLive: "Stop",
    showOverlay: "Overlay",
    hideOverlay: "Close overlay",
    showPreview: "Frame preview",
    liveStopped: "Stopped",
    liveRunning: "Running",
    liveStarting: "Starting live monitor...",
    liveStarted: "Live monitor started.",
    liveStoppedStatus: "Live monitor stopped.",
    liveBusy: "Live monitor is still processing a frame.",
    nativeCapture: "Native Windows capture",
    modelLoading: "Capturing screen and running detector...",
    modelLoaded: "Native detector ready.",
    modelUnavailable: "Person detector unavailable.",
    mousePosition: "Mouse",
    peopleCount: "People",
    modelStatus: "Model",
    captureStatus: "Capture",
    peopleKicker: "Detection",
    peopleTitle: "People",
    noPeople: "No people detected.",
    workflowKicker: "Offline",
    workflowTitle: "Dataset tools",
    statusReady: "Ready",
    frameJsonl: "Frame JSONL",
    choose: "Choose",
    eventOutput: "Event output",
    saveAs: "Save as",
    validate: "Validate",
    evaluate: "Evaluate",
    positions: "Person positions",
    preview: "Preview events",
    writeEvents: "Write events",
    runtimeKicker: "Runtime",
    runtimeTitle: "Inference",
    provider: "Provider",
    threshold: "Confidence",
    modelPath: "Model path",
    modelPathPlaceholder: "Bundled model",
    showRuntime: "Show config",
    updateStatusIdle: "Update",
    updateStatusChecking: "Checking",
    updateStatusReady: "Update ready",
    updateStatusCurrent: "Up to date",
    updateDialogKicker: "Update",
    updateDialogTitle: "A new version is available",
    updateDialogText: "AutoAim Review can restart now and apply the update.",
    restartToUpdate: "Restart and update",
    updateLater: "Later",
    updateAvailable: "Update available.",
    noUpdateAvailable: "You are already on the latest version.",
    updateCheckFailed: "Update check failed.",
    updateApplyFailed: "Update could not start.",
    metricsKicker: "Metrics",
    metricsTitle: "Current run",
    frames: "Frames",
    objects: "Objects",
    targets: "Targets",
    confidence: "Confidence",
    distance: "Mean distance",
    guideKicker: "How to use",
    guideTitle: "Four-step offline review",
    guide1: "Choose a frame JSONL file, or use the bundled sample file.",
    guide2: "Run validation to catch missing grouping fields or invalid boxes.",
    guide3: "Run evaluation to compute target suggestions and summary metrics.",
    guide4: "Write event JSONL when you need review-only inference results.",
    nextTitle: "Next runtime modules",
    nextText: "Live mode uses native Windows screen capture, cursor polling, and the Rust inference boundary.",
    consoleKicker: "Diagnostics",
    consoleTitle: "Log",
    copyDiagnostics: "Copy",
    clear: "Clear",
    validating: "Validating dataset...",
    validationOk: "Validation passed.",
    validationFailed: "Validation found issues.",
    evaluating: "Evaluating dataset...",
    evaluationDone: "Evaluation complete.",
    previewing: "Generating event preview...",
    previewDone: "Preview generated.",
    positionsPreviewing: "Reading person positions...",
    positionsDone: "Person positions generated.",
    writing: "Writing event JSONL...",
    writeDone: "Event JSONL written.",
    runtimeReady: "Runtime config generated.",
    checkingUpdates: "Checking updates...",
    updateCheckDone: "Update check complete.",
    applyingUpdate: "Starting updater...",
    applyUpdateStarted: "Updater started. The app may close.",
    selectInputFirst: "Select a frame JSONL file first.",
    selectOutputFirst: "Select an output path first.",
    noTauri: "Tauri API is not available. Run this page through AutoAimReview.exe.",
    selected: "Selected input file.",
    outputSelected: "Selected output file.",
    screensLoaded: "Screens loaded.",
    noScreens: "No screens available.",
  },
  zh: {
    eyebrow: "Windows 审阅工具",
    title: "AutoAim Review",
    subtitle: "选择屏幕后进行视觉监测。",
    languageLabel: "语言",
    liveKicker: "实时",
    liveTitle: "屏幕监测",
    screen: "屏幕",
    refreshScreens: "刷新",
    startLive: "开始",
    stopLive: "停止",
    showOverlay: "悬浮层",
    hideOverlay: "关闭悬浮层",
    liveStopped: "已停止",
    liveRunning: "运行中",
    liveStarting: "正在启动实时监控...",
    liveStarted: "实时监控已启动。",
    liveStoppedStatus: "实时监控已停止。",
    liveBusy: "实时监控仍在处理上一帧。",
    nativeCapture: "Windows 原生采集",
    modelLoading: "正在采集屏幕并运行检测器...",
    modelLoaded: "原生检测器已就绪。",
    modelUnavailable: "人物检测模型不可用。",
    mousePosition: "鼠标",
    peopleCount: "人物数",
    modelStatus: "模型",
    captureStatus: "采集",
    peopleKicker: "检测",
    peopleTitle: "人物",
    noPeople: "暂未识别到人物。",
    workflowKicker: "离线",
    workflowTitle: "数据集工具",
    statusReady: "就绪",
    frameJsonl: "帧 JSONL",
    choose: "选择",
    eventOutput: "事件输出",
    saveAs: "另存为",
    validate: "校验",
    evaluate: "评估",
    positions: "人物位置",
    preview: "预览事件",
    writeEvents: "写出事件",
    runtimeKicker: "运行时",
    runtimeTitle: "推理",
    provider: "推理后端",
    threshold: "置信度",
    modelPath: "模型路径",
    modelPathPlaceholder: "使用内置模型",
    showRuntime: "显示配置",
    updateStatusIdle: "更新",
    updateStatusChecking: "检查中",
    updateStatusReady: "可更新",
    updateStatusCurrent: "已是最新",
    updateDialogKicker: "更新",
    updateDialogTitle: "发现新版本",
    updateDialogText: "AutoAim Review 可以现在重启并安装更新。",
    restartToUpdate: "重启并更新",
    updateLater: "稍后",
    updateAvailable: "发现可用更新。",
    noUpdateAvailable: "当前已经是最新版本。",
    updateCheckFailed: "检查更新失败。",
    updateApplyFailed: "无法启动更新。",
    metricsKicker: "指标",
    metricsTitle: "当前运行",
    frames: "帧数",
    objects: "目标数",
    targets: "命中目标",
    confidence: "置信度",
    distance: "平均距离",
    guideKicker: "使用说明",
    guideTitle: "四步离线审阅",
    guide1: "选择一个帧 JSONL 文件，也可以使用内置样例。",
    guide2: "先执行校验，检查缺失分组字段或无效框。",
    guide3: "执行评估，计算目标建议和汇总指标。",
    guide4: "需要审阅用推理结果时，写出事件 JSONL。",
    nextTitle: "后续运行时模块",
    nextText: "实时模式使用 Windows 原生屏幕采集、鼠标轮询和 Rust 推理边界。",
    consoleKicker: "诊断",
    consoleTitle: "日志",
    copyDiagnostics: "复制",
    clear: "清空",
    validating: "正在校验数据集...",
    validationOk: "校验通过。",
    validationFailed: "校验发现问题。",
    evaluating: "正在评估数据集...",
    evaluationDone: "评估完成。",
    previewing: "正在生成事件预览...",
    previewDone: "预览已生成。",
    positionsPreviewing: "正在读取人物位置...",
    positionsDone: "人物位置已生成。",
    writing: "正在写出事件 JSONL...",
    writeDone: "事件 JSONL 已写出。",
    runtimeReady: "运行时配置已生成。",
    checkingUpdates: "正在检查更新...",
    updateCheckDone: "更新检查完成。",
    applyingUpdate: "正在启动更新器...",
    applyUpdateStarted: "更新器已启动，应用可能会关闭。",
    selectInputFirst: "请先选择帧 JSONL 文件。",
    selectOutputFirst: "请先选择输出路径。",
    noTauri: "Tauri API 不可用，请通过 AutoAimReview.exe 打开本页面。",
    selected: "已选择输入文件。",
    outputSelected: "已选择输出文件。",
    screensLoaded: "屏幕列表已加载。",
    noScreens: "没有可用屏幕。",
  },
};

const LIVE_POLL_INTERVAL_MS = 250;
const AUTO_UPDATE_CHECK_DELAY_MS = 1200;
const KEYPOINT_SCORE_THRESHOLD = 0.2;
const SKELETON_CONNECTIONS = [
  ["nose", "left_eye"],
  ["nose", "right_eye"],
  ["left_eye", "left_ear"],
  ["right_eye", "right_ear"],
  ["left_shoulder", "right_shoulder"],
  ["left_shoulder", "left_elbow"],
  ["left_elbow", "left_wrist"],
  ["right_shoulder", "right_elbow"],
  ["right_elbow", "right_wrist"],
  ["left_shoulder", "left_hip"],
  ["right_shoulder", "right_hip"],
  ["left_hip", "right_hip"],
  ["left_hip", "left_knee"],
  ["left_knee", "left_ankle"],
  ["right_hip", "right_knee"],
  ["right_knee", "right_ankle"],
];

const state = {
  language: localStorage.getItem("autoaim.language") || "zh",
  liveTimer: null,
  liveRunning: false,
  livePolling: false,
  liveSessionId: 0,
  detectedPeople: [],
  lastCursor: [0, 0],
  lastFrame: null,
  lastSnapshotSummary: null,
  previewFrameEnabled: localStorage.getItem("autoaim.previewFrame") !== "false",
  screens: [],
  selectedScreenId: null,
  updateCheckRunning: false,
  updateAvailable: false,
  lastUpdateResult: null,
};

const $ = (id) => document.getElementById(id);
const on = (element, eventName, handler) => {
  element?.addEventListener(eventName, handler);
};

const els = {
  languageSelect: $("languageSelect"),
  statusPill: $("statusPill"),
  screenSelect: $("screenSelect"),
  refreshScreensBtn: $("refreshScreensBtn"),
  startLiveBtn: $("startLiveBtn"),
  stopLiveBtn: $("stopLiveBtn"),
  showOverlayBtn: $("showOverlayBtn"),
  hideOverlayBtn: $("hideOverlayBtn"),
  previewFrameToggle: $("previewFrameToggle"),
  monitorCanvas: $("monitorCanvas"),
  liveState: $("liveState"),
  cursorReadout: $("cursorReadout"),
  peopleReadout: $("peopleReadout"),
  modelStatusReadout: $("modelStatusReadout"),
  captureStatusReadout: $("captureStatusReadout"),
  peopleList: $("peopleList"),
  inputPath: $("inputPath"),
  outputPath: $("outputPath"),
  chooseInput: $("chooseInput"),
  chooseOutput: $("chooseOutput"),
  validateBtn: $("validateBtn"),
  evaluateBtn: $("evaluateBtn"),
  positionsBtn: $("positionsBtn"),
  previewBtn: $("previewBtn"),
  writeBtn: $("writeBtn"),
  providerSelect: $("providerSelect"),
  confidenceInput: $("confidenceInput"),
  modelPath: $("modelPath"),
  showRuntimeBtn: $("showRuntimeBtn"),
  updateStatusBtn: $("updateStatusBtn"),
  updateDialog: $("updateDialog"),
  updateVersionText: $("updateVersionText"),
  dismissUpdateBtn: $("dismissUpdateBtn"),
  restartUpdateBtn: $("restartUpdateBtn"),
  copyDiagnosticsBtn: $("copyDiagnosticsBtn"),
  clearLog: $("clearLog"),
  logOutput: $("logOutput"),
  framesMetric: $("framesMetric"),
  objectsMetric: $("objectsMetric"),
  targetsMetric: $("targetsMetric"),
  confidenceMetric: $("confidenceMetric"),
  distanceMetric: $("distanceMetric"),
};

function t(key) {
  return i18n[state.language][key] || i18n.en[key] || key;
}

function applyLanguage(language) {
  state.language = language;
  localStorage.setItem("autoaim.language", language);
  document.documentElement.lang = language === "zh" ? "zh-CN" : "en";
  if (els.languageSelect) {
    els.languageSelect.value = language;
  }

  document.querySelectorAll("[data-i18n]").forEach((node) => {
    node.textContent = t(node.dataset.i18n);
  });
  document.querySelectorAll("[data-i18n-placeholder]").forEach((node) => {
    node.placeholder = t(node.dataset.i18nPlaceholder);
  });
  updatePreviewFrameVisibility();
  renderUpdateStatus();
}

function setStatus(message, tone = "ready") {
  if (!els.statusPill) {
    return;
  }
  els.statusPill.textContent = message;
  els.statusPill.dataset.tone = tone;
}

function writeFileLog(scope, message, data) {
  if (!invoke) {
    return;
  }
  const payload = typeof data === "undefined" ? null : JSON.stringify(data);
  invoke("frontend_log", { scope, message: String(message), payload }).catch(() => {});
}

function log(message, data) {
  if (els.logOutput) {
    const timestamp = new Date().toLocaleTimeString();
    const text = typeof data === "undefined" ? message : `${message}\n${JSON.stringify(data, null, 2)}`;
    els.logOutput.textContent = `[${timestamp}] ${text}\n\n${els.logOutput.textContent}`;
  }
  writeFileLog("ui", message, data);
}

async function copyDiagnostics() {
  requireTauri();
  const confidence = Number.parseFloat(els.confidenceInput.value || "0.25");
  const context = await invoke("diagnostics_context", {
    selectedScreenId: els.screenSelect.value || state.selectedScreenId || null,
    provider: els.providerSelect.value,
    modelPath: els.modelPath.value.trim(),
    confidenceThreshold: Number.isFinite(confidence) ? confidence : 0.25,
  });
  const payload = {
    generated_at: new Date().toISOString(),
    ui_state: {
      selected_screen_id: els.screenSelect.value || state.selectedScreenId || null,
      requested_provider: els.providerSelect.value,
      requested_model_path: els.modelPath.value.trim(),
      confidence_threshold: els.confidenceInput.value,
      status_pill: els.statusPill.textContent,
    },
    diagnostics: context,
    live_summary: state.lastSnapshotSummary,
    recent_log: els.logOutput.textContent,
  };
  const report = JSON.stringify(payload, null, 2);
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(report);
  } else {
    const textarea = document.createElement("textarea");
    textarea.value = report;
    document.body.appendChild(textarea);
    textarea.select();
    document.execCommand("copy");
    textarea.remove();
  }
  log(t("copyDiagnostics"), payload.diagnostics);
  setStatus(t("copyDiagnostics"), "success");
}

function requireTauri() {
  if (!invoke) {
    throw new Error(t("noTauri"));
  }
}

function requireInputPath() {
  const value = els.inputPath.value.trim();
  if (!value) {
    throw new Error(t("selectInputFirst"));
  }
  return value;
}

function requireOutputPath() {
  const value = els.outputPath.value.trim();
  if (!value) {
    throw new Error(t("selectOutputFirst"));
  }
  return value;
}

function setBusy(isBusy) {
  [
    els.validateBtn,
    els.evaluateBtn,
    els.positionsBtn,
    els.previewBtn,
    els.writeBtn,
    els.showRuntimeBtn,
    els.refreshScreensBtn,
    els.startLiveBtn,
    els.stopLiveBtn,
    els.showOverlayBtn,
    els.hideOverlayBtn,
    els.chooseInput,
    els.chooseOutput,
  ].filter(Boolean).forEach((button) => {
    button.disabled = isBusy;
  });
}

function decodeRgbaBase64(base64) {
  const binary = atob(base64);
  const bytes = new Uint8ClampedArray(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

function drawNativeFrame(ctx, canvas, frame) {
  if (!frame?.rgba_base64) {
    ctx.fillStyle = "#111111";
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    return;
  }

  const [frameWidth, frameHeight] = frame.frame_size;
  const rgba = decodeRgbaBase64(frame.rgba_base64);
  const imageData = new ImageData(rgba, frameWidth, frameHeight);
  const bitmapCanvas = document.createElement("canvas");
  bitmapCanvas.width = frameWidth;
  bitmapCanvas.height = frameHeight;
  bitmapCanvas.getContext("2d").putImageData(imageData, 0, 0);
  ctx.drawImage(bitmapCanvas, 0, 0, canvas.width, canvas.height);
}

function visibleKeypointMap(keypoints) {
  const points = new Map();
  if (!Array.isArray(keypoints)) {
    return points;
  }
  keypoints
    .filter((keypoint) => keypoint.score >= KEYPOINT_SCORE_THRESHOLD)
    .forEach((keypoint) => points.set(keypoint.name, keypoint));
  return points;
}

function drawSkeleton(ctx, keypoints, projectPoint) {
  const points = visibleKeypointMap(keypoints);
  if (points.size < 2) {
    return;
  }

  ctx.save();
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  ctx.lineWidth = 5;
  ctx.strokeStyle = "#000000";
  SKELETON_CONNECTIONS.forEach(([from, to]) => {
    const start = points.get(from);
    const end = points.get(to);
    if (!start || !end) {
      return;
    }
    const [x1, y1] = projectPoint(start.point);
    const [x2, y2] = projectPoint(end.point);
    ctx.beginPath();
    ctx.moveTo(x1, y1);
    ctx.lineTo(x2, y2);
    ctx.stroke();
  });

  ctx.lineWidth = 2;
  ctx.strokeStyle = "#ffffff";
  SKELETON_CONNECTIONS.forEach(([from, to]) => {
    const start = points.get(from);
    const end = points.get(to);
    if (!start || !end) {
      return;
    }
    const [x1, y1] = projectPoint(start.point);
    const [x2, y2] = projectPoint(end.point);
    ctx.beginPath();
    ctx.moveTo(x1, y1);
    ctx.lineTo(x2, y2);
    ctx.stroke();
  });
  ctx.restore();
}

function drawMonitor(snapshot, people = state.detectedPeople) {
  const canvas = els.monitorCanvas;
  const ctx = canvas.getContext("2d");
  const width = canvas.width;
  const height = canvas.height;
  ctx.clearRect(0, 0, width, height);
  drawNativeFrame(ctx, canvas, snapshot.frame);

  const grid = 48;
  ctx.strokeStyle = "rgba(255, 255, 255, 0.18)";
  ctx.lineWidth = 1;
  for (let x = 0; x < width; x += grid) {
    ctx.beginPath();
    ctx.moveTo(x, 0);
    ctx.lineTo(x, height);
    ctx.stroke();
  }
  for (let y = 0; y < height; y += grid) {
    ctx.beginPath();
    ctx.moveTo(0, y);
    ctx.lineTo(width, y);
    ctx.stroke();
  }

  const frame = snapshot.frame;
  if (!frame) {
    return;
  }
  const [originX, originY] = frame.screen_origin;
  const [screenW, screenH] = frame.screen_size;
  const scaleX = width / screenW;
  const scaleY = height / screenH;

  const cursorX = (snapshot.cursor[0] - originX) * scaleX;
  const cursorY = (snapshot.cursor[1] - originY) * scaleY;
  ctx.strokeStyle = "#ffffff";
  ctx.fillStyle = "#ffffff";
  ctx.lineWidth = 2;
  ctx.beginPath();
  ctx.moveTo(cursorX - 12, cursorY);
  ctx.lineTo(cursorX + 12, cursorY);
  ctx.moveTo(cursorX, cursorY - 12);
  ctx.lineTo(cursorX, cursorY + 12);
  ctx.stroke();
  ctx.beginPath();
  ctx.arc(cursorX, cursorY, 4, 0, Math.PI * 2);
  ctx.fill();

  people.forEach((person) => {
    const [x, y, w, h] = person.bbox;
    const rectX = (x - originX) * scaleX;
    const rectY = (y - originY) * scaleY;
    const projectPoint = (point) => [(point[0] - originX) * scaleX, (point[1] - originY) * scaleY];

    ctx.strokeStyle = "#000000";
    ctx.lineWidth = 4;
    ctx.strokeRect(rectX, rectY, w * scaleX, h * scaleY);
    ctx.strokeStyle = "#ffffff";
    ctx.lineWidth = 2;
    ctx.strokeRect(rectX, rectY, w * scaleX, h * scaleY);
    drawSkeleton(ctx, person.keypoints, projectPoint);

    const [headX, headY] = projectPoint(person.head_point);
    ctx.fillStyle = "#000000";
    ctx.beginPath();
    ctx.arc(headX, headY, 6, 0, Math.PI * 2);
    ctx.fill();
    ctx.fillStyle = "#ffffff";
    ctx.beginPath();
    ctx.arc(headX, headY, 4, 0, Math.PI * 2);
    ctx.fill();

    if (Array.isArray(person.keypoints)) {
      ctx.fillStyle = "#ffffff";
      person.keypoints
        .filter((keypoint) => keypoint.score >= KEYPOINT_SCORE_THRESHOLD)
        .forEach((keypoint) => {
          const [keypointX, keypointY] = projectPoint(keypoint.point);
          ctx.beginPath();
          ctx.arc(keypointX, keypointY, 3, 0, Math.PI * 2);
          ctx.fill();
        });
    }
  });
}

function renderPeople(people) {
  els.peopleList.innerHTML = "";
  if (!people.length) {
    const empty = document.createElement("p");
    empty.className = "empty-people";
    empty.textContent = t("noPeople");
    els.peopleList.appendChild(empty);
    return;
  }

  people.forEach((person) => {
    const item = document.createElement("article");
    item.className = "person-row";
    const keypoints = Array.isArray(person.keypoints)
      ? person.keypoints
          .filter((keypoint) => keypoint.score >= 0.2)
          .slice(0, 6)
          .map((keypoint) => `${keypoint.name} [${keypoint.point.map((value) => value.toFixed(0)).join(", ")}]`)
          .join("; ")
      : "";
    item.innerHTML = `
      <strong>#${person.object_index} ${person.class_name || "person"}</strong>
      <span>bbox [${person.bbox.map((value) => value.toFixed(0)).join(", ")}]</span>
      <span>head [${person.head_point.map((value) => value.toFixed(0)).join(", ")}]</span>
      <span>dx ${person.dx.toFixed(1)} / dy ${person.dy.toFixed(1)}</span>
      <span>confidence ${(person.confidence * 100).toFixed(1)}%</span>
      <span>${keypoints || "keypoints -"}</span>
    `;
    els.peopleList.appendChild(item);
  });
}

async function refreshScreens() {
  requireTauri();
  const screens = await invoke("list_screens");
  state.screens = screens;
  els.screenSelect.innerHTML = "";
  screens.forEach((screen) => {
    const option = document.createElement("option");
    option.value = screen.id;
    option.textContent = `${screen.name} (${screen.size[0]}x${screen.size[1]})${screen.primary ? " *" : ""}`;
    els.screenSelect.appendChild(option);
  });
  const primary = screens.find((screen) => screen.primary) || screens[0];
  if (primary) {
    state.selectedScreenId = primary.id;
    els.screenSelect.value = primary.id;
  }
  return screens;
}

function updatePreviewFrameVisibility() {
  const wrap = els.monitorCanvas?.parentElement;
  wrap?.classList.toggle("is-hidden", !state.previewFrameEnabled);
  if (els.previewFrameToggle) {
    els.previewFrameToggle.checked = state.previewFrameEnabled;
  }
}

function setPreviewFrameEnabled(enabled) {
  state.previewFrameEnabled = enabled;
  localStorage.setItem("autoaim.previewFrame", enabled ? "true" : "false");
  updatePreviewFrameVisibility();
  if (!enabled && els.monitorCanvas) {
    const ctx = els.monitorCanvas.getContext("2d");
    ctx.clearRect(0, 0, els.monitorCanvas.width, els.monitorCanvas.height);
  }
}

function currentInferenceOptions() {
  const confidence = Number.parseFloat(els.confidenceInput.value || "0.25");
  return {
    modelPath: els.modelPath.value.trim(),
    provider: els.providerSelect.value,
    confidenceThreshold: Number.isFinite(confidence) ? confidence : 0.25,
  };
}

async function openOverlayForSelectedScreen() {
  const screenId = els.screenSelect.value || state.selectedScreenId;
  if (!screenId) {
    throw new Error(t("noScreens"));
  }

  writeFileLog("ui", "Opening overlay", {
    screenId,
    options: currentInferenceOptions(),
  });
  await invoke("open_overlay_window", {
    screenId,
    ...currentInferenceOptions(),
  });
}

async function pollLiveSnapshot(sessionId = state.liveSessionId) {
  if (state.livePolling) {
    return false;
  }

  const screenId = els.screenSelect.value || state.selectedScreenId;
  if (!screenId) {
    return false;
  }

  state.livePolling = true;
  els.modelStatusReadout.textContent = t("modelLoading");
  try {
    const snapshot = await invoke("live_monitor_snapshot", {
      screenId,
      ...currentInferenceOptions(),
    });
    if (sessionId !== state.liveSessionId) {
      return false;
    }

    state.lastCursor = snapshot.cursor;
    state.lastFrame = snapshot.frame;
    state.lastSnapshotSummary = {
      screen_id: snapshot.screen_id,
      cursor: snapshot.cursor,
      cursor_on_screen: snapshot.cursor_on_screen,
      people_count: (snapshot.people || []).length,
      provider: snapshot.provider,
      model_status: snapshot.model_status,
      capture_status: snapshot.capture_status,
      frame_size: snapshot.frame?.frame_size,
      screen_size: snapshot.frame?.screen_size,
      capture_backend: snapshot.frame?.capture_backend,
    };
    const people = snapshot.people || [];
    state.detectedPeople = people;
    els.cursorReadout.textContent = `${snapshot.cursor[0].toFixed(0)}, ${snapshot.cursor[1].toFixed(0)}`;
    els.peopleReadout.textContent = people.length;
    els.modelStatusReadout.textContent = snapshot.model_status || t("modelLoaded");
    els.captureStatusReadout.textContent = snapshot.capture_status || t("nativeCapture");
    renderPeople(people);
    if (state.previewFrameEnabled) {
      drawMonitor(snapshot, people);
    }
    return true;
  } finally {
    state.livePolling = false;
  }
}

function clearLiveTimer() {
  if (state.liveTimer) {
    clearTimeout(state.liveTimer);
    state.liveTimer = null;
  }
}

function scheduleLivePoll(sessionId) {
  clearLiveTimer();
  if (!state.liveRunning || sessionId !== state.liveSessionId) {
    return;
  }

  state.liveTimer = setTimeout(async () => {
    state.liveTimer = null;
    if (!state.liveRunning || sessionId !== state.liveSessionId) {
      return;
    }

    try {
      await pollLiveSnapshot(sessionId);
      scheduleLivePoll(sessionId);
    } catch (error) {
      setStatus(error?.message || String(error), "error");
      stopLiveMonitor();
    }
  }, LIVE_POLL_INTERVAL_MS);
}

function stopLiveMonitor() {
  log("Live monitor stop requested");
  state.liveRunning = false;
  state.liveSessionId += 1;
  clearLiveTimer();
  state.detectedPeople = [];
  state.lastFrame = null;
  state.lastSnapshotSummary = null;
  renderPeople([]);
  els.liveState.textContent = t("liveStopped");
  setStatus(t("liveStoppedStatus"), "ready");
  invoke?.("close_overlay_window").catch?.(() => {});
}

function updateMetrics(summary) {
  if (!els.framesMetric) {
    return;
  }
  els.framesMetric.textContent = summary.frame_count ?? "-";
  els.objectsMetric.textContent = summary.object_count ?? "-";
  els.targetsMetric.textContent = summary.target_count ?? "-";
  els.confidenceMetric.textContent = typeof summary.mean_confidence === "number" ? summary.mean_confidence.toFixed(4) : "-";
  els.distanceMetric.textContent = typeof summary.mean_distance === "number" ? summary.mean_distance.toFixed(2) : "-";
}

async function runAction(statusText, action) {
  try {
    requireTauri();
    setBusy(true);
    setStatus(statusText, "busy");
    await action();
  } catch (error) {
    const message = error?.message || String(error);
    setStatus(message, "error");
    log(message);
  } finally {
    setBusy(false);
  }
}

function updateResultHasAvailableUpdate(result) {
  if (typeof result?.update_available === "boolean") {
    return result.update_available;
  }

  const output = String(result?.output || "").toLowerCase();
  if (output.includes("already up to date")) {
    return false;
  }
  return output.includes("incremental update available");
}

function renderUpdateStatus() {
  if (!els.updateStatusBtn) {
    return;
  }

  let key = "updateStatusIdle";
  let stateName = "idle";
  if (state.updateCheckRunning) {
    key = "updateStatusChecking";
    stateName = "busy";
  } else if (state.updateAvailable) {
    key = "updateStatusReady";
    stateName = "ready";
  } else if (state.lastUpdateResult?.update_available === false) {
    key = "updateStatusCurrent";
    stateName = "current";
  }

  els.updateStatusBtn.textContent = t(key);
  els.updateStatusBtn.dataset.state = stateName;
  els.updateStatusBtn.disabled = state.updateCheckRunning;
}

function updateVersionSummary(result) {
  const installed = result?.installed_version;
  const latest = result?.latest_version;
  if (installed && latest) {
    return `${installed} -> ${latest}`;
  }
  return "";
}

function showUpdateDialog(result) {
  if (!els.updateDialog) {
    return;
  }
  const versionSummary = updateVersionSummary(result);
  els.updateVersionText.textContent = versionSummary;
  els.updateDialog.hidden = false;
  els.restartUpdateBtn?.focus();
}

function hideUpdateDialog() {
  if (els.updateDialog) {
    els.updateDialog.hidden = true;
  }
}

async function checkForUpdates(options = {}) {
  const manual = Boolean(options.manual);
  if (state.updateCheckRunning) {
    return;
  }

  try {
    requireTauri();
  } catch (error) {
    if (manual) {
      setStatus(error?.message || String(error), "error");
      log(error?.message || String(error));
    }
    return;
  }

  state.updateCheckRunning = true;
  renderUpdateStatus();
  if (manual) {
    setStatus(t("checkingUpdates"), "busy");
  }

  try {
    const result = await invoke("check_updates", { installDir: null });
    if (!result.success) {
      throw new Error(result.output || t("updateCheckFailed"));
    }
    state.lastUpdateResult = result;
    state.updateAvailable = updateResultHasAvailableUpdate(result);

    if (state.updateAvailable) {
      setStatus(t("updateAvailable"), "warning");
      log(t("updateAvailable"), result);
      showUpdateDialog(result);
    } else if (manual) {
      setStatus(t("noUpdateAvailable"), "success");
      log(t("noUpdateAvailable"), result);
    }
  } catch (error) {
    if (manual) {
      const message = error?.message || String(error);
      setStatus(t("updateCheckFailed"), "error");
      log(`${t("updateCheckFailed")} ${message}`);
    }
  } finally {
    state.updateCheckRunning = false;
    renderUpdateStatus();
  }
}

async function restartAndApplyUpdate() {
  try {
    requireTauri();
    if (els.restartUpdateBtn) {
      els.restartUpdateBtn.disabled = true;
    }
    if (els.dismissUpdateBtn) {
      els.dismissUpdateBtn.disabled = true;
    }
    setStatus(t("applyingUpdate"), "busy");
    const result = await invoke("apply_update", { installDir: null });
    setStatus(t("applyUpdateStarted"), result.success ? "success" : "warning");
    log(t("applyUpdateStarted"), result);
  } catch (error) {
    const message = error?.message || String(error);
    setStatus(t("updateApplyFailed"), "error");
    log(`${t("updateApplyFailed")} ${message}`);
    if (els.restartUpdateBtn) {
      els.restartUpdateBtn.disabled = false;
    }
    if (els.dismissUpdateBtn) {
      els.dismissUpdateBtn.disabled = false;
    }
  }
}

on(els.languageSelect, "change", (event) => {
  applyLanguage(event.target.value);
});

on(els.chooseInput, "click", async () => {
  await runAction(t("selected"), async () => {
    const selected = await dialog.open({
      filters: [{ name: "JSONL", extensions: ["jsonl"] }],
      multiple: false,
    });
    if (selected) {
      els.inputPath.value = selected;
      setStatus(t("selected"), "ready");
    }
  });
});

on(els.chooseOutput, "click", async () => {
  await runAction(t("outputSelected"), async () => {
    const selected = await dialog.save({
      defaultPath: "events.jsonl",
      filters: [{ name: "JSONL", extensions: ["jsonl"] }],
    });
    if (selected) {
      els.outputPath.value = selected;
      setStatus(t("outputSelected"), "ready");
    }
  });
});

on(els.refreshScreensBtn, "click", async () => {
  await runAction(t("screensLoaded"), async () => {
    const screens = await refreshScreens();
    setStatus(screens.length ? t("screensLoaded") : t("noScreens"), screens.length ? "success" : "warning");
  });
});

on(els.screenSelect, "change", (event) => {
  state.selectedScreenId = event.target.value;
});

on(els.startLiveBtn, "click", async () => {
  await runAction(t("liveStarting"), async () => {
    if (state.livePolling) {
      throw new Error(t("liveBusy"));
    }
    if (!state.screens.length) {
      await refreshScreens();
    }
    state.liveRunning = false;
    clearLiveTimer();
    state.liveSessionId += 1;
    if (!state.previewFrameEnabled) {
      setPreviewFrameEnabled(true);
    }
    log("Live monitor start requested", {
      previewFrameEnabled: state.previewFrameEnabled,
      screenId: els.screenSelect.value || state.selectedScreenId,
      provider: els.providerSelect.value,
      modelPath: els.modelPath.value.trim(),
      confidence: els.confidenceInput.value,
    });
    const liveSessionId = state.liveSessionId;
    await openOverlayForSelectedScreen();
    state.liveRunning = true;
    const didPoll = await pollLiveSnapshot(liveSessionId);
    if (!didPoll) {
      throw new Error(t("liveBusy"));
    }
    scheduleLivePoll(liveSessionId);
    els.liveState.textContent = t("liveRunning");
    setStatus(t("liveStarted"), "success");
  });
});

on(els.stopLiveBtn, "click", () => {
  stopLiveMonitor();
});

on(els.showOverlayBtn, "click", async () => {
  await runAction(t("showOverlay"), async () => {
    await openOverlayForSelectedScreen();
    setStatus(t("showOverlay"), "success");
  });
});

on(els.hideOverlayBtn, "click", async () => {
  await runAction(t("hideOverlay"), async () => {
    await invoke("close_overlay_window");
    setStatus(t("hideOverlay"), "ready");
  });
});

on(els.previewFrameToggle, "change", (event) => {
  const enabled = Boolean(event.target.checked);
  setPreviewFrameEnabled(enabled);
  log("Frame preview toggled", { enabled });
  if (!state.liveRunning) {
    return;
  }
  if (enabled) {
    const lastSnapshot = state.lastSnapshotSummary && state.lastFrame
      ? {
          screen_id: state.lastSnapshotSummary.screen_id,
          frame: state.lastFrame,
          cursor: state.lastCursor,
          cursor_on_screen: state.lastSnapshotSummary.cursor_on_screen,
        }
      : null;
    if (lastSnapshot) {
      drawMonitor(lastSnapshot);
    }
  }
});

on(els.validateBtn, "click", async () => {
  await runAction(t("validating"), async () => {
    const diagnostics = await invoke("validate_dataset", { path: requireInputPath() });
    if (diagnostics.length === 0) {
      setStatus(t("validationOk"), "success");
      log(t("validationOk"));
    } else {
      setStatus(t("validationFailed"), "warning");
      log(t("validationFailed"), diagnostics);
    }
  });
});

on(els.evaluateBtn, "click", async () => {
  await runAction(t("evaluating"), async () => {
    const result = await invoke("evaluate_dataset", { path: requireInputPath() });
    updateMetrics(result.summary);
    setStatus(t("evaluationDone"), "success");
    log(t("evaluationDone"), result);
  });
});

on(els.positionsBtn, "click", async () => {
  await runAction(t("positionsPreviewing"), async () => {
    const result = await invoke("preview_person_positions", { path: requireInputPath(), limit: 50 });
    setStatus(t("positionsDone"), "success");
    log(t("positionsDone"), result);
  });
});

on(els.previewBtn, "click", async () => {
  await runAction(t("previewing"), async () => {
    const result = await invoke("preview_events", { path: requireInputPath(), limit: 20 });
    setStatus(t("previewDone"), "success");
    log(t("previewDone"), result);
  });
});

on(els.writeBtn, "click", async () => {
  await runAction(t("writing"), async () => {
    const result = await invoke("write_events", {
      inputPath: requireInputPath(),
      outputPath: requireOutputPath(),
    });
    setStatus(t("writeDone"), "success");
    log(t("writeDone"), result);
  });
});

on(els.showRuntimeBtn, "click", async () => {
  await runAction(t("runtimeReady"), async () => {
    const confidence = Number.parseFloat(els.confidenceInput.value || "0.25");
    const config = await invoke("inference_runtime_config", {
      provider: els.providerSelect.value,
      modelPath: els.modelPath.value.trim(),
      deviceId: 0,
      confidenceThreshold: Number.isFinite(confidence) ? confidence : 0.25,
    });
    setStatus(t("runtimeReady"), "success");
    log(t("runtimeReady"), config);
  });
});

on(els.updateStatusBtn, "click", async () => {
  if (state.updateAvailable && state.lastUpdateResult) {
    showUpdateDialog(state.lastUpdateResult);
    return;
  }
  await checkForUpdates({ manual: true });
});

on(els.dismissUpdateBtn, "click", () => {
  hideUpdateDialog();
});

on(els.restartUpdateBtn, "click", async () => {
  await restartAndApplyUpdate();
});

on(els.updateDialog, "click", (event) => {
  if (event.target === els.updateDialog) {
    hideUpdateDialog();
  }
});

on(document, "keydown", (event) => {
  if (event.key === "Escape" && !els.updateDialog?.hidden) {
    hideUpdateDialog();
  }
});

on(els.copyDiagnosticsBtn, "click", async () => {
  await runAction(t("copyDiagnostics"), async () => {
    await copyDiagnostics();
  });
});

on(els.clearLog, "click", () => {
  if (els.logOutput) {
    els.logOutput.textContent = "";
  }
});

applyLanguage(state.language);

if (!invoke) {
  setStatus(t("noTauri"), "error");
  log(t("noTauri"));
} else {
  refreshScreens()
    .then((screens) => {
      if (!screens.length) {
        setStatus(t("noScreens"), "warning");
      }
    })
    .catch((error) => log(error?.message || String(error)));
  invoke("app_info")
    .then((info) => log("AutoAim Review", info))
    .catch((error) => log(error?.message || String(error)));
  setTimeout(() => {
    checkForUpdates({ manual: false });
  }, AUTO_UPDATE_CHECK_DELAY_MS);
}
