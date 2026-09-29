import struct,sys
d=open(sys.argv[1],'rb').read()
body=d[8:]
bits=''.join(f'{x:08b}' for x in body[:20]); n=int(bits[:5],2)
off=(5+4*n+7)//8+4
OPN={0x04:'next',0x05:'prev',0x06:'play',0x07:'stop',0x0A:'add',0x0B:'sub',0x0C:'mul',0x0D:'div',0x0E:'eq',0x0F:'lt',0x10:'and',0x11:'or',0x12:'not',0x17:'pop',0x1C:'getvar',0x1D:'setvar',0x20:'settarget2',0x21:'strcat',0x22:'getprop',0x23:'setprop',0x26:'trace',0x34:'gettime',0x3A:'delete',0x3C:'deflocal',0x3D:'callfunc',0x3E:'return',0x3F:'mod',0x40:'new',0x41:'deflocal2',0x42:'initarray',0x43:'initobj',0x44:'typeof',0x47:'add2',0x48:'lt2',0x49:'eq2',0x4A:'tonum',0x4B:'tostr',0x4C:'dup',0x4D:'swap',0x4E:'getmember',0x4F:'setmember',0x50:'incr',0x51:'decr',0x52:'callmethod',0x53:'newmethod',0x54:'instanceof',0x55:'enum2',0x60:'bitand',0x61:'bitor',0x62:'bitxor',0x63:'shl',0x64:'shr',0x66:'stricteq',0x67:'gt',0x87:'storereg',0x88:'cpool',0x8E:'deffunc2',0x94:'with',0x96:'push',0x99:'jump',0x9B:'deffunc',0x9D:'if',0x9E:'call',0x9F:'gotoframe2',0x81:'gotoframe',0x83:'geturl',0x8C:'gotolabel',0x9A:'geturl2',0x8B:'settarget',0x8D:'waitframe2',0x2B:'cast',0x69:'extends',0x2C:'implements',0x24:'clone',0x25:'removeclip',0x27:'startdrag',0x28:'enddrag',0x45:'targetpath',0x46:'enum',0x30:'random',0x3B:'delete2',0x68:'strgt',0x2A:'throw',0x8F:'try'}
def dis(code,out,cp_holder):
    i=0
    while i<len(code):
        op=code[i]; i+=1
        if op==0: out.append('end'); continue
        ln=0; payload=b''
        if op>=0x80:
            ln=struct.unpack('<H',code[i:i+2])[0]; i+=2; payload=code[i:i+ln]; i+=ln
        name=OPN.get(op,hex(op))
        if op==0x88:
            cnt=struct.unpack('<H',payload[:2])[0]; parts=payload[2:].split(b'\0')[:cnt]
            cp_holder[:]=[p.decode('latin1') for p in parts]; out.append(f'cpool[{cnt}]'); continue
        if op==0x96:
            j=0; vals=[]
            while j<len(payload):
                t=payload[j]; j+=1
                if t==0: e=payload.index(b'\0',j); vals.append(repr(payload[j:e].decode('latin1'))); j=e+1
                elif t==1: vals.append(str(struct.unpack('<f',payload[j:j+4])[0])); j+=4
                elif t==2: vals.append('null')
                elif t==3: vals.append('undef')
                elif t==4: vals.append(f'r{payload[j]}'); j+=1
                elif t==5: vals.append(str(bool(payload[j]))); j+=1
                elif t==6: vals.append(str(struct.unpack('<d',payload[j+4:j+8]+payload[j:j+4])[0])); j+=8
                elif t==7: vals.append(str(struct.unpack('<i',payload[j:j+4])[0])); j+=4
                elif t==8: k=payload[j]; j+=1; vals.append(repr(cp_holder[k]) if k<len(cp_holder) else f'c{k}')
                elif t==9: k=struct.unpack('<H',payload[j:j+2])[0]; j+=2; vals.append(repr(cp_holder[k]) if k<len(cp_holder) else f'c{k}')
                else: break
            out.append('push '+', '.join(vals)); continue
        if op in (0x9B,0x8E):
            e=payload.index(b'\0'); fn=payload[:e].decode('latin1')
            out.append(f'{name} {fn!r}'); continue
        if op in (0x99,0x9D):
            out.append(f'{name} {struct.unpack("<h",payload)[0]}'); continue
        if op==0x87: out.append(f'storereg r{payload[0]}'); continue
        out.append(name)
def tags(buf,depth,out):
    p=0
    while p<len(buf):
        h=struct.unpack('<H',buf[p:p+2])[0]; p+=2
        code=h>>6; ln=h&0x3f
        if ln==0x3f: ln=struct.unpack('<I',buf[p:p+4])[0]; p+=4
        data=buf[p:p+ln]; p+=ln
        if code==0: 
            if depth: return
            continue
        if code==12: out.append(f'--- DoAction depth{depth}'); dis(data,out,[])
        elif code==59: out.append(f'--- DoInitAction sprite {struct.unpack("<H",data[:2])[0]}'); dis(data[2:],out,[])
        elif code==39: out.append(f'--- Sprite {struct.unpack("<H",data[:2])[0]}'); tags(data[4:],depth+1,out)
        elif code==56:
            cnt=struct.unpack('<H',data[:2])[0]; q=2; names=[]
            for _ in range(cnt):
                cid=struct.unpack('<H',data[q:q+2])[0]; q+=2; e=data.index(b'\0',q); names.append((cid,data[q:e].decode('latin1'))); q=e+1
            out.append(f'--- Export {names}')
        elif code==26 or code==70:
            # PlaceObject2/3 with clip actions
            pass
out=[]
tags(body[off:],0,out)
open(sys.argv[2],'w').write('\n'.join(out))
print(len(out))
