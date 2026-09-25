import sys,bisect,collections; sys.path.insert(0,'.')
from cond2 import load
sp=load('signoz-raw/spawns.json')
idx=collections.defaultdict(list)
for ts,_,_,b,kv in sp:
    idx[int(kv['npc_id'])].append((ts,kv.get('name'),kv.get('tag','').replace('Some("','').replace('")',''),kv.get('space_id'),kv.get('world')))
def name(npc,ts):
    l=idx.get(int(npc),[]); r=None
    for e in l:
        if e[0]<=ts: r=e
    return f"{r[1]}[{r[2]}]@{r[3]}" if r else '?'
