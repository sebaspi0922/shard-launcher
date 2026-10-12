#!/usr/bin/env python3
"""Genera las 8 propuestas de logo de Shard (SVG 256×256) y una hoja comparativa.

Uso: python3 docs/logo/generar.py   (necesita rsvg-convert y magick)
"""

import math
import subprocess
from pathlib import Path

HERE = Path(__file__).resolve().parent
OUT = HERE / "opciones"

# Violetas propios (inspirados en un cristal, no copiados de ningún juego).
L1 = "#EDE6FF"
L2 = "#C9B8FF"
M = "#9A7BF4"
D = "#6B4AD8"
DD = "#432A9E"
INK = "#24164F"


def svg(body: str, defs: str = "") -> str:
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" width="256" height="256">'
        f"<defs>{defs}</defs>{body}</svg>\n"
    )


def poly(points, fill, extra=""):
    pts = " ".join(f"{x:.1f},{y:.1f}" for x, y in points)
    return f'<polygon points="{pts}" fill="{fill}" {extra}/>'


def shard(x, y, w, h, light, dark, rot=0, tip=0.85):
    """Prisma de cristal visto de frente: mitad clara y mitad oscura, punta arriba."""
    top = y - h
    shoulder = y - h + w * tip
    left = [(x - w / 2, y), (x - w / 2, shoulder), (x, top), (x, y)]
    right = [(x, y), (x, top), (x + w / 2, shoulder), (x + w / 2, y)]
    g = f'<g transform="rotate({rot} {x} {y})">'
    return g + poly(left, light) + poly(right, dark) + "</g>"


# 1. Faceta: un fragmento único, inclinado, con cuatro caras.
def faceta():
    tip, ls, lb, bot, rs, rb, ridge = (
        (128, 22), (82, 92), (90, 196), (122, 236), (178, 100), (164, 206), (126, 132)
    )
    body = (
        '<g transform="rotate(-14 128 128)">'
        + poly([tip, ls, ridge], L2)
        + poly([ls, lb, bot, ridge], M)
        + poly([tip, rs, ridge], D)
        + poly([rs, rb, bot, ridge], DD)
        + poly([(126, 40), (100, 90), (112, 92)], L1, 'opacity="0.9"')
        + "</g>"
    )
    return svg(body)


# 2. Drusa: tres cristales que brotan de una base.
def drusa():
    body = (
        shard(84, 206, 44, 110, L2, D, rot=-24)
        + shard(176, 206, 44, 124, L2, D, rot=20)
        + shard(128, 212, 56, 186, L1, M)
        + poly([(52, 204), (204, 204), (190, 232), (66, 232)], INK)
        + poly([(52, 204), (204, 204), (196, 214), (60, 214)], DD)
    )
    return svg(body)


# 3. Píxel: fragmento en diagonal dibujado en una cuadrícula de 16×16.
def pixel():
    n, cell = 16, 14
    off = (256 - n * cell) / 2
    a, b = (1.6, 14.4), (14.4, 1.6)
    ax, ay = b[0] - a[0], b[1] - a[1]
    length = math.hypot(ax, ay)
    ux, uy = ax / length, ay / length
    inside = {}
    for gy in range(n):
        for gx in range(n):
            px, py = gx + 0.5 - a[0], gy + 0.5 - a[1]
            t = (px * ux + py * uy) / length
            d = px * -uy + py * ux
            if 0 <= t <= 1:
                half = 4.2 * min(1, (t + 0.12) * 3.2) * (1 - t) ** 0.9
                if abs(d) < half:
                    inside[(gx, gy)] = (t, d)
    # Fuera los píxeles sueltos de las puntas.
    for _ in range(2):
        inside = {
            k: v for k, v in inside.items()
            if sum((k[0] + dx, k[1] + dy) in inside for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1))) >= 2
        }
    rects = []
    for (gx, gy), (t, d) in inside.items():
        edge = any((gx + dx, gy + dy) not in inside for dx, dy in ((1, 0), (-1, 0), (0, 1), (0, -1)))
        if edge:
            color = INK
        elif d < -1.2:
            color = L2
        elif d < 0.6:
            color = M
        else:
            color = D
        if not edge and 0.5 < t < 0.72 and -2.2 < d < -1.0:
            color = L1
        rects.append(
            f'<rect x="{off + gx * cell}" y="{off + gy * cell}" width="{cell}" height="{cell}" fill="{color}"/>'
        )
    return svg("".join(rects))


# 4. Monograma: una S con dos fragmentos inclinados y desplazados.
def monograma():
    top = [(206, 30), (98, 30), (50, 122), (158, 122)]
    bottom = [(50, 226), (158, 226), (206, 134), (98, 134)]
    body = (
        poly(top, L2)
        + poly([(98, 30), (50, 122), (158, 122)], M)
        + poly(bottom, D)
        + poly([(158, 226), (206, 134), (98, 134)], DD)
    )
    return svg(body)


# 5. Bloque astillado: un cubo isométrico al que se le salta una esquina.
def bloque():
    c = (128, 132)
    top_v, ur, lr, bot, ll, ul = (128, 36), (211, 84), (211, 180), (128, 228), (45, 180), (45, 84)
    p_top, p_right, p_in = (176, 64), (211, 128), (184, 104)
    body = (
        poly([top_v, p_top, p_in, c, ul], L2)
        + poly([ul, c, bot, ll], M)
        + poly([c, p_in, p_right, lr, bot], D)
        # Huecos de la esquina rota (cara interior).
        + poly([p_top, ur, p_in], INK)
        + poly([ur, p_right, p_in], INK, 'opacity="0.85"')
        # La esquina que sale despedida.
        + '<g transform="translate(20 -22) rotate(10 200 90)">'
        + poly([p_top, ur, p_in], L1)
        + poly([ur, p_right, p_in], M)
        + "</g>"
    )
    return svg(body)


# 6. Trazo: solo líneas, con las aristas interiores del cristal.
def trazo():
    s = f'fill="none" stroke="{L2}" stroke-width="11" stroke-linejoin="round" stroke-linecap="round"'
    outline = [(128, 24), (186, 92), (172, 210), (128, 236), (84, 210), (70, 92)]
    pts = " ".join(f"{x},{y}" for x, y in outline)
    body = (
        f'<polygon points="{pts}" {s}/>'
        f'<polyline points="70,92 128,120 186,92" {s}/>'
        f'<polyline points="128,24 128,120 128,236" {s} stroke-opacity="0.55"/>'
    )
    return svg(body)


# 7. Ficha: icono de app con el fragmento recortado en negativo.
def ficha():
    defs = (
        '<linearGradient id="g" x1="0" y1="0" x2="1" y2="1">'
        f'<stop offset="0" stop-color="{M}"/><stop offset="1" stop-color="{DD}"/>'
        "</linearGradient>"
    )
    tip, ls, lb, bot, rs, rb, ridge = (
        (134, 46), (96, 104), (102, 186), (128, 214), (168, 110), (158, 192), (130, 140)
    )
    body = (
        '<rect x="16" y="16" width="224" height="224" rx="56" fill="url(#g)"/>'
        + poly([tip, ls, ridge], "#FFFFFF")
        + poly([ls, lb, bot, ridge], "#FFFFFF", 'opacity="0.78"')
        + poly([tip, rs, ridge], "#FFFFFF", 'opacity="0.62"')
        + poly([rs, rb, bot, ridge], "#FFFFFF", 'opacity="0.42"')
    )
    return svg(body, defs)


# 8. Órbita: un fragmento pequeño con un anillo fino alrededor.
def orbita():
    tip, ls, bot, rs, ridge = (128, 52), (100, 116), (128, 204), (156, 116), (128, 132)
    ring = (
        f'<ellipse cx="128" cy="128" rx="104" ry="38" fill="none" stroke="{L2}" '
        'stroke-width="7" transform="rotate(-24 128 128)"'
    )
    body = (
        # Mitad trasera del anillo, detrás del cristal.
        ring + ' stroke-opacity="0.45"/>'
        + poly([tip, ls, ridge], L1)
        + poly([ls, bot, ridge], M)
        + poly([tip, rs, ridge], D)
        + poly([rs, bot, ridge], DD)
        # Mitad delantera del anillo, por encima.
        + '<clipPath id="front"><rect x="0" y="128" width="256" height="128" transform="rotate(-24 128 128)"/></clipPath>'
        + f'<g clip-path="url(#front)">{ring}/></g>'
        + f'<circle cx="218" cy="70" r="9" fill="{L1}"/>'
    )
    return svg(body)


OPTIONS = [
    ("1-faceta", "1 · Faceta", faceta),
    ("2-drusa", "2 · Drusa", drusa),
    ("3-pixel", "3 · Píxel", pixel),
    ("4-monograma", "4 · Monograma S", monograma),
    ("5-bloque", "5 · Bloque astillado", bloque),
    ("6-trazo", "6 · Trazo", trazo),
    ("7-ficha", "7 · Ficha", ficha),
    ("8-orbita", "8 · Órbita", orbita),
]


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    tiles = []
    for slug, label, make in OPTIONS:
        path = OUT / f"{slug}.svg"
        path.write_text(make())
        big = OUT / f"{slug}-256.png"
        small = OUT / f"{slug}-32.png"
        subprocess.run(["rsvg-convert", "-w", "256", "-h", "256", str(path), "-o", str(big)], check=True)
        subprocess.run(["rsvg-convert", "-w", "32", "-h", "32", str(path), "-o", str(small)], check=True)
        tile = OUT / f"{slug}-tile.png"
        # Ficha: logo grande, y a 32 px sobre fondo oscuro y claro.
        subprocess.run(
            [
                "magick", "-size", "340x400", "xc:#161616",
                str(big), "-geometry", "+42+24", "-composite",
                "(", "-size", "56x56", "xc:#212121", str(small), "-gravity", "center", "-composite", ")",
                "-gravity", "northwest", "-geometry", "+110+296", "-composite",
                "(", "-size", "56x56", "xc:#ECECEC", str(small), "-gravity", "center", "-composite", ")",
                "-gravity", "northwest", "-geometry", "+174+296", "-composite",
                "-font", str(HERE.parent.parent / "app/assets/fonts/Inter-SemiBold.ttf"), "-pointsize", "22", "-fill", "#ECECEC",
                "-gravity", "south", "-annotate", "+0+18", label,
                str(tile),
            ],
            check=True,
        )
        tiles.append(str(tile))
    subprocess.run(
        ["magick", "montage", *tiles, "-tile", "4x2", "-geometry", "+6+6", "-background", "#0E0E0E",
         str(HERE / "opciones.png")],
        check=True,
    )
    for t in tiles:
        Path(t).unlink()
    print(HERE / "opciones.png")


if __name__ == "__main__":
    main()
