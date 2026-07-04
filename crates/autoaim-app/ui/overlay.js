const tauriApi = window.__TAURI__ || {};
const eventApi = tauriApi.event;
const poseCanvas = document.getElementById("overlayCanvas");
const cursorCanvas = document.getElementById("cursorCanvas");
const poseCtx = poseCanvas.getContext("2d");
const cursorCtx = cursorCanvas.getContext("2d");
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
const CURSOR_EVENT_SUPPRESS_AFTER_SNAPSHOT_MS = 80;
let latestSnapshot = null;
let latestCursorSnapshot = null;
let pendingPoseFrame = 0;
let pendingCursorFrame = 0;
let lastCursorRect = null;
let lastSnapshotReceivedAt = 0;
let overlaySnapshotLogCount = 0;
let overlayCursorLogCount = 0;

function overlayLog(message, data) {
  if (!tauriApi.tauri?.invoke) {
    return;
  }
  const payload = typeof data === "undefined" ? null : JSON.stringify(data);
  tauriApi.tauri.invoke("frontend_log", { scope: "overlay", message, payload }).catch(() => {});
}

function resizeCanvas(canvas, width, height) {
  if (canvas.width !== width || canvas.height !== height) {
    canvas.width = width;
    canvas.height = height;
  }
}

function resizeLayers(width, height) {
  resizeCanvas(poseCanvas, width, height);
  resizeCanvas(cursorCanvas, width, height);
}

function requestOverlayFrame(callback) {
  if (typeof window.requestAnimationFrame === "function") {
    return window.requestAnimationFrame(callback);
  }
  return window.setTimeout(callback, 16);
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
  ctx.strokeStyle = "#a7f3d0";
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

function drawPrediction(ctx, person, projectPoint) {
  if (!Array.isArray(person.predicted_head_point) || !Array.isArray(person.velocity)) {
    return;
  }
  const [headX, headY] = projectPoint(person.head_point);
  const [predictedX, predictedY] = projectPoint(person.predicted_head_point);
  const velocity = person.velocity;
  const speed = Math.hypot(velocity[0], velocity[1]);
  const curveDx = predictedX - headX;
  const curveDy = predictedY - headY;
  const curveLength = Math.hypot(curveDx, curveDy);
  const bend = Math.min(28, Math.max(8, curveLength * 0.18));
  const normalX = curveLength > 0 ? -curveDy / curveLength : 0;
  const normalY = curveLength > 0 ? curveDx / curveLength : 0;
  const controlX = headX + curveDx * 0.55 + normalX * bend;
  const controlY = headY + curveDy * 0.55 + normalY * bend;

  ctx.save();
  ctx.setLineDash([10, 8]);
  ctx.lineWidth = 3;
  ctx.strokeStyle = "#facc15";
  ctx.beginPath();
  ctx.moveTo(headX, headY);
  ctx.quadraticCurveTo(controlX, controlY, predictedX, predictedY);
  ctx.stroke();
  ctx.setLineDash([]);

  ctx.lineWidth = 2;
  ctx.strokeStyle = "#fde68a";
  ctx.fillStyle = "#facc15";
  ctx.beginPath();
  ctx.arc(predictedX, predictedY, 7, 0, Math.PI * 2);
  ctx.stroke();
  ctx.beginPath();
  ctx.moveTo(predictedX - 10, predictedY);
  ctx.lineTo(predictedX + 10, predictedY);
  ctx.moveTo(predictedX, predictedY - 10);
  ctx.lineTo(predictedX, predictedY + 10);
  ctx.stroke();

  if (speed > 1) {
    const velocityScale = Math.min(0.08, 80 / speed);
    ctx.strokeStyle = "#38bdf8";
    ctx.beginPath();
    ctx.moveTo(headX, headY);
    ctx.lineTo(headX + velocity[0] * velocityScale, headY + velocity[1] * velocityScale);
    ctx.stroke();
  }
  ctx.restore();
}

function drawPoseLayer(snapshot) {
  resizeLayers(snapshot.screen_size[0], snapshot.screen_size[1]);
  poseCtx.clearRect(0, 0, poseCanvas.width, poseCanvas.height);

  poseCtx.lineWidth = 2;
  poseCtx.strokeStyle = "#f97316";
  poseCtx.fillStyle = "#f97316";

  (snapshot.people || []).forEach((person) => {
    const [x, y, w, h] = person.bbox;
    const projectPoint = (point) => [
      point[0] - snapshot.screen_origin[0],
      point[1] - snapshot.screen_origin[1],
    ];
    const rectX = x - snapshot.screen_origin[0];
    const rectY = y - snapshot.screen_origin[1];
    poseCtx.strokeRect(rectX, rectY, w, h);

    drawSkeleton(poseCtx, person.keypoints, projectPoint);

    const [headX, headY] = projectPoint(person.head_point);
    poseCtx.beginPath();
    poseCtx.arc(headX, headY, 5, 0, Math.PI * 2);
    poseCtx.fill();
    drawPrediction(poseCtx, person, projectPoint);

    poseCtx.fillStyle = "#a7f3d0";
    (person.keypoints || [])
      .filter((keypoint) => keypoint.score >= KEYPOINT_SCORE_THRESHOLD)
      .forEach((keypoint) => {
        const [keypointX, keypointY] = projectPoint(keypoint.point);
        poseCtx.beginPath();
        poseCtx.arc(keypointX, keypointY, 3, 0, Math.PI * 2);
        poseCtx.fill();
      });
    poseCtx.fillStyle = "#f97316";
  });
}

function cursorClearRect(cursorX, cursorY) {
  const margin = 18;
  return [cursorX - margin, cursorY - margin, margin * 2, margin * 2];
}

function clearCursorRect(rect) {
  if (!rect) {
    return;
  }
  cursorCtx.clearRect(rect[0], rect[1], rect[2], rect[3]);
}

function drawCursorLayer(snapshot) {
  resizeLayers(snapshot.screen_size[0], snapshot.screen_size[1]);
  clearCursorRect(lastCursorRect);
  lastCursorRect = null;

  if (snapshot.cursor_on_screen) {
    const cursorX = snapshot.cursor[0] - snapshot.screen_origin[0];
    const cursorY = snapshot.cursor[1] - snapshot.screen_origin[1];
    lastCursorRect = cursorClearRect(cursorX, cursorY);
    cursorCtx.clearRect(lastCursorRect[0], lastCursorRect[1], lastCursorRect[2], lastCursorRect[3]);
    cursorCtx.strokeStyle = "#22d3ee";
    cursorCtx.fillStyle = "#22d3ee";
    cursorCtx.lineWidth = 2;
    cursorCtx.beginPath();
    cursorCtx.moveTo(cursorX - 12, cursorY);
    cursorCtx.lineTo(cursorX + 12, cursorY);
    cursorCtx.moveTo(cursorX, cursorY - 12);
    cursorCtx.lineTo(cursorX, cursorY + 12);
    cursorCtx.stroke();
    cursorCtx.beginPath();
    cursorCtx.arc(cursorX, cursorY, 4, 0, Math.PI * 2);
    cursorCtx.fill();
  }
}

function currentPoseSnapshot() {
  return latestSnapshot || latestCursorSnapshot;
}

function currentCursorSnapshot() {
  if (latestSnapshot) {
    if (latestCursorSnapshot && latestCursorSnapshot.screen_id === latestSnapshot.screen_id) {
      return {
        ...latestSnapshot,
        cursor: latestCursorSnapshot.cursor,
        cursor_on_screen: latestCursorSnapshot.cursor_on_screen,
      };
    }
    return latestSnapshot;
  }

  if (latestCursorSnapshot) {
    return { ...latestCursorSnapshot, people: [] };
  }
  return null;
}

function schedulePoseDraw() {
  if (pendingPoseFrame) {
    return;
  }
  pendingPoseFrame = requestOverlayFrame(() => {
    pendingPoseFrame = 0;
    const snapshot = currentPoseSnapshot();
    if (snapshot) {
      drawPoseLayer(snapshot);
    }
  });
}

function scheduleCursorDraw() {
  if (pendingCursorFrame) {
    return;
  }
  pendingCursorFrame = requestOverlayFrame(() => {
    pendingCursorFrame = 0;
    const snapshot = currentCursorSnapshot();
    if (snapshot) {
      drawCursorLayer(snapshot);
    }
  });
}

if (eventApi?.listen) {
  eventApi.listen("overlay_snapshot", (event) => {
    latestSnapshot = event.payload;
    latestCursorSnapshot = {
      screen_id: event.payload.screen_id,
      screen_origin: event.payload.screen_origin,
      screen_size: event.payload.screen_size,
      cursor: event.payload.cursor,
      cursor_on_screen: event.payload.cursor_on_screen,
    };
    lastSnapshotReceivedAt = performance.now();
    overlaySnapshotLogCount += 1;
    if (overlaySnapshotLogCount <= 3 || overlaySnapshotLogCount % 60 === 0) {
      overlayLog("overlay_snapshot", {
        people: (event.payload.people || []).length,
        cursor: event.payload.cursor,
        cursor_on_screen: event.payload.cursor_on_screen,
        screen_id: event.payload.screen_id,
      });
    }
    schedulePoseDraw();
    scheduleCursorDraw();
  });
  eventApi.listen("overlay_cursor", (event) => {
    const recentlyReceivedSnapshot =
      performance.now() - lastSnapshotReceivedAt < CURSOR_EVENT_SUPPRESS_AFTER_SNAPSHOT_MS;
    if (recentlyReceivedSnapshot && latestSnapshot?.screen_id === event.payload.screen_id) {
      return;
    }
    latestCursorSnapshot = event.payload;
    overlayCursorLogCount += 1;
    if (overlayCursorLogCount % 60 === 1) {
      overlayLog("overlay_cursor", {
        cursor: event.payload.cursor,
        cursor_on_screen: event.payload.cursor_on_screen,
        screen_id: event.payload.screen_id,
      });
    }
    scheduleCursorDraw();
  });
}
