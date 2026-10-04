import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { parseHTML } from 'linkedom';
import { mountInstall } from './install-view';
import type { InstallRequest, InstallStatus } from './install-workflow';
const html=await readFile(new URL('../ui/index.html',import.meta.url),'utf8');
const id='d539d049-61b7-4c82-b3d7-cb9b7a991adc';
const initial=():InstallStatus=>({schema_version:1,install_supported:true,progress:null,outcome:null,native:{schema_version:1,
  requires_reopen:false,operation:{schema_version:1,revision:0,operation:null},preferences:{schema_version:1,revision:1,
    install_directory:'/fixture',launcher_summary_consent:false}}});
const flush=()=>new Promise<void>(resolve=>setImmediate(resolve));
function dom(){const {document,window}=parseHTML(html);return {document:document as unknown as Document,
  get:(id:string)=>document.getElementById(id)! as unknown as HTMLButtonElement,
  click:(id:string)=>document.getElementById(id)!.dispatchEvent(new window.Event('click'))};}

test('install click dispatches once; content completion never becomes Play',{timeout:5000},async()=>{
 const ui=dom();let status=initial();const commands:string[]=[];
 const app=mountInstall(ui.document,async(command,args)=>{
   assert.equal(command,'install_command');const request=args!.request as InstallRequest;commands.push(request.command);
   if(request.command==='install')status={...status,outcome:'content_prepared',native:{...status.native,
     operation:{schema_version:1,revision:1,operation:{id,kind:'install',intent_digest:Array(32).fill(0),state:'succeeded'}}}};
   return status;
 },()=>id);
 try{await app.ready;await flush();assert.equal(ui.get('install').disabled,false);
   ui.click('install');ui.click('install');await app.settled();await flush();
   assert.equal(commands.filter(x=>x==='install').length,1);assert.equal(ui.get('install').textContent,'Content prepared');
   assert.equal(ui.get('install').disabled,true);assert.match(ui.get('install-status').textContent!,/Play are not connected/);
 }finally{await app.dispose();}
});

test('unsupported Mac view never dispatches install; refresh sees newly saved folder',async()=>{
 const ui=dom();let status={...initial(),install_supported:false};let installs=0;
 const app=mountInstall(ui.document,async(_command,args)=>{if((args!.request as InstallRequest).command==='install')installs++;return status;});
 try{await app.ready;await flush();ui.click('install');assert.equal(installs,0);assert.equal(ui.get('install').disabled,true);
   status={...status,install_supported:true};await app.refresh();await flush();assert.equal(ui.get('install').disabled,false);
 }finally{await app.dispose();}
});

test('reopened recovery exposes explicit resume and inspection, not new install',async()=>{
 const ui=dom();const status:InstallStatus={...initial(),native:{...initial().native,operation:{schema_version:1,revision:3,
   operation:{id,kind:'install',intent_digest:Array(32).fill(0),state:'reconciliation_required'}}}};
 const commands:string[]=[];
 const app=mountInstall(ui.document,async(_command,args)=>{commands.push((args!.request as InstallRequest).command);return status;});
 try{await app.ready;await flush();assert.equal(ui.get('resume-install').hidden,false);assert.equal(ui.get('install').disabled,true);
   ui.click('inspect-install');await app.settled();assert.deepEqual(commands,['inspect','inspect','reconcile']);
 }finally{await app.dispose();}
});

test('resume reconnects progress observation and mounted cancel stays available',{timeout:5000},async()=>{
 const ui=dom();let status:InstallStatus={...initial(),native:{...initial().native,operation:{schema_version:1,revision:3,
   operation:{id,kind:'install',intent_digest:Array(32).fill(0),state:'reconciliation_required'}}}};
 let cancelCount=0;
 const app=mountInstall(ui.document,async(_command,args)=>{
   const request=args!.request as InstallRequest;
   if(request.command==='resume')status={...status,native:{...status.native,operation:{...status.native.operation,revision:4,
     operation:{...status.native.operation.operation!,state:'running'}}}};
   if(request.command==='cancel'){cancelCount++;status={...status,outcome:'cancelled',native:{...status.native,
     operation:{...status.native.operation,revision:5,operation:{...status.native.operation.operation!,state:'cancelled'}}}};}
   return status;
 });
 try{await app.ready;await flush();ui.click('resume-install');await app.settled();await flush();
   assert.equal(ui.get('cancel-install').hidden,false);assert.equal(ui.get('cancel-install').disabled,false);
   ui.click('cancel-install');await app.settled();await flush();assert.equal(cancelCount,1);
   assert.equal(ui.get('cancel-install').hidden,true);assert.match(ui.get('install-status').textContent!,/Partial files are preserved/);
 }finally{await app.dispose();}
});
