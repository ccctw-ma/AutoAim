const tauriApi = window.__TAURI__ || {};
const eventApi = tauriApi.event;
const canvas = document.getElementById("overlayCanvas");
const ctx = canvas.getContext("2d");

function resizeCanvas(width, height) {
  if (canvas.width !== width || canvas.height !== height) {
    canvas.width = width;
    canvas.height = height;
  }
}

function clearOverlay() {
  ctx.clearRect(0, 0, canvas.width, canvas.height);
}

function drawSnapshot(snapshot) {
  resizeCanvas(snapshot.screen_size[0], snapshot.screen_size[1]);
  clearOverlay();

  ctx.lineWidth = 2;
  ctx.strokeStyle = "#f97316";
  ctx.fillStyle = "#f97316";

  (snapshot.people || []).forEach((person) => {
    const [x, y, w, h] = person.bbox;
    const rectX = x - snapshot.screen_origin[0];
    const rectY = y - snapshot.screen_origin[1];
    ctx.strokeRect(rectX, rectY, w, h);

    const headX = person.head_point[0] - snapshot.screen_origin[0];
    const headY = person.head_point[1] - snapshot.screen_origin[1];
    ctx.beginPath();
    ctx.arc(headX, headY, 5, 0, Math.PI * 2);
    ctx.fill();

    ctx.fillStyle = "#a7f3d0";
    (person.keypoints || [])
      .filter((keypoint) => keypoint.score >= 0.2)
      .forEach((keypoint) => {
        const keypointX = keypoint.point[0] - snapshot.screen_origin[0];
        const keypointY = keypoint.point[1] - snapshot.screen_origin[1];
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

if (eventApi?.listen) {
  eventApi.listen("overlay_snapshot", (event) => {
    drawSnapshot(event.payload);
  });
}
