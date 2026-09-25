import json,sys
def load(p):
    t=open(p,encoding='utf-8').read()
    d=json.loads(t[t.find('{'):])
    rows=d['data']['data']['results'][0]['rows']
    out=[]
    for r in rows:
        x=r['data']; kv={}
        for k in ('attributes_string','attributes_number','attributes_bool'): kv.update(x.get(k) or {})
        out.append((r['timestamp'],x['severity_text'],x['scope_name'].replace('cimmeria_services::',''),x['body'],kv))
    out.sort(key=lambda o:o[0]); return out
def line(o,drop=()):
    ts,sev,sc,b,kv=o
    return f"{ts[5:23]} {sev[:1]} {sc} | {b} | "+' '.join(f'{k}={v}' for k,v in sorted(kv.items()) if k not in drop)
if __name__=='__main__':
    for o in load(sys.argv[1]):
        if o[2]=='playtest.bookmark.entity': continue
        print(line(o))
