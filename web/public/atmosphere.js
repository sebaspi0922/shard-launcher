// Fondo «Grafito»: los mismos anillos y puntos pastel que la app
// (app/lib/ui/widgets/atmosphere.dart). Van y vuelven en 18 s, cada forma con
// su propio factor, y se quedan quietos con «reducir movimiento».

const SHAPES = [
  // x, fx, y, fy, sx, lx, sy, ly, size, color, radius (null = círculo), contorno
  [-70, 0, -40, 0, 0.4, 0, 0, 1, 340, "rgba(174,188,255,.40)", null, true],
  [70, 0, 0, 0.38, -0.3, 0, 0, 1, 220, "rgba(196,182,255,.33)", null, true],
  [-210, 1, -250, 1, 1, 0, 0, -1, 260, "rgba(143,212,255,.31)", null, true],
  [0, 0.55, 18, 0, 0, -0.4, 0.2, 0, 92, "rgba(255,255,255,.22)", null, true],
  [36, 0, -120, 1, 0, 0.2, 0.3, 0, 14, "#9eb0ff", 3, false],
  [0, 0.22, 64, 0, 1, 0, 0, -0.3, 8, "#d2c6ff", null, false],
  [-120, 1, 0, 0.22, -0.5, 0, 0, 1, 10, "#8fd4ff", null, false],
  [0, 0.4, -70, 1, 0, 0.5, -0.2, 0, 6, "rgba(255,255,255,.53)", 1, false],
  [180, 0, 0, 0.62, -0.6, 0, 0, 0, 18, "#aebcff", null, true],
  [0, 0.72, 120, 0, 0.5, 0, 0, 0.4, 150, "rgba(255,255,255,.19)", null, true],
  [0, 0.12, 0, 0.18, 0, -0.3, 0.4, 0, 7, "#c4b6ff", null, false],
  [-64, 1, 0, 0.72, 0.2, 0, 0, -0.3, 11, "rgba(143,212,255,.67)", 2, false],
];

const PERIOD = 18000;
const still = matchMedia("(prefers-reduced-motion: reduce)");

// Lienzos animados: el fondo de la página y el de las capturas «vivas» de la
// app (las formas a la escala de la captura, diseñada a 1280 px de ancho).
const layers = [];

function addLayer(canvas, { design = 0, background = null } = {}) {
  layers.push({ canvas, ctx: canvas.getContext("2d"), design, background, width: 0, height: 0 });
}

function resize(layer) {
  const dpr = window.devicePixelRatio || 1;
  const box = layer.canvas.getBoundingClientRect();
  layer.width = box.width;
  layer.height = box.height;
  layer.canvas.width = Math.round(box.width * dpr);
  layer.canvas.height = Math.round(box.height * dpr);
  // Las capturas se dibujan en coordenadas de la app y se escalan.
  const k = layer.design ? box.width / layer.design : 1;
  layer.scale = k;
  layer.ctx.setTransform(dpr * k, 0, 0, dpr * k, 0, 0);
}

function draw(layer, now) {
  const { ctx, scale } = layer;
  const width = layer.width / scale;
  const height = layer.height / scale;
  const phase = ((now % PERIOD) / PERIOD) * 2;
  const swing = phase < 1 ? phase : 2 - phase;
  const sway = -28 + 62 * swing;
  const lift = 30 - 72 * swing;
  if (layer.background) {
    ctx.fillStyle = layer.background;
    ctx.fillRect(0, 0, width, height);
  } else {
    ctx.clearRect(0, 0, width, height);
  }
  ctx.lineWidth = 1 / Math.max(scale, 0.5);
  for (const [x, fx, y, fy, sx, lx, sy, ly, size, color, radius, outline] of SHAPES) {
    const left = x + fx * width + sx * sway + lx * lift;
    const top = y + fy * height + sy * sway + ly * lift;
    ctx.beginPath();
    if (radius == null) {
      const r = size / 2;
      ctx.arc(left + r, top + r, outline ? r - 0.5 : r, 0, Math.PI * 2);
    } else {
      ctx.roundRect(left, top, size, size, radius);
    }
    if (outline) {
      ctx.strokeStyle = color;
      ctx.stroke();
    } else {
      ctx.fillStyle = color;
      ctx.fill();
    }
  }
}

function drawAll(now) {
  for (const layer of layers) {
    if (layer.canvas.offsetParent !== null || layer.canvas.id === "atmosphere") draw(layer, now);
  }
}

function frame(now) {
  drawAll(now);
  if (!still.matches) requestAnimationFrame(frame);
}

function resizeAll() {
  for (const layer of layers) resize(layer);
  if (still.matches) drawAll(0);
}

addLayer(document.getElementById("atmosphere"));
for (const canvas of document.querySelectorAll("canvas.live-bg")) {
  addLayer(canvas, { design: 1280, background: "#212121" });
}
resizeAll();
addEventListener("resize", resizeAll);
// También cuando un lienzo cambia de tamaño o vuelve a mostrarse.
const observer = new ResizeObserver(resizeAll);
for (const layer of layers) observer.observe(layer.canvas);
still.addEventListener("change", () => (still.matches ? drawAll(0) : requestAnimationFrame(frame)));
requestAnimationFrame(frame);
