import sys,re
data=open(sys.argv[1],'rb').read()
pat=re.compile(sys.argv[2].encode()); w=int(sys.argv[3]) if len(sys.argv)>3 else 80
seen=set()
for m in pat.finditer(data):
    s=data[max(0,m.start()-w):m.end()+w]
    s=re.sub(rb'[^\x20-\x7e]',b'\xc2\xb7',s).decode('utf8','replace')
    if s in seen: continue
    seen.add(s); print(hex(m.start()), s)
