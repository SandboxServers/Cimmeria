import sys,collections,math;sys.path.insert(0,'.')
from cond2 import load
ev=load('signoz-raw/ticks.json')
print(len(ev),ev[0][0],ev[-1][0])
by=collections.defaultdict(list)
for o in ev: by[o[4]['npc_id']].append(o)
st=collections.Counter((o[4].get('ai_state'),o[4].get('decision_outcome')) for o in ev); print(st.most_common())
print(collections.Counter(o[4].get('npc_name') for o in ev).most_common())
# stuck: consecutive Fighting ticks, same xyz, decision chase or ''/hold, target moving or dist > 30
runs=[]
for n,l in by.items():
    run=[]
    for o in l:
        kv=o[4]
        if kv.get('ai_state')=='Fighting' and kv.get('decision_outcome') not in ('attack_in_place',):
            if run and (abs(run[-1][4]['x']-kv['x'])<0.05 and abs(run[-1][4]['z']-kv['z'])<0.05):
                run.append(o); continue
            if len(run)>=3: runs.append(run)
            run=[o]
        else:
            if len(run)>=3: runs.append(run)
            run=[]
    if len(run)>=3: runs.append(run)
print('\nSTUCK RUNS (>=3 consecutive non-attack Fighting ticks, NPC xz unchanged):',len(runs))
for r in runs:
    a=r[0][4]; b=r[-1][4]
    print(f" {r[0][0][5:19]}..{r[-1][0][11:19]} n={len(r)} npc={a['npc_id']} {a.get('npc_name')}[{a.get('tag')}] pos=({a['x']:.1f},{a['y']:.1f},{a['z']:.1f}) spawn_d={a.get('dist_to_spawn')} outcomes={collections.Counter(o[4].get('decision_outcome') for o in r)} d_tgt={a.get('dist_to_target')}->{b.get('dist_to_target')} los={collections.Counter(str(o[4].get('has_los')) for o in r)} nav_len={collections.Counter(o[4].get('nav_path_len') for o in r)}")
# y vs target y
print('\nFIGHT TICKS with NPC y above target y by >2 or NPC y far from both spawn & target:')
for o in ev:
    kv=o[4]; tp=kv.get('target_pos')
    if kv.get('ai_state')!='Fighting' or not tp or tp=='None': continue
    ty=float(tp.replace('Some([','').rstrip('])').split(',')[1])
    if kv['y']-ty>2: print(f" {o[0][5:19]} npc={kv['npc_id']} {kv.get('tag')} y={kv['y']:.2f} tgt_y={ty:.2f} dy={kv['y']-ty:.2f} out={kv.get('decision_outcome')} next_wp={kv.get('next_wp')} dest={kv.get('dest')}")
