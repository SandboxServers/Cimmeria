import sys,collections; sys.path.insert(0,'.')
from cond2 import load
from names import name
from datetime import datetime
def t(ts):
    b=ts.rstrip("Z"); s,_,f=b.partition("."); return datetime.fromisoformat(s).timestamp()+float("0."+(f or "0"))
ev=load('signoz-raw/ai_main.json')
last_auto={}; first={}; autos=collections.Counter(); leash=collections.Counter(); kills=collections.Counter(); holds=collections.Counter()
byname=collections.defaultdict(collections.Counter)
for ts,sev,sc,b,kv in ev:
    n=kv.get('npc_id')
    if 'auto-aggro' in b: last_auto[n]=t(ts); autos[(n,name(n,ts))]+=1
    elif 'preempt' in b:
        la=last_auto.get(n); kind='proximity' if la and t(ts)-la<2 else 'damage/chain'
        key=(n,name(n,ts))
        if key not in first: first[key]=(ts,kind); byname[name(n,ts).split('@')[0]][kind]+=1
    elif 'leashing' in b: leash[(n,name(n,ts))]+=1
    elif 'stationary' in b: holds[(n,name(n,ts))]+=1
print("FIRST ENGAGEMENT BY NPC NAME/TAG (per npc instance):")
for k,v in sorted(byname.items(), key=lambda x:-sum(x[1].values())): print(f"  {k}: {dict(v)}")
print("\nAUTO-AGGRO counts by npc:"); 
for k,v in autos.most_common(40): print(' ',v,k)
print("\nLEASH counts by npc:"); 
for k,v in leash.most_common(30): print(' ',v,k)
print("\nSTATIONARY HOLD counts by npc:")
for k,v in holds.most_common(30): print(' ',v,k)
