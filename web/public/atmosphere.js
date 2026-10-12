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
const canvas = document.getElementById("atmosphere");
const ctx = canvas.getContext("2d");
const still = matchMedia("(prefers-reduced-motion: reduce)");
let width = 0;
let height = 0;

function resize() {
  const dpr = window.devicePixelRatio || 1;
  width = innerWidth;
  height = innerHeight;
  canvas.width = Math.round(width * dpr);
  canvas.height = Math.round(height * dpr);
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
}

function draw(now) {
  const phase = ((now % PERIOD) / PERIOD) * 2;
  const swing = phase < 1 ? phase : 2 - phase;
  const sway = -28 + 62 * swing;
  const lift = 30 - 72 * swing;
  ctx.clearRect(0, 0, width, height);
  ctx.lineWidth = 1;
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

function frame(now) {
  draw(now);
  if (!still.matches) requestAnimationFrame(frame);
}

resize();
addEventListener("resize", () => {
  resize();
  if (still.matches) draw(0);
});
still.addEventListener("change", () => (still.matches ? draw(0) : requestAnimationFrame(frame)));
requestAnimationFrame(frame);
