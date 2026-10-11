# Prints JSON structure (keys and value types only), never values.
import json,sys
def walk(v,p,depth=0):
    if isinstance(v,dict):
        for k,x in v.items():
            print('  '*depth+f'{k}: {type(x).__name__}'+(f'[{len(x)}]' if isinstance(x,(list,dict)) else ''))
            walk(x,p+[k],depth+1)
    elif isinstance(v,list) and v:
        print('  '*depth+'[0]:'); walk(v[0],p,depth+1)
for f in sys.argv[1:]:
    print('##',f)
    try: walk(json.load(open(f)),[])
    except Exception as e: print('ERR',type(e).__name__)
