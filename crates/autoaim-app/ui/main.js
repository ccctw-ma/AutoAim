const tauriApi = window.__TAURI__ || {};
const invoke = tauriApi.tauri?.invoke;
const dialog = tauriApi.dialog;

const i18n = {
  en: {
    eyebrow: "Rust + Tauri live monitor",
    title: "AutoAim Review",
    subtitle: "Select a screen, start live monitoring, and inspect cursor plus person positions in real time.",
    languageLabel: "Language",
    liveKicker: "Live",
    liveTitle: "Monitor a screen",
    screen: "Screen",
    refreshScreens: "Refresh screens",
    startLive: "Start now",
    stopLive: "Stop",
    liveStopped: "Stopped",
    liveRunning: "Running",
    liveStarting: "Starting live monitor...",
    liveStarted: "Live monitor started.",
    liveStoppedStatus: "Live monitor stopped.",
    nativeCapture: "Native Windows capture",
    modelLoading: "Capturing screen and running detector...",
    modelLoaded: "Native detector ready.",
    modelUnavailable: "Person detector unavailable.",
    mousePosition: "Mouse position",
    peopleCount: "People",
    modelStatus: "Model",
    captureStatus: "Capture",
    peopleKicker: "Positions",
    peopleTitle: "Detected people",
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
    runtimeTitle: "MoveNet pose inference and updates",
    provider: "Provider",
    threshold: "Confidence",
    modelPath: "Model path",
    showRuntime: "Show config",
    checkUpdates: "Check updates",
    applyUpdate: "Apply update",
    safetyTitle: "Safety boundary",
    safetyText: "This app never moves the cursor, clicks, injects input, attaches to processes, or controls games.",
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
    consoleKicker: "Output",
    consoleTitle: "Command result",
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
    eyebrow: "Rust + Tauri 实时监控",
    title: "AutoAim Review",
    subtitle: "选择屏幕后立即开始，实时查看鼠标位置和画面中的人物位置。",
    languageLabel: "语言",
    liveKicker: "实时",
    liveTitle: "监控屏幕",
    screen: "屏幕",
    refreshScreens: "刷新屏幕",
    startLive: "立即开始",
    stopLive: "停止",
    liveStopped: "已停止",
    liveRunning: "运行中",
    liveStarting: "正在启动实时监控...",
    liveStarted: "实时监控已启动。",
    liveStoppedStatus: "实时监控已停止。",
    nativeCapture: "Windows 原生采集",
    modelLoading: "正在采集屏幕并运行检测器...",
    modelLoaded: "原生检测器已就绪。",
    modelUnavailable: "人物检测模型不可用。",
    mousePosition: "鼠标位置",
    peopleCount: "人物数",
    modelStatus: "模型",
    captureStatus: "采集",
    peopleKicker: "位置",
    peopleTitle: "识别到的人物",
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
    runtimeTitle: "MoveNet 姿态推理与更新",
    provider: "推理后端",
    threshold: "置信度",
    modelPath: "模型路径",
    showRuntime: "显示配置",
    checkUpdates: "检查更新",
    applyUpdate: "立即更新",
    safetyTitle: "安全边界",
    safetyText: "本应用不会移动鼠标、点击、注入输入、附加进程，也不会控制游戏。",
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
    consoleKicker: "输出",
    consoleTitle: "命令结果",
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

const state = {
  language: localStorage.getItem("autoaim.language") || "en",
  liveTimer: null,
  detectedPeople: [],
  lastCursor: [0, 0],
  lastFrame: null,
  screens: [],
  selectedScreenId: null,
};

const $ = (id) => document.getElementById(id);

const els = {
  languageSelect: $("languageSelect"),
  statusPill: $("statusPill"),
  screenSelect: $("screenSelect"),
  refreshScreensBtn: $("refreshScreensBtn"),
  startLiveBtn: $("startLiveBtn"),
  stopLiveBtn: $("stopLiveBtn"),
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
  checkUpdatesBtn: $("checkUpdatesBtn"),
  applyUpdateBtn: $("applyUpdateBtn"),
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
  els.languageSelect.value = language;

  document.querySelectorAll("[data-i18n]").forEach((node) => {
    node.textContent = t(node.dataset.i18n);
  });
}

function setStatus(message, tone = "ready") {
  els.statusPill.textContent = message;
  els.statusPill.dataset.tone = tone;
}

function log(message, data) {
  const timestamp = new Date().toLocaleTimeString();
  const text = typeof data === "undefined" ? message : `${message}\n${JSON.stringify(data, null, 2)}`;
  els.logOutput.textContent = `[${timestamp}] ${text}\n\n${els.logOutput.textContent}`;
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
    els.checkUpdatesBtn,
    els.applyUpdateBtn,
    els.refreshScreensBtn,
    els.startLiveBtn,
    els.stopLiveBtn,
    els.chooseInput,
    els.chooseOutput,
  ].forEach((button) => {
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
    ctx.fillStyle = "#07111f";
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

function drawMonitor(snapshot, people = state.detectedPeople) {
  const canvas = els.monitorCanvas;
  const ctx = canvas.getContext("2d");
  const width = canvas.width;
  const height = canvas.height;
  ctx.clearRect(0, 0, width, height);
  drawNativeFrame(ctx, canvas, snapshot.frame);

  const grid = 48;
  ctx.strokeStyle = "rgba(148, 163, 184, 0.13)";
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
  ctx.strokeStyle = "#22d3ee";
  ctx.fillStyle = "#22d3ee";
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

  ctx.strokeStyle = "#f97316";
  ctx.fillStyle = "#f97316";
  people.forEach((person) => {
    const [x, y, w, h] = person.bbox;
    const rectX = (x - originX) * scaleX;
    const rectY = (y - originY) * scaleY;
    ctx.strokeRect(rectX, rectY, w * scaleX, h * scaleY);
    const headX = (person.head_point[0] - originX) * scaleX;
    const headY = (person.head_point[1] - originY) * scaleY;
    ctx.beginPath();
    ctx.arc(headX, headY, 5, 0, Math.PI * 2);
    ctx.fill();

    if (Array.isArray(person.keypoints)) {
      ctx.fillStyle = "#a7f3d0";
      person.keypoints
        .filter((keypoint) => keypoint.score >= 0.2)
        .forEach((keypoint) => {
          const keypointX = (keypoint.point[0] - originX) * scaleX;
          const keypointY = (keypoint.point[1] - originY) * scaleY;
          ctx.beginPath();
          ctx.arc(keypointX, keypointY, 3, 0, Math.PI * 2);
          ctx.fill();
        });
      ctx.fillStyle = "#f97316";
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

async function pollLiveSnapshot() {
  const screenId = els.screenSelect.value || state.selectedScreenId;
  if (!screenId) {
    return;
  }
  els.modelStatusReadout.textContent = t("modelLoading");
  const confidence = Number.parseFloat(els.confidenceInput.value || "0.25");
  const snapshot = await invoke("live_monitor_snapshot", {
    screenId,
    modelPath: els.modelPath.value.trim(),
    provider: els.providerSelect.value,
    confidenceThreshold: Number.isFinite(confidence) ? confidence : 0.25,
  });
  state.lastCursor = snapshot.cursor;
  state.lastFrame = snapshot.frame;
  const people = snapshot.people || [];
  state.detectedPeople = people;
  els.cursorReadout.textContent = `${snapshot.cursor[0].toFixed(0)}, ${snapshot.cursor[1].toFixed(0)}`;
  els.peopleReadout.textContent = people.length;
  els.modelStatusReadout.textContent = snapshot.model_status || t("modelLoaded");
  els.captureStatusReadout.textContent = snapshot.capture_status || t("nativeCapture");
  renderPeople(people);
  drawMonitor(snapshot, people);
}

function stopLiveMonitor() {
  if (state.liveTimer) {
    clearInterval(state.liveTimer);
    state.liveTimer = null;
  }
  state.detectedPeople = [];
  state.lastFrame = null;
  renderPeople([]);
  els.liveState.textContent = t("liveStopped");
  setStatus(t("liveStoppedStatus"), "ready");
}

function updateMetrics(summary) {
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

els.languageSelect.addEventListener("change", (event) => {
  applyLanguage(event.target.value);
});

els.chooseInput.addEventListener("click", async () => {
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

els.chooseOutput.addEventListener("click", async () => {
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

els.refreshScreensBtn.addEventListener("click", async () => {
  await runAction(t("screensLoaded"), async () => {
    const screens = await refreshScreens();
    setStatus(screens.length ? t("screensLoaded") : t("noScreens"), screens.length ? "success" : "warning");
  });
});

els.screenSelect.addEventListener("change", (event) => {
  state.selectedScreenId = event.target.value;
});

els.startLiveBtn.addEventListener("click", async () => {
  await runAction(t("liveStarting"), async () => {
    if (!state.screens.length) {
      await refreshScreens();
    }
    await pollLiveSnapshot();
    if (state.liveTimer) {
      clearInterval(state.liveTimer);
    }
    state.liveTimer = setInterval(() => {
      pollLiveSnapshot().catch((error) => {
        setStatus(error?.message || String(error), "error");
        stopLiveMonitor();
      });
    }, 250);
    els.liveState.textContent = t("liveRunning");
    setStatus(t("liveStarted"), "success");
  });
});

els.stopLiveBtn.addEventListener("click", () => {
  stopLiveMonitor();
});

els.validateBtn.addEventListener("click", async () => {
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

els.evaluateBtn.addEventListener("click", async () => {
  await runAction(t("evaluating"), async () => {
    const result = await invoke("evaluate_dataset", { path: requireInputPath() });
    updateMetrics(result.summary);
    setStatus(t("evaluationDone"), "success");
    log(t("evaluationDone"), result);
  });
});

els.positionsBtn.addEventListener("click", async () => {
  await runAction(t("positionsPreviewing"), async () => {
    const result = await invoke("preview_person_positions", { path: requireInputPath(), limit: 50 });
    setStatus(t("positionsDone"), "success");
    log(t("positionsDone"), result);
  });
});

els.previewBtn.addEventListener("click", async () => {
  await runAction(t("previewing"), async () => {
    const result = await invoke("preview_events", { path: requireInputPath(), limit: 20 });
    setStatus(t("previewDone"), "success");
    log(t("previewDone"), result);
  });
});

els.writeBtn.addEventListener("click", async () => {
  await runAction(t("writing"), async () => {
    const result = await invoke("write_events", {
      inputPath: requireInputPath(),
      outputPath: requireOutputPath(),
    });
    setStatus(t("writeDone"), "success");
    log(t("writeDone"), result);
  });
});

els.showRuntimeBtn.addEventListener("click", async () => {
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

els.checkUpdatesBtn.addEventListener("click", async () => {
  await runAction(t("checkingUpdates"), async () => {
    const result = await invoke("check_updates", { installDir: null });
    setStatus(t("updateCheckDone"), result.success ? "success" : "warning");
    log(t("updateCheckDone"), result);
  });
});

els.applyUpdateBtn.addEventListener("click", async () => {
  await runAction(t("applyingUpdate"), async () => {
    const result = await invoke("apply_update", { installDir: null });
    setStatus(t("applyUpdateStarted"), result.success ? "success" : "warning");
    log(t("applyUpdateStarted"), result);
  });
});

els.clearLog.addEventListener("click", () => {
  els.logOutput.textContent = "";
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
}
