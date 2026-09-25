import sys,collections
sys.path.insert(0,'.')
from cond2 import load
from datetime import datetime
ev=load('signoz-raw/ai_main.json')
def t(ts):
    b=ts.rstrip("Z"); s,_,f=b.partition("."); return datetime.fromisoformat(s).timestamp()+float("0."+(f or "0"))
last_auto={}
sess=None
stats=collections.Counter(); per=collections.defaultdict(list)
for ts,sev,sc,b,kv in ev:
    if b=='player entered world':
        sess=f"{ts[5:19]} {kv['character_name']}@{kv['world']} pl_ent={kv['entity_id']}"; continue
    if 'auto-aggro' in b:
        last_auto[kv['npc_id']]=t(ts); per[sess].append(f"{ts[11:19]} AUTO npc={kv['npc_id']} -> pl {kv['player_id']}"); continue
    if 'preempt' in b:
        n=kv['npc_id']; la=last_auto.get(n)
        kind='proximity' if la and (t(ts)-la)<2 else 'damage/chain'
        stats[(kind,kv.get('prev'))]+=1
        per[sess].append(f"{ts[11:19]} FIGHT npc={n} by={kind} attacker={kv['attacker']} prev={kv.get('prev')}")
    elif 'leashing' in b:
        per[sess].append(f"{ts[11:19]} LEASH npc={kv['npc_id']} dist_to_spawn={kv['dist_to_spawn']:.1f} tgt={kv['target_id']}")
    elif 'Target killed' in b:
        per[sess].append(f"{ts[11:19]} KILL target={kv['target']} by={kv['attacker']} is_npc={kv['is_npc']}")
    elif 'stationary' in b:
        per[sess].append(f"{ts[11:19]} HOLD(stationary) npc={kv['npc_id']} d={kv['dist_to_target']:.1f} los={kv['has_los']} in_range={kv['in_range']}")
    elif 'path_fail' in sc:
        per[sess].append(f"{ts[11:19]} PATHFAIL npc={kv['npc_id']} reason={kv.get('reason')} dy={kv.get('dy')} npc_y={kv.get('npc_y')} dest_y={kv.get('dest_y')}")
print(stats)
for s,l in per.items():
    print('\n##',s); 
    # compress consecutive dup lines
    prev=None;c=0
    for x in l:
        key=x[9:]
        if key==prev: c+=1; continue
        if c: print(f"      (x{c+1})")
        print('  ',x); prev=key; c=0
    if c: print(f"      (x{c+1})")
