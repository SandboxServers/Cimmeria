import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parseHTML} from 'linkedom';
import {launchText,mountLaunch} from './launch-view';
import {jobErrorFailure,LaunchBridge,launchBridgeLayer} from './launch-workflow';
import {FailureCode} from './failure-codes';
import {failureText} from './failure-copy';
import {mountDesktop} from './composition';
import {mountPreflight} from './preflight-view';
import {preflightWorkflow} from './preflight-workflow';
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
test('an install operation names the busy launcher, and an uncoded error keeps the generic line',()=>{
 assert.equal(launchText({status:withOperation('install','running'),pending:false,uncertain:false,error:null} as any),'The launcher is busy (install). Play is available when it finishes.');
 assert.match(launchText({status:withOperation('install','running'),pending:false,uncertain:false,error:'transport'} as any),/Resolve any folder-access prompt/);
});
const text=(status:any,error:string|null=null)=>launchText({status,pending:false,uncertain:false,error});
test('Play names what blocks it, per operation kind and state',()=>{
 const lines:[string,string,string][]=[
  ['repair','reconciliation_required','An earlier repair was interrupted. Abandon it to play, or recover it in Settings.'],
  ['repair','running','A repair is running. Play is available when it finishes.'],
  ['repair','starting','A repair is running. Play is available when it finishes.'],
  ['install','reconciliation_required','An earlier install was interrupted. Resume or abandon it in the install panel.'],
  ['install','running','The launcher is busy (install). Play is available when it finishes.'],
  ['update','running','The launcher is busy (game update). Play is available when it finishes.'],
  ['uninstall','running','The launcher is busy (uninstall). Play is available when it finishes.'],
  ['prepare_runtime','cancel_requested','The launcher is busy (compatibility setup). Play is available when it finishes.'],
  ['update','reconciliation_required','An earlier game update was interrupted. Recover or abandon it in Settings.'],
  ['launch','running','Another operation owns the game. Wait for it to finish.'],
  ['uninstall','reconciliation_required','Another operation owns the game. Wait for it to finish.'],
  ['adopt','reconciliation_required','Finish or abandon the interrupted adoption in the adoption panel before you play.'],
 ];
 for(const [kind,state,line] of lines)assert.equal(text(withOperation(kind,state)),line,`${kind} ${state}`);
 assert.equal(text({...initial(),native:{...initial().native,requires_reopen:true}}),'The launcher needs to restart to check its saved state. Close and reopen it.');
 assert.equal(text(withOperation('mystery','running')),'Another operation owns the game. Wait for it to finish.');
});
test('a coded Play failure shows its message and fix; no code keeps the generic lines',()=>{
 const rosetta=failureText('rosetta_missing');
 assert.equal(text({...initial(),observation:{phase:'not_started'},failure:'rosetta_missing'}),`${rosetta.message}\n${rosetta.fix}`);
 // Every C-PD1 code renders through failureText, including codes Play has no special case for.
 for(const code of FailureCode.literals){const copy=failureText(code);
  assert.equal(text({...initial(),observation:{phase:'not_started'},failure:code}),`${copy.message}\n${copy.fix}`,code);}
 const exited=failureText('game_exited_nonzero');
 assert.equal(text({...initial(),observation:{phase:'process_exited',host_pid:1,guest_pid:2,code:3,early:true}}),`${exited.message}\n${exited.fix}`);
 assert.equal(text({...initial(),observation:{phase:'process_exited',host_pid:1,guest_pid:2,code:0,early:false}}),'Game exited (code 0).');
 assert.match(text({...initial(),observation:{phase:'not_started'}}),/^Game could not start\./);
 const io=failureText('local_io');
 assert.equal(text(initial(),'local_io'),`${io.message}\n${io.fix}`);
 assert.match(text(initial(),'transport'),/Resolve any folder-access prompt/);
});
test('every native JobError code keeps its C-PD1 failure code; anything else is transport',async()=>{
 const fixture=JSON.parse(readFileSync('src/failure-codes.json','utf8')) as {job_errors:Record<string,string>};
 assert.deepEqual({...jobErrorFailure},fixture.job_errors);
 const {Effect,Result}=await import('effect');
 const code=async(thrown:unknown)=>{const result=await Effect.runPromise(Effect.result(Effect.flatMap(LaunchBridge,b=>b.invoke({command:'inspect',schema_version:1})).pipe(Effect.provide(launchBridgeLayer(async()=>{throw thrown;})))));
  assert.ok(Result.isFailure(result));return result.failure.code;};
 assert.equal(await code('io'),'local_io');assert.equal(await code('busy'),'operation_busy');assert.equal(await code('unsupported_schema'),'launcher_too_old');
 assert.equal(await code('toString'),'transport');assert.equal(await code(new Error('x')),'transport');assert.equal(await code('mystery'),'transport');
});
test('a status with failure rosetta_missing shows the Rosetta message and fix on Play',async()=>{
 const status={...initial(),observation:{phase:'not_started'},failure:'rosetta_missing'};const {document}=parseHTML(html);
 const app=mountLaunch(document,async()=>status,()=>{},()=>'attempt');
 try{await app.ready;await new Promise(resolve=>setImmediate(resolve));
  const {message,fix}=failureText('rosetta_missing');
  assert.equal(document.getElementById('launch-status')!.textContent,`${message}\n${fix}`);
 }finally{await app.dispose();}
});
const installStatus=(native:any,recovery:boolean)=>({schema_version:1,native,install_supported:true,can_resume:false,can_reconcile:false,can_retry:false,uninstall:null,runtime_setup:null,progress:null,outcome:null,repair:{directory:'/fixture',target:null,recovery,cleanup:false}});
function desktop(kind:string,state:string,recovery=true){
 const {document,window}=parseHTML(html);const launch=withOperation(kind,state);const calls:any[]=[];
 const app=mountDesktop(document,async(command,args:any)=>{calls.push([command,args?.request?.command]);
  if(command==='launch_command')return launch;
  if(command==='install_command')return installStatus(launch.native,recovery);
  return launch.native;});
 const get=(id:string)=>document.getElementById(id)!;
 const settle=async()=>{await Promise.all([app.play.ready,app.installation.ready]);await app.play.settled();await app.installation.settled();await new Promise(resolve=>setTimeout(resolve,10));};
 return {app,get,calls,settle,click:(id:string)=>get(id).dispatchEvent(new window.Event('click'))};
}
test('the inline Abandon shows only for an interrupted repair, and its first press opens the Settings confirmation',async()=>{
 for(const [kind,state] of [['repair','running'],['install','reconciliation_required'],['update','reconciliation_required']]){
  const ui=desktop(kind,state);
  try{await ui.settle();assert.equal(ui.get('launch-abandon-repair').hidden,true,`${kind} ${state}`);}finally{await ui.app.dispose();}
 }
 const ui=desktop('repair','reconciliation_required');
 try{await ui.settle();
  assert.equal(ui.get('launch-abandon-repair').hidden,false);
  assert.equal(ui.get('gear').hidden,true);assert.equal(ui.get('repair-confirmation').hidden,true);
  ui.click('launch-abandon-repair');
  assert.equal(ui.get('gear').hidden,false);assert.equal(ui.get('play').hidden,true);assert.equal(ui.get('settings').getAttribute('aria-expanded'),'true');
  assert.equal(ui.get('repair-confirmation').hidden,false);
  assert.match(ui.get('repair-consequences').textContent!,/^Stop only interrupted preparation/);
  assert.equal(ui.calls.some(([,request])=>request==='abandon_repair'),false,'the press only asks; Confirm sends');
 }finally{await ui.app.dispose();}
 const declined=desktop('repair','reconciliation_required',false);
 try{await declined.settle();declined.click('launch-abandon-repair');
  assert.equal(declined.get('gear').hidden,false,'Settings opens even when native offers no abandon');
  assert.equal(declined.get('repair-confirmation').hidden,true);
 }finally{await declined.app.dispose();}
});
test('Play hides Recheck while an adoption is open',async()=>{
 const status=withOperation('adopt','reconciliation_required');const {document}=parseHTML(html);
 const app=mountLaunch(document,async()=>status,()=>{},()=>'attempt');
 try{await app.ready;await new Promise(resolve=>setImmediate(resolve));
  assert.equal((document.getElementById('inspect-launch') as HTMLButtonElement).hidden,true);
  assert.equal(document.getElementById('launch-status')!.textContent,'Finish or abandon the interrupted adoption in the adoption panel before you play.');
 }finally{await app.dispose();}
});
// PD-12: Play runs preflight first.
const pfCheck=(over:Record<string,unknown>={})=>({id:'rosetta',status:'missing',action:'auto_install',failure:'rosetta_missing',blocking:true,bytes_needed:null,bytes_free:null,download_bytes:null,...over});
const pfReport=(checks:ReturnType<typeof pfCheck>[],operation='play')=>({schema_version:1,operation,checks,ready:!checks.some(c=>c.blocking)});
const pfWarning=pfCheck({id:'app_location',status:'warning',action:'manual',failure:'app_translocated',blocking:false});
function gated(preflight:(request:any)=>Promise<unknown>){
 const {document,window}=parseHTML(html);const sent:string[]=[];
 const invoke=async(command:string,args?:any)=>{
  if(command==='preflight_command'){sent.push(`${args.request.command}:${args.request.operation}${args.request.check?`:${args.request.check}`:''}`);return preflight(args.request);}
  if(args?.request?.command==='play')sent.push('play');
  return initial();};
 const panel=mountPreflight(document as unknown as Document,preflightWorkflow(invoke));
 const app=mountLaunch(document,invoke,()=>{},()=>'attempt',false,()=>{},panel.gate);
 const get=(id:string)=>document.getElementById(id) as HTMLButtonElement;
 const press=(id:string)=>get(id).dispatchEvent(new window.Event('click'));
 const settle=async()=>{await panel.settled();await app.settled();await new Promise(resolve=>setImmediate(resolve));};
 const plays=()=>sent.filter(s=>s==='play').length;
 return {app,panel,get,press,settle,sent,plays,ready:async()=>{await app.ready;await new Promise(resolve=>setImmediate(resolve));},
  dispose:async()=>{await panel.dispose();await app.dispose();}};
}
test('Play with a blocking preflight report sends no launch command, shows the panel and says Play needs attention',async()=>{
 let answer!:(value:unknown)=>void;const ui=gated(()=>new Promise(resolve=>{answer=resolve;}));
 try{await ui.ready();
  ui.press('launch');ui.press('launch');
  assert.equal(ui.get('launch').textContent,'Checking…','the first press shows it is working');assert.equal(ui.get('launch').disabled,true);
  assert.equal(ui.get('launch-status').textContent,'Checking what Play needs…');
  assert.equal(ui.get('preflight-panel').hidden,false);
  answer(pfReport([pfCheck()]));await ui.settle();
  assert.deepEqual(ui.sent,['inspect:play'],'one inspect, no launch');assert.equal(ui.plays(),0);
  assert.equal(ui.get('preflight-panel').hidden,false);assert.equal(ui.get('preflight-continue').hidden,true);
  assert.equal(ui.get('launch-status').textContent,'Play needs attention: see below.');
  assert.equal(ui.get('launch').textContent,'Play');assert.equal(ui.get('launch').disabled,false);
 }finally{await ui.dispose();}
});
test('Play with a ready preflight report sends exactly one launch command',async()=>{
 const ui=gated(async()=>pfReport([pfCheck({status:'ok',action:'none',failure:null,blocking:false})]));
 try{await ui.ready();ui.press('launch');await ui.settle();
  assert.equal(ui.plays(),1);assert.equal(ui.sent[0],'inspect:play');assert.equal(ui.get('preflight-panel').hidden,true);
 }finally{await ui.dispose();}
});
test('Play with a warning report waits for Continue, which launches once',async()=>{
 const ui=gated(async()=>pfReport([pfWarning]));
 try{await ui.ready();ui.press('launch');await ui.settle();
  assert.equal(ui.plays(),0);assert.equal(ui.get('preflight-continue').hidden,false);
  assert.equal(ui.get('launch-status').textContent,'Play needs attention: see below.');
  ui.press('preflight-continue');ui.press('preflight-continue');await ui.settle();
  assert.equal(ui.plays(),1);assert.equal(ui.get('preflight-panel').hidden,true);
 }finally{await ui.dispose();}
});
test('a preflight transport error never blocks Play silently: it shows the failure and Continue anyway',async()=>{
 const ui=gated(async()=>{throw new Error('ipc down');});
 try{await ui.ready();ui.press('launch');await ui.settle();
  assert.equal(ui.plays(),0);
  const {message,fix}=failureText('unknown');
  assert.equal(ui.get('preflight-summary').textContent,`${message}\n${fix}`);
  assert.equal(ui.get('preflight-continue').hidden,false);assert.equal(ui.get('preflight-continue').textContent,'Continue anyway');
  ui.press('preflight-continue');await ui.settle();assert.equal(ui.plays(),1);
 }finally{await ui.dispose();}
});
test('a refused preflight shows the mapped JobError copy',async()=>{
 const ui=gated(async()=>{throw 'busy';});
 try{await ui.ready();ui.press('launch');await ui.settle();
  const {message,fix}=failureText('operation_busy');
  assert.equal(ui.get('preflight-summary').textContent,`${message}\n${fix}`);assert.equal(ui.get('preflight-continue').textContent,'Continue anyway');
 }finally{await ui.dispose();}
});
test('after a dependency install the panel says Ready. Press Play again. and does not launch by itself',async()=>{
 let installed=false;const ok=()=>pfReport([pfCheck({status:'ok',action:'none',failure:null,blocking:false})]);
 const ui=gated(async request=>{if(request.command==='install')installed=true;return installed?ok():pfReport([pfCheck()]);});
 try{await ui.ready();ui.press('launch');await ui.settle();
  ui.press('preflight-install-rosetta');assert.equal(ui.get('preflight-install-rosetta').textContent,'Installing…');await ui.settle();
  assert.deepEqual(ui.sent,['inspect:play','install:play:rosetta']);assert.equal(ui.plays(),0);
  assert.equal(ui.get('preflight-status').textContent,'Ready. Press Play again.');
  ui.press('launch');await ui.settle();assert.equal(ui.plays(),1,'the next press re-checks and launches');
 }finally{await ui.dispose();}
});
