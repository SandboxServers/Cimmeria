import struct,sys
d=open(sys.argv[1],"rb").read(); body=d[8:]; NB=int(("".join(f"{x:08b}" for x in body[:2]))[:5],2)
class BR:
    def __init__(s,b,p=0): s.b=b; s.p=p*8
    def u(s,n):
        v=0
        for _ in range(n):
            v=(v<<1)|((s.b[s.p>>3]>>(7-(s.p&7)))&1); s.p+=1
        return v
    def sb(s,n):
        if n==0: return 0
        v=s.u(n); return v-(1<<n) if v>>(n-1) else v
    def align(s): s.p=(s.p+7)&~7
def rect(b,p=0):
    r=BR(b,p); n=r.u(5); v=[r.sb(n) for _ in range(4)]; r.align(); return v,(r.p>>3)
def matrix(r):
    sx=sy=1.0; r0=r1=0.0
    if r.u(1): n=r.u(5); sx=r.sb(n)/65536; sy=r.sb(n)/65536
    if r.u(1): n=r.u(5); r0=r.sb(n)/65536; r1=r.sb(n)/65536
    n=r.u(5); tx=r.sb(n); ty=r.sb(n); r.align(); return (sx,r0,r1,sy,tx,ty)
shapes={}; sprites={}; exports={}
def tags(buf,cur):
    p=0
    while p<len(buf):
        h=struct.unpack('<H',buf[p:p+2])[0]; p+=2; code=h>>6; ln=h&0x3f
        if ln==0x3f: ln=struct.unpack('<I',buf[p:p+4])[0]; p+=4
        data=buf[p:p+ln]; p+=ln
        if code==0:
            if cur is not None: return
            continue
        if code in (2,22,32,83,46,84,10,11,33,37,48,75):
            cid=struct.unpack('<H',data[:2])[0]
            try: shapes[cid]=rect(data,2)[0]
            except: pass
        elif code==39:
            sid=struct.unpack('<H',data[:2])[0]; sprites[sid]=[]; tags(data[4:],sid)
        elif code==56:
            cnt=struct.unpack('<H',data[:2])[0]; q=2
            for _ in range(cnt):
                cid=struct.unpack('<H',data[q:q+2])[0]; q+=2; e=data.index(b'\0',q); exports[data[q:e].decode()]=cid; q=e+1
        elif code in (26,70) and cur is not None:
            flags=data[0]; q=1
            if code==70: q=2
            q+=2  # depth
            if code==70 and (data[1]&0x08): e=data.index(b'\0',q); q=e+1
            if flags&0x02:
                cid=struct.unpack('<H',data[q:q+2])[0]; q+=2
                m=(1,0,0,1,0,0)
                if flags&0x04:
                    r=BR(data,q); m=matrix(r)
                sprites[cur].append((cid,m))
tags(body[(5+4*NB+7)//8+4:],None)
def bbox(cid,depth=0):
    if cid in shapes and cid not in sprites: return shapes[cid]
    if cid not in sprites or depth>6: return None
    xs=[];ys=[]
    for c,(sx,r0,r1,sy,tx,ty) in sprites[cid]:
        b=bbox(c,depth+1)
        if not b: continue
        for x in (b[0],b[1]):
            for y in (b[2],b[3]):
                xs.append(sx*x+r1*y+tx); ys.append(r0*x+sy*y+ty)
    if not xs: return None
    return [min(xs),max(xs),min(ys),max(ys)]
for name in sorted(exports):
    if name[0] in 'gomp' and not name.startswith('hack'):
        b=bbox(exports[name])
        if b: print(f'{name:16s} x[{b[0]/20:7.1f},{b[1]/20:7.1f}] y[{b[2]/20:7.1f},{b[3]/20:7.1f}]')

print('--- named placements')
def named(buf,path,depth=0):
    p=0
    while p<len(buf):
        h=struct.unpack('<H',buf[p:p+2])[0]; p+=2; code=h>>6; ln=h&0x3f
        if ln==0x3f: ln=struct.unpack('<I',buf[p:p+4])[0]; p+=4
        data=buf[p:p+ln]; p+=ln
        if code==0 and depth: return
        if code==39 and depth==0: continue
        if code in (26,70):
            flags=data[0]; q=1+(1 if code==70 else 0); q+=2
            if code==70 and (data[1]&0x08): e=data.index(b'\0',q); q=e+1
            cid=None; m=(1,0,0,1,0,0)
            if flags&0x02: cid=struct.unpack('<H',data[q:q+2])[0]; q+=2
            if flags&0x04:
                r=BR(data,q); m=matrix(r); q=r.p>>3
            if flags&0x08:
                r=BR(data,q)
                # cxform with alpha
                ha=r.u(1); hm=r.u(1); n=r.u(4)
                if hm: [r.sb(n) for _ in range(4)]
                if ha: [r.sb(n) for _ in range(4)]
                r.align(); q=r.p>>3
            if flags&0x10: q+=2
            if flags&0x20:
                e=data.index(b'\0',q); nm=data[q:e].decode('latin1')
                b=bbox(cid) if cid else None
                print(path, nm, 'cid',cid,'tx,ty',m[4]/20,m[5]/20,'scale',round(m[0],3),round(m[3],3),'bbox',[round(v/20,1) for v in b] if b else None)
named(body[(5+4*NB+7)//8+4:],'_root')
