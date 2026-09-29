exec(open('solver_sim.py').read().split('rng=random.Random(1)')[0])
import statistics
def best_clear(W):
    live={d:w for d,w in W.items() if w[1] in LIBS}
    C={d:cells(w[1],w[2],w[3]) for d,w in live.items()}
    res=[]
    for g,w in live.items():
        if not w[0].startswith('g'): continue
        others=set().union(*[C[d] for d in C if d>g]) if any(d>g for d in C) else set()
        best=0
        for p in C[g]:
            if blocked(p) or p in others: continue
            # clearance: min steps (5px) to leave g's cells or hit others, radius search up to 20px
            r=0
            for rr in range(5,25,5):
                ring=[(p[0]+dx,p[1]+dy) for dx in range(-rr,rr+1,5) for dy in range(-rr,rr+1,5)]
                if all(q in C[g] and q not in others for q in ring): r=rr
                else: break
            best=max(best,r)
        res.append(best)
    return res
rng=random.Random(7)
for fixed in (False,True):
  for diff in (1,4):
    allc=[]
    for _ in range(150): allc+=best_clear(board(diff,1,fixed,rng))
    print(f'libfix={fixed} diff={diff}: goals {len(allc)}, clearance(px) min {min(allc)} median {statistics.median(allc)}; share >=5px {sum(c>=5 for c in allc)/len(allc):.2f}')
