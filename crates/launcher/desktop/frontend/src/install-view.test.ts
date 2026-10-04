import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { parseHTML } from 'linkedom';
import { mountInstall } from './install-view';
import type { InstallRequest, InstallStatus } from './install-workflow';
const html=await readFile(new URL('../ui/index.html',import.meta.url),'utf8');
const id='d539d049-61b7-4c82-b3d7-cb9b7a991adc';
const initial=():InstallStatus=>({schema_version:1,install_supported:true,can_resume:true,can_reconcile:true,can_retry:false,uninstall:null,runtime_setup:null,progress:null,outcome:null,native:{schema_version:1,
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
   assert.equal(ui.get('install').disabled,true);assert.match(ui.get('install-status').textContent!,/cannot continue compatibility setup/);
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

for (const [outcome, message] of [['rosetta_required', /Rosetta is required/], ['runtime_unavailable', /compatibility could not be prepared/]] as const) {
 test(`native ${outcome} is readable and never enables Play`,async()=>{
  const ui=dom();const status:InstallStatus={...initial(),outcome,native:{...initial().native,
   operation:{schema_version:1,revision:2,operation:{id,kind:'install',intent_digest:Array(32).fill(0),state:'failed'}}}};
  const app=mountInstall(ui.document,async()=>status);
  try{await app.ready;await flush();assert.match(ui.get('install-status').textContent!,message);
   assert.equal(ui.get('install').disabled,true);assert.equal(ui.get('cancel-install').hidden,true);
  }finally{await app.dispose();}
 });
}

test('Wine recovery cannot invoke native resume or reconciliation',async()=>{
 const ui=dom();const status:InstallStatus={...initial(),can_resume:false,can_reconcile:false,native:{...initial().native,
  operation:{schema_version:1,revision:2,operation:{id,kind:'install',intent_digest:Array(32).fill(0),state:'reconciliation_required'}}}};
 const calls:string[]=[];const app=mountInstall(ui.document,async(_command,args)=>{calls.push((args!.request as InstallRequest).command);return status;});
 try{await app.ready;await flush();assert.equal(ui.get('resume-install').hidden,true);
  assert.match(ui.get('install-status').textContent!,/recovery is not available/);
  ui.click('resume-install');ui.click('inspect-install');await app.settled();
  assert.ok(calls.every(command=>command==='inspect'));
 }finally{await app.dispose();}
});

test('uninstall requires confirmation, dismisses safely and enables reinstall only after acknowledgement',async()=>{
 const ui=dom();let status:InstallStatus={...initial(),uninstall:{installation_id:id,directory:'/owned/game',recovery:false},
  native:{...initial().native,operation:{schema_version:1,revision:3,operation:{id,kind:'install',intent_digest:Array(32).fill(0),state:'succeeded'}}}};
 const removalId='11b003de-8094-43d7-8daa-f3f3bd577c40';const calls:InstallRequest[]=[];
 const app=mountInstall(ui.document,async(_command,args)=>{const request=args!.request as InstallRequest;calls.push(request);
  if(request.command==='uninstall'){assert.equal(request.installation_id,id);assert.equal(request.confirmed,true);
   status={...status,uninstall:null,can_retry:true,outcome:null,native:{...status.native,
    operation:{schema_version:1,revision:6,operation:{id:removalId,kind:'uninstall',intent_digest:Array(32).fill(0),state:'succeeded'}}}};}
  return status;
 },()=>removalId);
 try{await app.ready;await flush();assert.equal(ui.get('uninstall').disabled,false);
  ui.click('uninstall');assert.equal(ui.get('uninstall-confirmation').hidden,false);assert.equal(ui.get('uninstall-directory').textContent,'/owned/game');
  assert.equal(calls.every(c=>c.command==='inspect'),true);ui.click('dismiss-uninstall');assert.equal(ui.get('uninstall-confirmation').hidden,true);
  ui.click('uninstall');ui.click('confirm-uninstall');ui.click('confirm-uninstall');await app.settled();await flush();
  assert.equal(calls.filter(c=>c.command==='uninstall').length,1);assert.equal(ui.get('install').disabled,false);
  assert.equal(ui.get('install').textContent,'Install Stargate Worlds');assert.match(ui.get('install-status').textContent!,/Game uninstalled/);
  assert.equal(status.native.preferences.launcher_summary_consent,false);
 }finally{await app.dispose();}
});

test('lost uninstall reply only inspects; explicit recovery reuses operation identity',async()=>{
 const ui=dom();const removalId='11b003de-8094-43d7-8daa-f3f3bd577c40';
 let status:InstallStatus={...initial(),uninstall:{installation_id:id,directory:'/owned/game',recovery:false},
  native:{...initial().native,operation:{schema_version:1,revision:3,operation:{id,kind:'install',intent_digest:Array(32).fill(0),state:'succeeded'}}}};
 const calls:InstallRequest[]=[];
 const app=mountInstall(ui.document,async(_command,args)=>{const request=args!.request as InstallRequest;calls.push(request);
  if(request.command==='uninstall'){status={...status,uninstall:{...status.uninstall!,recovery:true},can_reconcile:false,can_resume:false,
    native:{...status.native,operation:{schema_version:1,revision:5,operation:{id:removalId,kind:'uninstall',intent_digest:Array(32).fill(0),state:'reconciliation_required'}}}};
   throw new Error('lost reply');}
  return status;
 },()=>removalId);
 try{await app.ready;await flush();ui.click('uninstall');ui.click('confirm-uninstall');await app.settled();await flush();
  assert.equal(calls.filter(c=>c.command==='uninstall').length,1);assert.equal(ui.get('uninstall').disabled,true);
  ui.click('inspect-install');await app.settled();await flush();assert.equal(calls.filter(c=>c.command==='uninstall').length,1);
  assert.equal(ui.get('uninstall').textContent,'Finish uninstall…');assert.equal(ui.get('cancel-install').hidden,true);
  ui.click('uninstall');ui.click('confirm-uninstall');await app.settled();await flush();
  const removals=calls.filter(c=>c.command==='uninstall');assert.equal(removals.length,2);
  assert.ok(removals.every(c=>c.operation_id===removalId));assert.ok(calls.every(c=>c.command!=='reconcile'));
 }finally{await app.dispose();}
});

for (const state of ['running','succeeded','reconciliation_required'] as const) {
 test(`runtime setup ${state} remains distinct from installation and Play`,async()=>{
  const ui=dom();const status:InstallStatus={...initial(),can_resume:false,can_reconcile:false,
   native:{...initial().native,operation:{schema_version:1,revision:5,
    operation:{id,kind:'prepare_runtime',state,intent_digest:Array(32).fill(0)}}}};
  const calls:string[]=[];const app=mountInstall(ui.document,async(_command,args)=>{
   calls.push((args!.request as InstallRequest).command);return status;
  });
  try {await app.ready;await flush();assert.equal(ui.get('install').disabled,true);
   assert.match(ui.get('install-status').textContent!,state==='succeeded'?/Graphics and Play still need validation/:/compatibility|Compatibility/);
   ui.click('install');await flush();assert.ok(calls.every(call=>call==='inspect'));
   assert.equal(ui.get('cancel-install').hidden,state!=='running');
   assert.equal(status.native.preferences.launcher_summary_consent,false);
  } finally {await app.dispose();}
 });
}


const runtimeId='a2ec19a1-7345-43ef-aef5-8a944d8ee26c';
const prepared=():InstallStatus=>({...initial(),runtime_setup:id,outcome:'content_prepared',native:{...initial().native,
 operation:{schema_version:1,revision:3,operation:{id,kind:'install',state:'succeeded',intent_digest:Array(32).fill(0)}}}});
async function until(predicate:()=>boolean){for(let n=0;n<100&&!predicate();n++)await new Promise(resolve=>setTimeout(resolve,10));assert.ok(predicate());}
for(const delayed of [false,true])test(`one Install click advances to compatibility (${delayed?'observed':'immediate'} completion)`,{timeout:5000},async()=>{
 const ui=dom();let status=initial();const calls:InstallRequest[]=[];let counter=0;
 const app=mountInstall(ui.document,async(_command,args)=>{
  const request=args!.request as InstallRequest;calls.push(request);
  if(request.command==='install'){status=prepared();if(delayed)status={...status,runtime_setup:null,native:{...status.native,operation:{...status.native.operation,operation:{...status.native.operation.operation!,state:'running'}}}};}
  if(request.command==='prepare_runtime'){
   assert.equal(request.installation_id,id);assert.equal(request.operation_id,runtimeId);assert.equal(request.operation_revision,3);
   status={...status,runtime_setup:null,outcome:null,native:{...status.native,operation:{schema_version:1,revision:4,
    operation:{id:runtimeId,kind:'prepare_runtime',state:'running',intent_digest:Array(32).fill(0)}}}};
  }
  if(request.command==='cancel')status={...status,native:{...status.native,operation:{...status.native.operation,revision:5,
   operation:{...status.native.operation.operation!,state:'cancel_requested'}}}};
  return status;
 },()=>counter++===0?id:runtimeId);
 try{await app.ready;await flush();ui.click('install');await app.settled();await flush();
  if(delayed){assert.equal(calls.filter(c=>c.command==='prepare_runtime').length,0);status=prepared();}
  await until(()=>calls.some(c=>c.command==='prepare_runtime'));await app.settled();await flush();
  assert.equal(calls.filter(c=>c.command==='install').length,1);assert.equal(calls.filter(c=>c.command==='prepare_runtime').length,1);
  assert.equal(ui.get('install').textContent,'Checking compatibility…');assert.equal(ui.get('cancel-install').hidden,false);
  ui.click('cancel-install');await app.settled();await flush();
  assert.equal(calls.find(c=>c.command==='cancel')?.operation_id,runtimeId);assert.match(ui.get('install-status').textContent!,/Cancellation requested/);
  assert.equal(status.native.preferences.launcher_summary_consent,false);
 }finally{await app.dispose();}
});

test('reopen requires Continue; lost setup reply is inspected without replay',async()=>{
 const ui=dom();let status=prepared();const calls:InstallRequest[]=[];
 const app=mountInstall(ui.document,async(_command,args)=>{
  const request=args!.request as InstallRequest;calls.push(request);
  if(request.command==='prepare_runtime'){
   status={...status,runtime_setup:null,native:{...status.native,operation:{schema_version:1,revision:4,
    operation:{id:runtimeId,kind:'prepare_runtime',state:'running',intent_digest:Array(32).fill(0)}}}};
   throw new Error('lost reply');
  }return status;
 },()=>runtimeId);
 try{await app.ready;await flush();assert.equal(ui.get('install').textContent,'Continue installation');
  assert.equal(calls.every(c=>c.command==='inspect'),true);ui.click('install');ui.click('install');await app.settled();await flush();
  assert.match(ui.get('install-status').textContent!,/Could not confirm/);await app.refresh();await flush();
  assert.equal(calls.filter(c=>c.command==='prepare_runtime').length,1);assert.ok(calls.every(c=>c.command!=='install'));
  assert.equal(ui.get('install').disabled,true);assert.equal(status.native.preferences.launcher_summary_consent,false);
 }finally{await app.dispose();}
});

test('cancelled journey cannot advance if content completion wins the cancellation race',{timeout:5000},async()=>{
 const ui=dom();let status=initial();const calls:InstallRequest[]=[];
 const app=mountInstall(ui.document,async(_command,args)=>{
  const request=args!.request as InstallRequest;calls.push(request);
  if(request.command==='install')status={...prepared(),runtime_setup:null,native:{...prepared().native,
   operation:{...prepared().native.operation,operation:{...prepared().native.operation.operation!,state:'running'}}}};
  if(request.command==='cancel')status=prepared();
  return status;
 },()=>id);
 try{await app.ready;await flush();ui.click('install');await app.settled();await flush();
  ui.click('cancel-install');await app.settled();await flush();await new Promise(resolve=>setTimeout(resolve,300));
  assert.equal(ui.get('install').textContent,'Continue installation');
  assert.equal(calls.filter(c=>c.command==='prepare_runtime').length,0);
  assert.equal(calls.filter(c=>c.command==='cancel').length,1);
 }finally{await app.dispose();}
});

test('observed compatibility recovery is explicit and never dispatches setup again',async()=>{
 const ui=dom();let status:InstallStatus={...initial(),can_resume:false,can_reconcile:true,native:{...initial().native,
  operation:{schema_version:1,revision:7,operation:{id,kind:'prepare_runtime',state:'reconciliation_required',intent_digest:Array(32).fill(0)}}}};
 const calls:InstallRequest[]=[];
 const app=mountInstall(ui.document,async(_command,args)=>{
  const request=args!.request as InstallRequest;calls.push(request);
  if(request.command==='reconcile'){
   assert.equal(request.operation_id,id);assert.equal(request.operation_revision,7);
   status={...status,can_reconcile:false,native:{...status.native,operation:{schema_version:1,revision:8,
    operation:{...status.native.operation.operation!,state:'succeeded'}}}};
  }return status;
 });
 try{await app.ready;await flush();assert.equal(ui.get('inspect-install').textContent,'Recover compatibility setup');
  assert.equal(calls.every(c=>c.command==='inspect'),true);ui.click('inspect-install');await app.settled();await flush();
  assert.equal(calls.filter(c=>c.command==='reconcile').length,1);
  assert.ok(calls.every(c=>c.command!=='install'&&c.command!=='prepare_runtime'));
  assert.equal(ui.get('install').disabled,true);assert.equal(ui.get('install').textContent,'Compatibility checked');
  assert.equal(status.native.preferences.launcher_summary_consent,false);
 }finally{await app.dispose();}
});

test('repair confirmation dismisses without mutation and duplicate confirm sends one saved-owner request',async()=>{
 const ui=dom();let status:InstallStatus={...initial(),repair:{target:{installation_id:'owner',directory:'/owned-game'},recovery:false,cleanup:false}};
 const commands:InstallRequest[]=[];
 const app=mountInstall(ui.document,async(_command,args)=>{const request=args!.request as InstallRequest;commands.push(request);
   if(request.command==='repair'){assert.equal(request.installation_id,'owner');assert.equal(request.confirmed,true);
     status={...status,repair:{target:null,recovery:false,cleanup:true,backup:'retained'},native:{...status.native,operation:{schema_version:1,revision:2,operation:{id,kind:'repair',state:'succeeded',intent_digest:Array(32).fill(0)}}}};}
   if(request.command==='cleanup_repair')status={...status,repair:{...status.repair!,cleanup:false,backup:'removed'}};
   return status;
 },()=>id);
 try{await app.ready;await flush();ui.click('repair');assert.equal(ui.get('repair-directory').textContent,'/owned-game');
   ui.click('dismiss-repair');assert.equal(commands.some(x=>x.command==='repair'),false);
   ui.click('repair');ui.click('confirm-repair');ui.click('confirm-repair');await app.settled();await flush();
   assert.equal(commands.filter(x=>x.command==='repair').length,1);assert.match(ui.get('install-status').textContent!,/Game reconstructed/);
   assert.equal(ui.get('install').disabled,true);assert.equal(ui.get('cleanup-repair').hidden,false);
   ui.click('cleanup-repair');assert.match(ui.get('repair-consequences').textContent!,/Permanently delete/);
   ui.click('confirm-repair');await app.settled();assert.equal(commands.filter(x=>x.command==='cleanup_repair').length,1);
   assert.equal(ui.get('cleanup-repair').hidden,true);assert.match(ui.get('install-status').textContent!,/old backup has been removed/);
   status={...status,repair:{...status.repair!,backup:'not_retained'}};await app.refresh();await flush();
   assert.equal(ui.get('cleanup-repair').hidden,true);assert.match(ui.get('install-status').textContent!,/No old backup was retained/);
 }finally{await app.dispose();}
});

for(const state of ['running','succeeded','reconciliation_required'] as const)test(`install view leaves ${state} launch presentation to launch view`,async()=>{
 const ui=dom();const status:InstallStatus={...initial(),native:{...initial().native,operation:{schema_version:1,revision:2,operation:{id,kind:'launch',state,intent_digest:Array(32).fill(0)}}}};
 const app=mountInstall(ui.document,async()=>status,()=>id);
 try{await app.ready;await flush();assert.equal(ui.get('install').hidden,true);assert.equal(ui.get('install-status').textContent,'');}
 finally{await app.dispose();}
});

test('minimum-version rejection preserves state and reports update requirement',async()=>{
 const ui=dom();const status=initial();let installs=0;
 const app=mountInstall(ui.document,async(_,{request}:any)=>{if(request.command==='install'){installs++;throw 'launcher_too_old';}return status;});
 try{await app.ready;await flush();ui.click('install');ui.click('install');await app.settled();await flush();
 assert.equal(installs,1);assert.match(ui.get('install-status').textContent!,/Update the launcher/);assert.deepEqual(status,initial());
 }finally{await app.dispose();}
});
