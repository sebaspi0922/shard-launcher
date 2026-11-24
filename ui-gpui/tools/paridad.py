"""Paridad visual entre capturas de 1280×800 (criterio de PLAN.md §5).

    python paridad.py SALIDA_DIR nombre=a.png:b.png [nombre=a.png:b.png ...]

Por par: SSIM (gris, ventana 7), % de píxeles con ΔE76 > 10 (Lab), un mapa
de diferencias (rojo = ΔE > 10) y un montaje «a | b | diferencias». Escribe
`paridad.md` con la tabla. Requiere numpy, pillow y scikit-image.
"""

import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from skimage.color import rgb2lab
from skimage.metrics import structural_similarity


def load(path: str) -> np.ndarray:
    return np.asarray(Image.open(path).convert("RGB"), dtype=np.uint8)


def compare(a: np.ndarray, b: np.ndarray):
    ssim = structural_similarity(a, b, channel_axis=2, data_range=255)
    de = np.linalg.norm(rgb2lab(a) - rgb2lab(b), axis=2)
    return ssim, float((de > 10).mean() * 100), de


def heat(a: np.ndarray, de: np.ndarray) -> Image.Image:
    base = (a.astype(np.float32) * 0.35).astype(np.uint8)
    mask = de > 10
    base[mask] = [255, 60, 60]
    return Image.fromarray(base)


def montage(a, b, d, labels):
    w, h = a.width, a.height
    out = Image.new("RGB", (w * 3 + 40, h + 36), (18, 18, 18))
    draw = ImageDraw.Draw(out)
    for i, (img, label) in enumerate(zip((a, b, d), labels)):
        x = i * (w + 20)
        out.paste(img, (x, 36))
        draw.text((x + 8, 10), label, fill=(230, 230, 230))
    return out


def main() -> None:
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    lines = ["| par | SSIM | píxeles ΔE>10 |", "|---|---|---|"]
    for spec in sys.argv[2:]:
        name, pair = spec.split("=", 1)
        pa, pb = pair.split(":")
        a, b = load(pa), load(pb)
        if a.shape != b.shape:
            raise SystemExit(f"{name}: tamaños distintos {a.shape} vs {b.shape}")
        ssim, pct, de = compare(a, b)
        d = heat(a, de)
        d.save(out / f"diff-{name}.png")
        montage(Image.fromarray(a), Image.fromarray(b), d,
                (Path(pa).stem, Path(pb).stem, f"ΔE>10: {pct:.2f} %  SSIM {ssim:.4f}")) \
            .save(out / f"lado-a-lado-{name}.png")
        lines.append(f"| {name} | {ssim:.4f} | {pct:.2f} % |")
        print(f"{name}: SSIM={ssim:.4f} dE>10={pct:.2f}%")
    (out / "paridad.md").write_text("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
