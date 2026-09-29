import sys,os
sys.path.insert(0,os.path.dirname(os.path.abspath(__file__)))
from shapes import *
# root placements by name
roots={}
def named(buf):
    p=0
    while p<len(buf):
        h=struct.unpack('<H',buf[p:p+2])[0]; p+=2; code=h>>6; ln=h&0x3f
        if ln==0x3f: ln=struct.unpack('<I',buf[p:p+4])[0]; p+=4
        data=buf[p:p+ln]; p+=ln
        if code in (26,70):
            flags=data[0]; q=1+(1 if code==70 else 0); dep=struct.unpack('<H',data[q:q+2])[0]; q+=2
            if code==70 and (data[1]&0x08): e=data.index(b'\0',q); q=e+1
            cid=None; m=(1,0,0,1,0,0)
            if flags&0x02: cid=struct.unpack('<H',data[q:q+2])[0]; q+=2
            if flags&0x04: r=BR(data,q); m=matrix(r); q=r.p>>3
            if flags&0x08:
                r=BR(data,q); ha=r.u(1); hm=r.u(1); n=r.u(4)
                if hm: [r.sb(n) for _ in range(4)]
                if ha: [r.sb(n) for _ in range(4)]
                r.align(); q=r.p>>3
            if flags&0x10: q+=2
            if flags&0x20:
                e=data.index(b'\0',q); roots[data[q:e].decode()]=(cid,m,dep)
named(body[(5+4*NB+7)//8+4:])
for nm in ('mask_mc','wireCover','cover_mc','load_mc'):
    cid,m,dep=roots[nm]; E=flat(cid,m)
    print(nm,'depth',dep,'edges',len(E))
    if nm=='load_mc': continue
    rows=[]
    for sy in range(100,960,40):
        rows.append(''.join('#' if hit(E,sx*20,sy*20) else '.' for sx in range(0,1280,20)))
    print('\n'.join(f'{100+40*i:4d} '+r for i,r in enumerate(rows)))
