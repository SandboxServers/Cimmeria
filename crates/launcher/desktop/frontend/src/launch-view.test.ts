import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parseHTML} from 'linkedom';
import {mountLaunch} from './launch-view';
const html=readFileSync('ui/index.html','utf8');
const initial=()=>({schema_version:1,resources_available:true,launcher_update_required:false,installation_id:'owner',observation:null as unknown,native:{schema_version:1,requires_reopen:false,preferences:{schema_version:1,revision:1,install_directory:'/fixture',launcher_summary_consent:false},operation:{schema_version:1,revision:0,operation:null as unknown}}});
test('Play first click gives feedback, lost reply is observed without replay, unknown stays blocked',async()=>{
 let status=initial();let plays=0;let finish!:()=>void;const wait=new Promise<void>(resolve=>{finish=resolve;});
 const {document,window}=parseHTML(html);
 const app=mountLaunch(document,async(_,{request}:any)=>{
   if(request.command==='play'){plays++;status={...status,installation_id:null as any,observation:{phase:'process_started',host_pid:1,guest_pid:2},native:{...status.native,operation:{schema_version:1,revision:1,operation:{id:request.operation_id,kind:'launch',state:'running',intent_digest:Array(32).fill(0)}}}};await wait;throw 'transport';}
   return status;
 },()=>{},()=> 'attempt');
 const click=()=>document.getElementById('launch')!.dispatchEvent(new window.Event('click'));
 try {await app.ready;await new Promise(resolve=>setImmediate(resolve));click();click();
 assert.match(document.getElementById('launch-status')!.textContent!,/Starting game/);finish();await app.settled();assert.equal(plays,1);
 await app.refresh();await new Promise(resolve=>setImmediate(resolve));assert.match(document.getElementById('launch-status')!.textContent!,/Login and world entry are not verified/);
 assert.equal((document.getElementById('launch') as HTMLButtonElement).disabled,true);
 status={...status,observation:{phase:'unknown'},native:{...status.native,operation:{...status.native.operation,revision:2,operation:{...(status.native.operation.operation as object),state:'reconciliation_required'}}}};
 await app.refresh();await new Promise(resolve=>setImmediate(resolve));assert.match(document.getElementById('launch-status')!.textContent!,/unknown/);click();assert.equal(plays,1);assert.equal(status.native.preferences.launcher_summary_consent,false);
 }finally{await app.dispose();}
});
test('unavailable verified resources never advertise Play',async()=>{
 const {document}=parseHTML(html);const status={...initial(),resources_available:false,installation_id:null};const app=mountLaunch(document,async()=>status);
 try{await app.ready;await new Promise(resolve=>setImmediate(resolve));assert.equal((document.getElementById('launch') as HTMLButtonElement).disabled,true);assert.match(document.getElementById('launch-status')!.textContent!,/missing verified/);}finally{await app.dispose();}
});

test('Play timeout leaves one attempt uncertain and refuses another mutation', {timeout:5000},async()=>{
 const {Effect,Fiber,Result}=await import('effect');const {TestClock}=await import('effect/testing');
 const {makeLaunchWorkflow,launchBridgeLayer}=await import('./launch-workflow');let calls=0;
 await Effect.runPromise(Effect.scoped(Effect.gen(function*(){
 const service=yield* makeLaunchWorkflow;yield* service.inspect;
 const started=yield* Effect.result(service.play('attempt')).pipe(Effect.forkScoped);
 yield* TestClock.adjust('6 seconds');assert.equal(Result.isFailure(yield* Fiber.join(started)),true);
 assert.equal(calls,1);assert.equal((yield* service.snapshot).uncertain,true);assert.equal((yield* service.snapshot).pending,false);
 assert.equal(Result.isFailure(yield* Effect.result(service.play('another'))),true);assert.equal(calls,1);
 }).pipe(Effect.provide(launchBridgeLayer(async request=>{if(request.command==='inspect')return initial();calls++;return new Promise(()=>{});})),Effect.provide(TestClock.layer()))));
});

test('minimum-version Play rejection is distinct from transport uncertainty',async()=>{
 const status=initial();let plays=0;const {document,window}=parseHTML(html);
 const app=mountLaunch(document,async(_,{request}:any)=>{if(request.command==='play'){plays++;throw 'launcher_too_old';}return status;});
 try{await app.ready;await new Promise(resolve=>setImmediate(resolve));document.getElementById('launch')!.dispatchEvent(new window.Event('click'));await app.settled();await new Promise(resolve=>setImmediate(resolve));
 assert.equal(plays,1);assert.match(document.getElementById('launch-status')!.textContent!,/Update the launcher/);assert.deepEqual(status,initial());
 }finally{await app.dispose();}
});

test('native minimum gate survives successful status polls and blocks repeated Play',async()=>{
 let status={...initial(),launcher_update_required:true,installation_id:null as string|null};let plays=0;
 const {document,window}=parseHTML(html);
 const app=mountLaunch(document,async(_,{request}:any)=>{if(request.command==='play')plays++;return status;});
 try{await app.ready;for(let i=0;i<3;i++){await app.refresh();await new Promise(resolve=>setImmediate(resolve));
 assert.match(document.getElementById('launch-status')!.textContent!,/Update the launcher/);
 assert.equal((document.getElementById('launch') as HTMLButtonElement).disabled,true);
 document.getElementById('launch')!.dispatchEvent(new window.Event('click'));}
 assert.equal(plays,0);status=initial();await app.refresh();await new Promise(resolve=>setImmediate(resolve));
 assert.equal((document.getElementById('launch') as HTMLButtonElement).disabled,false);
 }finally{await app.dispose();}
});

test('failed status read stops automatic polling until explicit recheck succeeds', {timeout:5000},async()=>{
 const {document}=parseHTML(html);let reads=0;let unavailable=true;
 const app=mountLaunch(document,async()=>{reads++;if(unavailable)throw 'transport';return initial();});
 try {
  await app.ready;await new Promise(resolve=>setImmediate(resolve));
  assert.match(document.getElementById('launch-status')!.textContent!,/folder-access prompt/);
  await new Promise(resolve=>setTimeout(resolve,1100));
  assert.equal(reads,1,'a stalled native read is not multiplied by the polling timer');
  unavailable=false;await app.refresh();await new Promise(resolve=>setImmediate(resolve));
  assert.equal(reads,2);
  assert.equal((document.getElementById('launch') as HTMLButtonElement).disabled,false);
 } finally {await app.dispose();}
});
