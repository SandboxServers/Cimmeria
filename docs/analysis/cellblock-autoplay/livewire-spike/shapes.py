import struct,sys,math,os
GFX=os.environ["GFX"]
exec(open(os.path.join(os.path.dirname(os.path.abspath(__file__)),"bounds_core.py")).read())
# ---- shape parsing ----
rawshapes={}
def collect(buf):
    p=0
    while p<len(buf):
        h=struct.unpack('<H',buf[p:p+2])[0]; p+=2; code=h>>6; ln=h&0x3f
        if ln==0x3f: ln=struct.unpack('<I',buf[p:p+4])[0]; p+=4
        data=buf[p:p+ln]; p+=ln
        if code in (2,22,32,83): rawshapes[struct.unpack('<H',data[:2])[0]]=(code,data)
collect(body[(5+4*NB+7)//8+4:])
def parse_shape(code,data):
    ver={2:1,22:2,32:3,83:4}[code]
    _,q=rect(data,2)
    if ver==4:
        _,q=rect(data,q); q+=1
    r=BR(data,q)
    def byte(): v=r.u(8); return v
    def u16(): lo=r.u(8); hi=r.u(8); return lo|(hi<<8)
    def rgb(alpha):
        for _ in range(4 if alpha else 3): r.u(8)
    def mat(): 
        m=matrix(r)
    def gradient(focal):
        r.u(2); r.u(2); n=r.u(4)
        for _ in range(n): r.u(8); rgb(ver>=3)
        if focal: u16()
    def fillstyles():
        n=byte()
        if n==0xff and ver>=2: n=u16()
        out=[]
        for _ in range(n):
            t=byte()
            if t==0: rgb(ver>=3)
            elif t in (0x10,0x12,0x13): mat(); gradient(t==0x13)
            elif t in (0x40,0x41,0x42,0x43): u16(); mat()
            else: raise ValueError('fill %x'%t)
            out.append(t)
        return out
    def linestyles():
        n=byte()
        if n==0xff and ver>=2: n=u16()
        out=[]
        for _ in range(n):
            w=u16()
            if ver==4:
                sc=r.u(2); join=r.u(2); hasfill=r.u(1); r.u(1);r.u(1);r.u(1);r.u(5); r.u(1); ec=r.u(2)
                if join==2: u16()
                if hasfill:
                    t=byte()
                    if t==0: rgb(True)
                    elif t in (0x10,0x12,0x13): mat(); gradient(t==0x13)
                    elif t in (0x40,0x41,0x42,0x43): u16(); mat()
                else: rgb(True)
            else: rgb(ver>=3)
            out.append(w)
        return out
    fills=fillstyles(); lines=linestyles()
    nf=r.u(4); nl=r.u(4)
    x=y=0; f0=f1=ls=0; edges=[]; lw=lines
    while True:
        if r.u(1)==0:
            flags=r.u(5)
            if flags==0: break
            if flags&1:
                n=r.u(5); x=r.sb(n); y=r.sb(n)
            if flags&2: f0=r.u(nf)
            if flags&4: f1=r.u(nf)
            if flags&8: ls=r.u(nl)
            if flags&16:
                r.align(); fills=fillstyles(); lw=linestyles(); nf=r.u(4); nl=r.u(4)
        else:
            straight=r.u(1); n=r.u(4)+2
            if straight:
                if r.u(1): dx=r.sb(n); dy=r.sb(n)
                else:
                    if r.u(1): dx=0; dy=r.sb(n)
                    else: dx=r.sb(n); dy=0
                edges.append(((x,y),(x+dx,y+dy),f0,f1,lw[ls-1] if ls else 0)); x+=dx; y+=dy
            else:
                cx=x+r.sb(n); cy=y+r.sb(n); ax=cx+r.sb(n); ay=cy+r.sb(n)
                px,py=x,y
                for k in range(1,5):
                    t=k/4; nx=(1-t)**2*x+2*(1-t)*t*cx+t*t*ax; ny=(1-t)**2*y+2*(1-t)*t*cy+t*t*ay
                    edges.append(((px,py),(nx,ny),f0,f1,lw[ls-1] if ls else 0)); px,py=nx,ny
                x,y=ax,ay
    return edges
shape_edges={}
for cid,(code,data) in rawshapes.items():
    try: shape_edges[cid]=parse_shape(code,data)
    except Exception as e: shape_edges[cid]=None
def flat(cid,m=(1,0,0,1,0,0),depth=0):
    """return list of transformed edges (p,q,filled_boundary,halfwidth)"""
    out=[]
    if cid in shape_edges and cid not in sprites:
        es=shape_edges[cid]
        if not es: return out
        sx,r0,r1,sy,tx,ty=m
        T=lambda p:(sx*p[0]+r1*p[1]+tx, r0*p[0]+sy*p[1]+ty)
        sc=math.sqrt(abs(sx*sy-r0*r1)) or 1
        for p,q,f0,f1,w in es:
            out.append((T(p),T(q),(f0==0)!=(f1==0),w*sc/2))
        return out
    if cid in sprites and depth<8:
        for c,cm in sprites[cid]:
            a,b,c2,d,e,f=cm; A,B,C,D,E,F=m
            # compose m * cm
            nm=(A*a+C*b, B*a+D*b, A*c2+C*d, B*c2+D*d, A*e+C*f+E, B*e+D*f+F)
            out+=flat(c,nm,depth+1)
    return out
def hit(edges,px,py):
    # any stroke within halfwidth, or inside fill by even-odd over boundary edges
    cross=0
    for (x1,y1),(x2,y2),bnd,hw in edges:
        if hw>0:
            dx,dy=x2-x1,y2-y1; L=dx*dx+dy*dy
            t=0 if L==0 else max(0,min(1,((px-x1)*dx+(py-y1)*dy)/L))
            if (px-x1-t*dx)**2+(py-y1-t*dy)**2<=hw*hw: return True
        if bnd and ((y1>py)!=(y2>py)):
            xi=x1+(py-y1)*(x2-x1)/(y2-y1)
            if xi>px: cross+=1
    return cross%2==1
