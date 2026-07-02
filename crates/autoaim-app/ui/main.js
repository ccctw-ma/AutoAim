const tauriApi = window.__TAURI__ || {};
const invoke = tauriApi.tauri?.invoke;
const dialog = tauriApi.dialog;

const i18n = {
  en: {
    eyebrow: "Rust + Tauri desktop review console",
    title: "AutoAim Review",
    subtitle: "Inspect JSONL capture records, validate dataset quality, and produce review-only inference events.",
    languageLabel: "Language",
    workflowKicker: "Workflow",
    workflowTitle: "Review a frame dataset",
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
    runtimeTitle: "GPU inference and updates",
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
    nextText: "Live capture, ONNX inference, and overlay rendering are planned Rust crates and are intentionally disabled here.",
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
  },
  zh: {
    eyebrow: "Rust + Tauri 桌面审阅控制台",
    title: "AutoAim Review",
    subtitle: "检查 JSONL 采集记录，验证数据集质量，并生成仅用于审阅的推理事件。",
    languageLabel: "语言",
    workflowKicker: "流程",
    workflowTitle: "审阅帧数据集",
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
    runtimeTitle: "GPU 推理与更新",
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
    nextText: "实时采集、ONNX 推理和 overlay 渲染会作为 Rust crate 继续实现，此版本暂不启用。",
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
  },
};

const state = {
  language: localStorage.getItem("autoaim.language") || "en",
};

const $ = (id) => document.getElementById(id);

const els = {
  languageSelect: $("languageSelect"),
  statusPill: $("statusPill"),
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
    els.chooseInput,
    els.chooseOutput,
  ].forEach((button) => {
    button.disabled = isBusy;
  });
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
  invoke("app_info")
    .then((info) => log("AutoAim Review", info))
    .catch((error) => log(error?.message || String(error)));
}
