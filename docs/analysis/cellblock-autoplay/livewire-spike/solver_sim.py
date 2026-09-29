import json,random,math,sys
A={k:set(map(tuple,v)) for k,v in json.load(open('livewire_atlas.json')).items()}
LIBS=set(A)
LV=[dict(pb=6,ptc=0.075472,g=2,mb=2,mtc=0.188679,ob=2,otc=0.188679),
    dict(pb=6,ptc=0.094340,g=4,mb=2,mtc=0.188679,ob=2,otc=0.188679),
    dict(pb=6,ptc=0.094340,g=4,mb=3,mtc=0.245283,ob=3,otc=0.245283),
    dict(pb=6,ptc=0.150943,g=6,mb=5,mtc=0.320755,ob=3,otc=0.245283)]
PW=["pGray","pRust","pBronze","pGreen"]; PE=["pProcessorFan1","pPurpleBlack1","pPurpleBlack2","pOrangeSilver1","pOrangeSilver2","pLEDWhite","pLEDRed","pLEDBlue","pLEDGreen","pLEDYellow"]
MW=["mYellowStripe","mPurpleStripe","mOrangeStripe","mBlackStripe"]; ME=["mGear","mMicroChip1","mMicroChip2"]
GW=["gGreenBlack","gRedSilver","gBlackGreen"]; OW=["oRed","oGreen","oBlack"]; YS=[391,497,577,657,783]; X0=275.6
def board(diff,tc,fixed,rng):
    L=LV[diff-1]; W={}
    def rdepth(lo,hi):
        while True:
            d=rng.randrange(lo,hi)
            if d not in W: return d
    def adepth(s):
        d=s
        while d in W: d+=1
        return d
    sfx=(lambda: str(rng.randint(1,4))) if fixed else (lambda: '')
    gsfx=(lambda: str(rng.randint(1,4))) if fixed else (lambda: str(rng.randrange(1,4)))
    pt=math.floor(L['pb']+tc*L['ptc']); mt=math.floor(L['mb']+tc*L['mtc']); ot=math.floor(L['ob']+tc*L['otc'])
    e=math.floor(pt*0.3)
    for i in range(e): W[adepth(101)]=('pe%d'%(i+1),rng.choice(PE),rng.uniform(375,900),rng.uniform(250,700))
    for i in range(pt-e): W[rdepth(1,100)]=('p%d'%(i+1),rng.choice(PW)+sfx(),X0,rng.choice(YS))
    for i in range(L['g']): W[rdepth(1,100)]=('g%d'%(i+1),rng.choice(GW)+gsfx(),X0,rng.choice(YS))
    e=math.floor(mt*0.3)
    for i in range(e): W[adepth(101)]=('me%d'%(i+1),rng.choice(ME),rng.uniform(375,900),rng.uniform(250,700))
    for i in range(mt-e): W[rdepth(1,100)]=('m%d'%(i+1),rng.choice(MW)+sfx(),X0,rng.choice(YS))
    for i in range(ot): W[rdepth(1,100)]=('o%d'%(i+1),rng.choice(OW)+gsfx(),X0,rng.choice(YS))
    return W
def cells(lib,x,y):
    return {(int(round((x+lx)/5))*5,int(round((y+ly)/5))*5) for lx,ly in A[lib]}
def blocked(p): x,y=p; return x<445 or x>1270 or y<110 or y>890
def solve(W):
    live={d:w for d,w in W.items() if w[1] in LIBS}
    C={d:cells(w[1],w[2],w[3]) for d,w in live.items()}
    cut=set(); clicks=0; stuck=0
    goals=[d for d,w in live.items() if w[0].startswith('g') and not w[0].startswith('ge')]
    def top(p):
        best=None
        for d in C:
            if d in cut: continue
            if p in C[d] and (best is None or d>best): best=d
        return best
    for _ in range(40):
        rem=[g for g in goals if g not in cut]
        if not rem: return clicks,0
        progressed=False
        for g in rem:
            exp=[p for p in C[g] if not blocked(p) and top(p)==g]
            if exp:
                cut.add(g); clicks+=1; progressed=True; break
        if progressed: continue
        # cut a cuttable occluder (o/m, not p*) that tops the most points of a remaining goal
        cnt={}
        for g in rem:
            for p in C[g]:
                if blocked(p): continue
                t=top(p)
                if t is not None and live[t][1][0] in 'om': cnt[t]=cnt.get(t,0)+1
        if not cnt: return clicks,len(rem)
        o=max(cnt,key=cnt.get); cut.add(o); clicks+=1
    return clicks,len([g for g in goals if g not in cut])
rng=random.Random(1)
for fixed in (False,True):
    for diff in (1,2,3,4):
        N=300; ok=0; tot=0; worst=0
        for _ in range(N):
            c,left=solve(board(diff,1,fixed,rng)); ok+=left==0; tot+=c; worst=max(worst,c)
        print(f'libfix={fixed} diff={diff}: solved {ok}/{N}, mean clicks {tot/N:.2f}, max {worst}')
