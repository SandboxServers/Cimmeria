import sys,collections;sys.path.insert(0,'.')
from cond2 import load
ticks=load('signoz-raw/ticks.json')
mv=load('signoz-raw/npc_movement.json')
from datetime import datetime
def t(ts):
    b=ts.rstrip("Z"); s,_,f=b.partition("."); return datetime.fromisoformat(s).timestamp()+float("0."+(f or "0"))
# last movement event per npc before time: was the path completed?
mvby=collections.defaultdict(list)
for o in mv: mvby[o[4]['npc_id']].append((t(o[0]),o))
by=collections.defaultdict(list)
for o in ticks: by[o[4]['npc_id']].append(o)
interrupts=collections.Counter(); runs=[]
detail=[]
for n,l in by.items():
    for i in range(1,len(l)):
        p=l[i-1][4]; c=l[i][4]
        if p.get('nav_path_len',0)>0 and c.get('nav_path_len',0)==0:
            # was the path completed by movement between the ticks?
            tp,tc=t(l[i-1][0]),t(l[i][0])
            done=any(o[3]=='NPC reached waypoint' and o[4].get('path_complete') in (True,'true','True') and tp<=tt<=tc+0.15 for tt,o in mvby[n])
            key=(c.get('ai_state'),c.get('decision_outcome'),'path_completed' if done else 'INTERRUPTED')
            interrupts[key]+=1
            if not done:
                # how long does the NPC then stay stationary (xz unchanged) ?
                j=i; 
                while j+1<len(l) and abs(l[j+1][4]['x']-c['x'])<0.05 and abs(l[j+1][4]['z']-c['z'])<0.05 and l[j+1][4].get('ai_state')!='Idle': j+=1
                dur=t(l[j][0])-tc
                acts=collections.Counter(l[k][4].get('decision_outcome') for k in range(i,j+1))
                detail.append((l[i][0][5:19],n,c.get('tag'),c.get('ai_state'),c.get('decision_outcome'),round(dur,1),dict(acts),round(c['x'],1),round(c['y'],1),round(c['z'],1)))
print('Path->no-path transitions between consecutive AI ticks:')
for k,v in interrupts.most_common(): print(' ',v,k)
print('\nINTERRUPTED paths (velocity not zeroed by these branches) and how long the NPC then stood at that xz while not Idle:')
for d in sorted(detail,key=lambda d:-d[5]): print(' ',d)
# frozen-but-attacking: runs of attack_in_place with xz unchanged
print('\nLongest attack_in_place runs with NPC xz unchanged:')
rr=[]
for n,l in by.items():
    run=[]
    for o in l:
        kv=o[4]
        if kv.get('decision_outcome') in ('attack_in_place','no_ability','stationary_holds') and (not run or (abs(run[-1][4]['x']-kv['x'])<0.05 and abs(run[-1][4]['z']-kv['z'])<0.05)):
            run.append(o)
        else:
            if len(run)>=4: rr.append(run)
            run=[o] if kv.get('decision_outcome') in ('attack_in_place','no_ability','stationary_holds') else []
    if len(run)>=4: rr.append(run)
rr.sort(key=lambda r:-len(r))
tot=collections.Counter()
for r in rr:
    a=r[0][4]; tot[a.get('tag')]+=len(r)
for r in rr[:25]:
    a=r[0][4]; ds=[o[4].get('dist_to_target') for o in r]
    print(f"  {r[0][0][5:19]}..{r[-1][0][11:19]} ticks={len(r)} npc={a['npc_id']} {a.get('tag')} pos=({a['x']:.1f},{a['y']:.1f},{a['z']:.1f}) spawn_d={a.get('dist_to_spawn')} d_tgt {ds[0]}..{ds[-1]} outcomes={dict(collections.Counter(o[4].get('decision_outcome') for o in r))}")
print('ticks in frozen-attack runs by tag:',tot)
