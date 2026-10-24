import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parseHTML} from 'linkedom';
import {launchText,mountLaunch} from './launch-view';
const html=readFileSync('ui/index.html','utf8');
const initial=()=>({schema_version:1,setup_complete:true,resources_available:true,launcher_update_required:false,patches_incomplete:0,ops_skipped:0,installation_id:'owner',observation:null as unknown,failure:null as unknown,native:{schema_version:1,requires_reopen:false,adopted_login_servers:null,display_modes:['windowed' as const],launcher:{version:'0.1.0',tag:null},preferences:{schema_version:2,revision:1,install_directory:'/fixture',launcher_summary_consent:false,login_servers:[],client_patches_enabled:true,display:{mode:'windowed' as const,width:1024,height:768}},operation:{schema_version:1,revision:0,operation:null as unknown}}});
test('ready content hides setup recheck across launch, update gate and errors',async()=>{
 let status=initial(),fail=false;const observed:boolean[]=[];
 const {document}=parseHTML(html);
 const app=mountLaunch(document,async()=>{if(fail)throw 'transport';return status;},undefined,undefined,true,ready=>observed.push(ready));
 const check=()=>document.getElementById('inspect-launch')!.hidden;
 const settle=async()=>{await app.refresh();await new Promise(resolve=>setImmediate(resolve));};
 try{
  await app.ready;await settle();assert.equal(check(),true);assert.equal(observed.at(-1),true);
  assert.equal((document.getElementById('launch') as HTMLButtonElement).disabled,true,'updater still owns Play');
  status={...status,installation_id:null as any,native:{...status.native,operation:{...status.native.operation,revision:1,operation:{id:'saved',kind:'launch',state:'running',intent_digest:Array(32).fill(0)}}}};
  await settle();assert.equal(check(),true,'a running game does not resurrect setup');
  status={...status,observation:{phase:'unknown'}};await settle();assert.equal(check(),false,'unknown game state retains recheck');
  fail=true;await settle();assert.equal(check(),false);assert.equal(observed.at(-1),false);
  fail=false;status={...initial(),installation_id:null as any,setup_complete:false};await settle();assert.equal(check(),false);
  status={...initial(),setup_complete:false,resources_available:false};await settle();assert.equal(check(),false);
  status={...initial(),native:{...initial().native,requires_reopen:true}};await settle();assert.equal(check(),false);
 }finally{await app.dispose();}
});
test('Settings inspection gives first-press feedback and sends one read while ready setup stays hidden',async()=>{
 const {document,window}=parseHTML(html);let reads=0;let finish!:()=>void;
 const gate=new Promise<void>(resolve=>{finish=resolve;});
 const app=mountLaunch(document,async()=>{reads++;if(reads>1)await gate;return initial();});
 try{
  await app.ready;await new Promise(resolve=>setImmediate(resolve));
  document.getElementById('gear')!.hidden=false;
  const button=document.getElementById('inspect-launch-settings') as HTMLButtonElement;
  assert.equal(button.closest<HTMLElement>('#gear')?.hidden,false);
  assert.equal(document.getElementById('inspect-launch')!.hidden,true);
  button.dispatchEvent(new window.Event('click'));button.dispatchEvent(new window.Event('click'));
  assert.equal(document.getElementById('launch-settings-status')!.textContent,'Checking Play status…');
  assert.equal(button.disabled,true);assert.equal(button.getAttribute('aria-busy'),'true');
  await new Promise(resolve=>setImmediate(resolve));assert.equal(reads,2);
  finish();await app.settled();await new Promise(resolve=>setImmediate(resolve));
  assert.equal(document.getElementById('launch-settings-status')!.textContent,'Ready to Play.');
 }finally{finish?.();await app.dispose();}
});
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
 const {document}=parseHTML(html);const status={...initial(),setup_complete:false,resources_available:false,installation_id:null};const app=mountLaunch(document,async()=>status);
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

test('a refused Play is reported to telemetry',async()=>{
 const uiCalls:unknown[]=[];
 Object.assign(globalThis,{window:{__TAURI__:{core:{invoke:async(command:string,args:unknown)=>{uiCalls.push([command,args]);}}}}});
 try{
  const status=initial();const {document,window}=parseHTML(html);
  const app=mountLaunch(document,async(_,{request}:any)=>{if(request.command==='play')throw 'busy';return status;});
  try{await app.ready;await new Promise(resolve=>setImmediate(resolve));document.getElementById('launch')!.dispatchEvent(new window.Event('click'));await app.settled();await new Promise(resolve=>setImmediate(resolve));
   assert.deepEqual(uiCalls,[['ui_event',{area:'launch',event:'invoke_failed',code:'busy',level:'warn'}]]);
  }finally{await app.dispose();}
 }finally{Reflect.deleteProperty(globalThis,'window');}
});

test('a minimum-version Play refusal is not reported to telemetry',async()=>{
 const uiCalls:unknown[]=[];
 Object.assign(globalThis,{window:{__TAURI__:{core:{invoke:async(command:string,args:unknown)=>{uiCalls.push([command,args]);}}}}});
 try{
  const status=initial();let plays=0;const {document,window}=parseHTML(html);
  const app=mountLaunch(document,async(_,{request}:any)=>{if(request.command==='play'){plays++;throw 'launcher_too_old';}return status;});
  try{await app.ready;await new Promise(resolve=>setImmediate(resolve));document.getElementById('launch')!.dispatchEvent(new window.Event('click'));await app.settled();await new Promise(resolve=>setImmediate(resolve));
   assert.equal(plays,1);
   assert.deepEqual(uiCalls,[]);
  }finally{await app.dispose();}
 }finally{Reflect.deleteProperty(globalThis,'window');}
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

test('background inspection keeps Play steady and accepts one click while the read is pending', {timeout:5000},async()=>{
 const {document,window}=parseHTML(html);let reads=0;let plays=0;
 let release!:()=>void;const held=new Promise<void>(resolve=>{release=resolve;});
 const app=mountLaunch(document,async(_,{request}:any)=>{
  if(request.command==='inspect'&&++reads===2)await held;
  if(request.command==='play')plays++;
  return initial();
 },()=>{},()=> 'single-attempt');
 try {
  await app.ready;await new Promise(resolve=>setImmediate(resolve));
  const button=document.getElementById('launch') as HTMLButtonElement;
  const read=app.refresh();await new Promise(resolve=>setImmediate(resolve));
  assert.equal(button.disabled,false);
  assert.equal(button.getAttribute('aria-busy'),'false');
  assert.equal((document.getElementById('inspect-launch') as HTMLButtonElement).disabled,false, 'background reads keep Recheck stable too');
  assert.equal(button.textContent,'Play');
  button.dispatchEvent(new window.Event('click'));button.dispatchEvent(new window.Event('click'));
  assert.equal(button.disabled,true);
  assert.equal(button.textContent,'Starting…');
  release();await read;await app.settled();await new Promise(resolve=>setImmediate(resolve));
  assert.equal(plays,1);
  assert.equal(button.disabled,false);
  assert.equal(button.textContent,'Play');
 } finally {release();await app.dispose();}
});
test('an install with skipped patch changes names them in the main view, and Play stays enabled',async()=>{
 const status=initial();status.patches_incomplete=1;status.ops_skipped=3;
 const {document}=parseHTML(html);
 const app=mountLaunch(document,async()=>status,()=>{},()=>'attempt');
 try{await app.ready;await new Promise(resolve=>setImmediate(resolve));
  assert.match(document.getElementById('launch-status')!.textContent!,/3 patch changes could not be applied\. If the game crashes or misbehaves, use Repair \(downloads the full game, 4\.1 GB\)\./);
  assert.equal((document.getElementById('launch') as HTMLButtonElement).disabled,false);
 }finally{await app.dispose();}
});
test('an install with no skipped patch changes reads exactly Ready to Play.',()=>{
 const status={...initial(),patches_incomplete:0,ops_skipped:3} as any;
 assert.equal(launchText({status,pending:false,uncertain:false,error:null}),'Ready to Play.');
});
test('without an installation the skipped-change hint is not shown',()=>{
 const status={...initial(),installation_id:null,patches_incomplete:1,ops_skipped:3} as any;
 assert.equal(launchText({status,pending:false,uncertain:false,error:null}),'Finish installation and compatibility checks to enable Play.');
});
test('one skipped patch change reads in the singular, several in the plural',()=>{
 const text=(n:number)=>launchText({status:{...initial(),patches_incomplete:1,ops_skipped:n} as any,pending:false,uncertain:false,error:null});
 assert.match(text(1),/^Ready to Play\. 1 patch change could not be applied\. /);
 assert.match(text(2),/^Ready to Play\. 2 patch changes could not be applied\. /);
});
test('a Play that did not start points at an edited login server list',()=>{
 const status={...initial(),observation:{phase:'not_started'}} as any;
 assert.match(launchText({status,pending:false,uncertain:false,error:null}),/If you edited the login servers, check them under Login servers\./);
});
const withOperation=(kind:string,state:string)=>({...initial(),native:{...initial().native,operation:{schema_version:1,revision:1,operation:{id:'op',kind,state,intent_digest:Array(32).fill(0)}}}}) as any;
test('an interrupted adoption gives Play one adoption line in place of the owner and error lines',()=>{
 const play=(status:any,error:string|null=null)=>({status,pending:false,uncertain:false,error}) as any;
 assert.equal(launchText(play(withOperation('adopt','reconciliation_required'))),'Finish or abandon the interrupted adoption in the adoption panel before you play.');
 assert.equal(launchText(play(withOperation('adopt','running'))),'An adoption review is open. Finish or cancel it in the adoption panel.');
 assert.equal(launchText(play(withOperation('adopt','running'),'io')),'An adoption review is open. Finish or cancel it in the adoption panel.');
});
test('an install operation keeps the generic owner line and error line',()=>{
 assert.equal(launchText({status:withOperation('install','running'),pending:false,uncertain:false,error:null} as any),'Another operation owns the game. Wait for it to finish.');
 assert.match(launchText({status:withOperation('install','running'),pending:false,uncertain:false,error:'io'} as any),/Resolve any folder-access prompt/);
});
test('Play hides Recheck while an adoption is open',async()=>{
 const status=withOperation('adopt','reconciliation_required');const {document}=parseHTML(html);
 const app=mountLaunch(document,async()=>status,()=>{},()=>'attempt');
 try{await app.ready;await new Promise(resolve=>setImmediate(resolve));
  assert.equal((document.getElementById('inspect-launch') as HTMLButtonElement).hidden,true);
  assert.equal(document.getElementById('launch-status')!.textContent,'Finish or abandon the interrupted adoption in the adoption panel before you play.');
 }finally{await app.dispose();}
});
