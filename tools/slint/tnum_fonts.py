#!/usr/bin/env python3
"""Genera ui/fonts/InterTnum-{Medium,Bold}.ttf: Inter con las cifras
tabulares (rasgo OpenType `tnum`) como cifras por defecto. Slint no activa
rasgos OpenType y Flutter usa `FontFeature.tabularFigures()` en el botón
«Jugar» y en la píldora de actividad (`Descargando 1834/3920`).

  tools/slint/tnum_fonts.py

Solo latín básico y Latin-1 (lo que llevan esos textos). Necesita fonttools.
"""
import os
from fontTools.ttLib import TTFont
from fontTools import subset

root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
for weight in ["Medium", "Bold"]:
    f = TTFont(os.path.join(root, f"ui/fonts/Inter-{weight}.ttf"))
    gsub = f["GSUB"].table
    tnum = {}
    for fr in gsub.FeatureList.FeatureRecord:
        if fr.FeatureTag == "tnum":
            for li in fr.Feature.LookupListIndex:
                for st in gsub.LookupList.Lookup[li].SubTable:
                    st = getattr(st, "ExtSubTable", st)
                    tnum.update(getattr(st, "mapping", {}) or {})
    for table in f["cmap"].tables:
        for cp, g in list(table.cmap.items()):
            if g in tnum:
                table.cmap[cp] = tnum[g]
    opts = subset.Options(); opts.layout_features = ["kern"]; opts.name_IDs = ["*"]
    s = subset.Subsetter(opts); s.populate(unicodes=list(range(0x20, 0x7f)) + list(range(0xa0, 0x100))); s.subset(f)
    for rec in f["name"].names:
        if rec.nameID in (1, 16):
            rec.string = "Inter Tnum"
        elif rec.nameID in (4, 6):
            rec.string = f"InterTnum-{weight}" if rec.nameID == 6 else f"Inter Tnum {weight}"
    out = os.path.join(root, f"ui/fonts/InterTnum-{weight}.ttf")
    f.save(out)
    print(out, len(tnum), "sustituciones")
