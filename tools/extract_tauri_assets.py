import sys, struct, os, brotli
from elftools.elf.elffile import ELFFile
path, out = sys.argv[1], sys.argv[2]
f = open(path,'rb'); data = f.read(); elf = ELFFile(open(path,'rb'))
segs = [(s['p_vaddr'], s['p_offset'], s['p_filesz']) for s in elf.iter_segments() if s['p_type']=='PT_LOAD']
def v2o(v):
    for va,off,sz in segs:
        if va <= v < va+sz: return off+(v-va)
    return None
# relocations: offset(vaddr)->addend
rel = {}
for sec in elf.iter_sections():
    if sec.header['sh_type'] == 'SHT_RELA':
        for r in sec.iter_relocations():
            rel[r['r_offset']] = r['r_addend']
print('relocs', len(rel))
ro = elf.get_section_by_name('.data.rel.ro'); base = ro['sh_addr']; n = ro['sh_size']
def ptr(va):
    if va in rel: return rel[va]
    o = v2o(va); return struct.unpack_from('<Q', data, o)[0]
count=0
for va in range(base, base+n-32, 8):
    if va not in rel: continue
    kp = rel[va]; o = v2o(kp)
    if o is None: continue
    kl = struct.unpack_from('<Q', data, v2o(va+8))[0]
    if not (2 <= kl < 200): continue
    key = data[o:o+kl]
    if not key.startswith(b'/') : continue
    try: key = key.decode()
    except: continue
    if va+16 not in rel: continue
    dp = rel[va+16]; dl = struct.unpack_from('<Q', data, v2o(va+24))[0]
    do = v2o(dp)
    if do is None or dl > 50_000_000: continue
    blob = data[do:do+dl]
    try: content = brotli.decompress(blob)
    except Exception: content = blob
    dst = os.path.join(out, key.lstrip('/'))
    os.makedirs(os.path.dirname(dst) or out, exist_ok=True)
    open(dst,'wb').write(content); count+=1
    print(key, dl, len(content))
print('extracted', count)
