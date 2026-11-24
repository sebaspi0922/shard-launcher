#!/usr/bin/env python3
"""Compara las capturas y cajas de Slint con las referencias de Flutter.

  compare.py --ref tools/paridad/ref --slint target/paridad/slint \
             --out target/paridad/informe [--scenes a,b] [--ssim 0.97] [--px 2]

Por escena:
- SSIM global (RGB) de la captura Slint frente a la de Flutter (>= --ssim).
- Cajas de elementos clave: x, y, ancho y alto con tolerancia ±--px. Dos
  excepciones, porque su ancho sale del motor de texto y no del layout:
  `txt-*` (textos) compara x, y y alto; `*-play` (botón «Jugar», anclado a la
  derecha y tan ancho como su texto) compara borde derecho, y y alto.
- Imagen Flutter | Slint | diferencias (rojo = canal con ΔE > 10 en sRGB).
Escribe informe.md, informe.html e informe.json y sale con 1 si algo falla.
Necesita numpy, pillow y scikit-image.
"""
import argparse
import html
import json
import os
import sys

import numpy as np
from PIL import Image
from skimage.metrics import structural_similarity


def load_rgb(path):
    return np.asarray(Image.open(path).convert("RGB"), dtype=np.uint8)


def compare_boxes(ref, got, tol):
    """[(nombre, estado, ref, got, peor diferencia)]; estado ok|mal|falta."""
    rows = []
    for name, r in ref.items():
        g = got.get(name)
        if g is None:
            rows.append((name, "falta", r, None, None))
            continue
        if name.startswith("txt-"):
            worst = max(abs(r[i] - g[i]) for i in (0, 1, 3))
        elif name.endswith("-play"):
            worst = max(abs((r[0] + r[2]) - (g[0] + g[2])), abs(r[1] - g[1]), abs(r[3] - g[3]))
        else:
            worst = max(abs(r[i] - g[i]) for i in range(4))
        rows.append((name, "ok" if worst <= tol else "mal", r, g, worst))
    for name, g in got.items():
        if name not in ref:
            rows.append((name, "sobra", None, g, None))
    return rows


def diff_image(a, b):
    d = np.abs(a.astype(np.int16) - b.astype(np.int16)).max(axis=2)
    mask = d > 10
    base = (b.astype(np.float32) * 0.35).astype(np.uint8)
    base[mask] = [255, 40, 40]
    return base, float(mask.mean())


def side_by_side(a, b, d):
    h = max(x.shape[0] for x in (a, b, d))
    gap = np.full((h, 8, 3), 255, np.uint8)
    return np.concatenate([a, gap, b, gap, d], axis=1)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--ref", required=True)
    ap.add_argument("--slint", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--scenes", default="all")
    ap.add_argument("--ssim", type=float, default=0.97)
    ap.add_argument("--px", type=float, default=2.0)
    a = ap.parse_args()
    os.makedirs(a.out, exist_ok=True)

    scenes = sorted(f[:-5] for f in os.listdir(a.ref) if f.endswith(".json"))
    if a.scenes != "all":
        wanted = set(a.scenes.split(","))
        scenes = [s for s in scenes if s in wanted]
    if not scenes:
        print("ninguna escena", file=sys.stderr)
        return 2

    results = []
    for s in scenes:
        ref_png, got_png = os.path.join(a.ref, s + ".png"), os.path.join(a.slint, s + ".png")
        if not os.path.exists(got_png):
            results.append({"scene": s, "ok": False, "error": "falta la captura de Slint"})
            continue
        ra, ga = load_rgb(ref_png), load_rgb(got_png)
        if ra.shape != ga.shape:
            results.append({"scene": s, "ok": False, "error": f"tamaños distintos {ra.shape} vs {ga.shape}"})
            continue
        ssim = float(structural_similarity(ra, ga, channel_axis=2, data_range=255))
        dimg, frac = diff_image(ra, ga)
        Image.fromarray(side_by_side(ra, ga, dimg)).save(os.path.join(a.out, s + "-lado-a-lado.png"))
        ref_boxes = json.load(open(os.path.join(a.ref, s + ".json")))["boxes"]
        got_boxes = json.load(open(os.path.join(a.slint, s + ".json")))["boxes"]
        rows = compare_boxes(ref_boxes, got_boxes, a.px)
        bad = [r for r in rows if r[1] in ("mal", "falta")]
        results.append({
            "scene": s,
            "ssim": round(ssim, 4),
            "diff_px": round(frac * 100, 2),
            "boxes_ok": sum(r[1] == "ok" for r in rows),
            "boxes_total": sum(r[1] != "sobra" for r in rows),
            "boxes": [{"name": n, "state": st, "ref": r, "slint": g, "worst": w} for n, st, r, g, w in rows],
            "ok": ssim >= a.ssim and not bad,
        })

    json.dump(results, open(os.path.join(a.out, "informe.json"), "w"), indent=2)
    write_md(a, results)
    write_html(a, results)
    for r in results:
        if "error" in r:
            print(f"{r['scene']}: ERROR {r['error']}")
        else:
            print(f"{r['scene']}: SSIM {r['ssim']:.4f}  cajas {r['boxes_ok']}/{r['boxes_total']}  "
                  f"ΔE>10 {r['diff_px']}%  {'OK' if r['ok'] else 'FALLA'}")
    return 0 if all(r["ok"] for r in results) else 1


def fmt_box(b):
    return "—" if b is None else "(" + ", ".join(f"{v:.1f}" for v in b) + ")"


def write_md(a, results):
    lines = [f"# Paridad Flutter → Slint", "",
             f"Umbral: SSIM ≥ {a.ssim}, cajas ±{a.px} px (textos: x, y, alto; botones Jugar: borde derecho, y, alto).", "",
             "| escena | SSIM | cajas | píxeles ΔE>10 | estado |", "|---|---|---|---|---|"]
    for r in results:
        if "error" in r:
            lines.append(f"| {r['scene']} | — | — | — | ERROR: {r['error']} |")
        else:
            lines.append(f"| {r['scene']} | {r['ssim']:.4f} | {r['boxes_ok']}/{r['boxes_total']} | "
                         f"{r['diff_px']} % | {'ok' if r['ok'] else '**falla**'} |")
    for r in results:
        bad = [b for b in r.get("boxes", []) if b["state"] != "ok"]
        if bad:
            lines += ["", f"## {r['scene']}: cajas fuera de tolerancia", "",
                      "| caja | estado | Flutter | Slint | Δ máx |", "|---|---|---|---|---|"]
            for b in bad:
                w = "—" if b["worst"] is None else f"{b['worst']:.1f}"
                lines.append(f"| {b['name']} | {b['state']} | {fmt_box(b['ref'])} | {fmt_box(b['slint'])} | {w} |")
    open(os.path.join(a.out, "informe.md"), "w").write("\n".join(lines) + "\n")


def write_html(a, results):
    e = html.escape
    parts = ["<!doctype html><meta charset=utf-8><title>Paridad Slint</title>",
             "<style>body{font:14px system-ui;margin:24px;background:#fafafa;color:#222}"
             "table{border-collapse:collapse;margin:8px 0}td,th{border:1px solid #ccc;padding:4px 8px;text-align:left}"
             ".ok{color:#0a7a3a}.mal{color:#b00020;font-weight:600}img{max-width:100%;border:1px solid #ccc}"
             "h2{margin-top:32px}</style>",
             f"<h1>Paridad Flutter → Slint</h1><p>Umbral: SSIM ≥ {a.ssim}, cajas ±{a.px} px. "
             "Imagen: Flutter | Slint | diferencias (rojo = ΔE &gt; 10).</p><table>"
             "<tr><th>escena<th>SSIM<th>cajas<th>ΔE&gt;10<th>estado"]
    for r in results:
        if "error" in r:
            parts.append(f"<tr><td>{e(r['scene'])}<td colspan=4 class=mal>{e(r['error'])}")
        else:
            cls = "ok" if r["ok"] else "mal"
            parts.append(f"<tr><td><a href='#{e(r['scene'])}'>{e(r['scene'])}</a><td>{r['ssim']:.4f}"
                         f"<td>{r['boxes_ok']}/{r['boxes_total']}<td>{r['diff_px']} %"
                         f"<td class={cls}>{'ok' if r['ok'] else 'falla'}")
    parts.append("</table>")
    for r in results:
        if "error" in r:
            continue
        parts.append(f"<h2 id='{e(r['scene'])}'>{e(r['scene'])}</h2>"
                     f"<img src='{e(r['scene'])}-lado-a-lado.png' alt=''>"
                     "<table><tr><th>caja<th>estado<th>Flutter<th>Slint<th>Δ máx")
        for b in r["boxes"]:
            cls = "ok" if b["state"] == "ok" else "mal"
            w = "—" if b["worst"] is None else f"{b['worst']:.1f}"
            parts.append(f"<tr><td>{e(b['name'])}<td class={cls}>{b['state']}<td>{fmt_box(b['ref'])}"
                         f"<td>{fmt_box(b['slint'])}<td>{w}")
        parts.append("</table>")
    open(os.path.join(a.out, "informe.html"), "w").write("\n".join(parts) + "\n")


if __name__ == "__main__":
    sys.exit(main())
