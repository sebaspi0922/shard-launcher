import sys
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer
from fontTools import subset
src, out_dir = sys.argv[1], sys.argv[2]
# 'm': GPUI (cosmic_text_system.rs) ignora como fuente principal una cara sin 'm'.
cps = [0x6d, 0xf0d3,0xe9b2,0xf720,0xe87b,0xe875,0xf19e,0xe8b8,0xef7a,0xe9b0,0xe8e9,0xe92e,0xe145,0xefd6,0xe15b,0xe3c6,0xe5cd]
for name, fill, wght in [("ShardIcons", 0, 400), ("ShardIconsFill", 1, 500)]:
    f = TTFont(src)
    f = instancer.instantiateVariableFont(f, {"FILL": fill, "wght": wght, "GRAD": 0, "opsz": 24})
    opts = subset.Options(); opts.layout_features = []; opts.name_IDs = ["*"]
    s = subset.Subsetter(opts); s.populate(unicodes=cps); s.subset(f)
    for rec in f["name"].names:
        if rec.nameID in (1, 3, 4, 6, 16, 21):
            rec.string = name
    f.save(f"{out_dir}/{name}.ttf")
    print(name, "ok")
