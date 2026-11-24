#!/usr/bin/env python3
"""Genera ui/fonts/ShardIcons.ttf (FILL 0, wght 400) y ShardIconsFill.ttf
(FILL 1, wght 500) con solo los iconos de ui/slint/icons.slint.

  tools/slint/subset_icons.py [MaterialSymbolsRounded.ttf]

Por defecto usa la fuente del paquete material_symbols_icons de pub (la misma
que pinta Flutter). Necesita fonttools.
"""
import glob, os, re, sys
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer
from fontTools import subset

root = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
src = sys.argv[1] if len(sys.argv) > 1 else sorted(glob.glob(os.path.expanduser(
    "~/.pub-cache/hosted/pub.dev/material_symbols_icons-*/lib/fonts/MaterialSymbolsRounded.ttf")))[-1]
icons = open(os.path.join(root, "ui/slint/icons.slint"), encoding="utf-8").read()
cps = sorted({int(h, 16) for h in re.findall(r'\\u\{([0-9a-fA-F]+)\}', icons)})
for name, fill, wght in [("ShardIcons", 0, 400), ("ShardIconsFill", 1, 500)]:
    f = instancer.instantiateVariableFont(TTFont(src), {"FILL": fill, "wght": wght, "GRAD": 0, "opsz": 24})
    opts = subset.Options(); opts.layout_features = []; opts.name_IDs = ["*"]
    s = subset.Subsetter(opts); s.populate(unicodes=cps); s.subset(f)
    for rec in f["name"].names:
        if rec.nameID in (1, 3, 4, 6, 16, 21):
            rec.string = name
    out = os.path.join(root, "ui/fonts", name + ".ttf")
    f.save(out)
    print(f"{out}: {len(cps)} iconos")
