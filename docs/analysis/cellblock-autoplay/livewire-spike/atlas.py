import sys,os,json
sys.path.insert(0,os.path.dirname(os.path.abspath(__file__)))
from shapes import *
STEP=5
atlas={}
for name,cid in sorted(exports.items()):
    if not (name[0] in 'gomp') or name.startswith('hack'): continue
    E=flat(cid)
    if not E: continue
    b=bbox(cid)
    x0,x1,y0,y1=[int(v/20) for v in b]
    pts=[]
    for lx in range(x0 - x0%STEP, x1+1, STEP):
        for ly in range(y0 - y0%STEP, y1+1, STEP):
            if hit(E,lx*20,ly*20): pts.append((lx,ly))
    atlas[name]=pts
    print(name,len(pts),file=sys.stderr)
json.dump(atlas,open('livewire_atlas.json','w'))
