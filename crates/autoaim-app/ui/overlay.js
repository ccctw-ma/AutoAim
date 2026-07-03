const tauriApi = window.__TAURI__ || {};
const eventApi = tauriApi.event;
const canvas = document.getElementById("overlayCanvas");
const ctx = canvas.getContext("2d");
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
let latestSnapshot = null;
let overlayCursorLogCount = 0;

function overlayLog(message, data) {
  if (!tauriApi.tauri?.invoke) {
    return;
  }
  const payload = typeof data === "undefined" ? null : JSON.stringify(data);
  tauriApi.tauri.invoke("frontend_log", { scope: "overlay", message, payload }).catch(() => {});
}

function resizeCanvas(width, height) {
  if (canvas.width !== width || canvas.height !== height) {
    canvas.width = width;
    canvas.height = height;
  }
}

function clearOverlay() {
  ctx.clearRect(0, 0, canvas.width, canvas.height);
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

function drawSkeleton(keypoints, projectPoint) {
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

function drawSnapshot(snapshot) {
  latestSnapshot = snapshot;
  resizeCanvas(snapshot.screen_size[0], snapshot.screen_size[1]);
  clearOverlay();

  ctx.lineWidth = 2;
  ctx.strokeStyle = "#f97316";
  ctx.fillStyle = "#f97316";

  (snapshot.people || []).forEach((person) => {
    const [x, y, w, h] = person.bbox;
    const projectPoint = (point) => [
      point[0] - snapshot.screen_origin[0],
      point[1] - snapshot.screen_origin[1],
    ];
    const rectX = x - snapshot.screen_origin[0];
    const rectY = y - snapshot.screen_origin[1];
    ctx.strokeRect(rectX, rectY, w, h);

    drawSkeleton(person.keypoints, projectPoint);

    const [headX, headY] = projectPoint(person.head_point);
    ctx.beginPath();
    ctx.arc(headX, headY, 5, 0, Math.PI * 2);
    ctx.fill();

    ctx.fillStyle = "#a7f3d0";
    (person.keypoints || [])
      .filter((keypoint) => keypoint.score >= KEYPOINT_SCORE_THRESHOLD)
      .forEach((keypoint) => {
        const [keypointX, keypointY] = projectPoint(keypoint.point);
        ctx.beginPath();
        ctx.arc(keypointX, keypointY, 3, 0, Math.PI * 2);
        ctx.fill();
      });
    ctx.fillStyle = "#f97316";
  });

  if (snapshot.cursor_on_screen) {
    const cursorX = snapshot.cursor[0] - snapshot.screen_origin[0];
    const cursorY = snapshot.cursor[1] - snapshot.screen_origin[1];
    ctx.strokeStyle = "#22d3ee";
    ctx.fillStyle = "#22d3ee";
    ctx.beginPath();
    ctx.moveTo(cursorX - 12, cursorY);
    ctx.lineTo(cursorX + 12, cursorY);
    ctx.moveTo(cursorX, cursorY - 12);
    ctx.lineTo(cursorX, cursorY + 12);
    ctx.stroke();
    ctx.beginPath();
    ctx.arc(cursorX, cursorY, 4, 0, Math.PI * 2);
    ctx.fill();
  }
}

function drawCursorSnapshot(snapshot) {
  const merged =
    latestSnapshot && latestSnapshot.screen_id === snapshot.screen_id
      ? { ...latestSnapshot, cursor: snapshot.cursor, cursor_on_screen: snapshot.cursor_on_screen }
      : { ...snapshot, people: [] };
  drawSnapshot(merged);
}

if (eventApi?.listen) {
  eventApi.listen("overlay_snapshot", (event) => {
    overlayLog("overlay_snapshot", {
      people: (event.payload.people || []).length,
      cursor: event.payload.cursor,
      cursor_on_screen: event.payload.cursor_on_screen,
      screen_id: event.payload.screen_id,
    });
    drawSnapshot(event.payload);
  });
  eventApi.listen("overlay_cursor", (event) => {
    overlayCursorLogCount += 1;
    if (overlayCursorLogCount % 60 === 1) {
      overlayLog("overlay_cursor", {
        cursor: event.payload.cursor,
        cursor_on_screen: event.payload.cursor_on_screen,
        screen_id: event.payload.screen_id,
      });
    }
    drawCursorSnapshot(event.payload);
  });
}
