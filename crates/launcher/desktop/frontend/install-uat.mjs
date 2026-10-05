// Sequential headless UI logic exercise. Native install IPC is a controlled fixture;
// native filesystem/worker behavior is covered separately by Rust tests.
import { readFile } from 'node:fs/promises';
import assert from 'node:assert/strict';
import { parseHTML } from 'linkedom';
import { mountInstall } from './.test-build/install-view.mjs';
const html=await readFile(new URL('./ui/index.html',import.meta.url),'utf8');
const id='7e438f46-9b99-450d-83b6-3c12436b403c';
let status={schema_version:1,install_supported:true,can_resume:true,can_reconcile:true,can_retry:false,uninstall:null,runtime_setup:null,progress:null,outcome:null,native:{schema_version:1,requires_reopen:false,
  preferences:{schema_version:1,revision:1,install_directory:'/fixture/app-data/Stargate Worlds',launcher_summary_consent:false},
  operation:{schema_version:1,revision:0,operation:null}}};
const calls=[];const requests=[];
const invoke=async(_command,{request})=>{
  calls.push(request.command);requests.push(request);
  if(request.command==='install')status={...status,progress:{phase:'download',current:32,total:64},native:{...status.native,
    operation:{schema_version:1,revision:1,operation:{id:request.operation_id,kind:'install',intent_digest:Array(32).fill(0),state:'running'}}}};
  if(request.command==='clean_failed'){
    assert.equal(request.confirmed,true);
    status={...status,can_retry:true};
  }
  if(request.command==='uninstall'){
    assert.equal(request.confirmed,true);assert.equal(request.installation_id,status.uninstall.installation_id);
    status={...status,uninstall:null,can_retry:true,outcome:null,progress:null,native:{...status.native,
      operation:{schema_version:1,revision:status.native.operation.revision+3,
        operation:{id:request.operation_id,kind:'uninstall',intent_digest:Array(32).fill(0),state:'succeeded'}}}};
  }
  if(request.command==='cancel')status={...status,native:{...status.native,operation:{...status.native.operation,revision:2,
    operation:{...status.native.operation.operation,state:'cancel_requested'}}}};
  return status;
};
const mount=(operationId=id)=>{const {document,window}=parseHTML(html);return {document,window,app:mountInstall(document,invoke,()=>operationId)};};
const settle=async app=>{await app.settled();await new Promise(resolve=>setImmediate(resolve));};
let ui=mount();await ui.app.ready;await settle(ui.app);
assert.equal(ui.document.getElementById('install').disabled,false);
assert.equal(status.native.preferences.install_directory,'/fixture/app-data/Stargate Worlds');
assert.equal(status.native.preferences.launcher_summary_consent,false);
ui.document.getElementById('install').dispatchEvent(new ui.window.Event('click'));await settle(ui.app);
assert.equal(status.native.operation.operation.state,'running');
assert.equal(ui.document.getElementById('cancel-install').hidden,false);
ui.document.getElementById('cancel-install').dispatchEvent(new ui.window.Event('click'));await settle(ui.app);
assert.match(ui.document.getElementById('install-status').textContent,/Waiting for the installer/);
assert.equal(status.native.operation.operation.state,'cancel_requested');
await ui.app.dispose();assert.equal(calls.filter(x=>x==='cancel').length,1);
// Reconnection sees native state; no invented cancellation or install replay.
ui=mount();await ui.app.ready;await settle(ui.app);
assert.equal(ui.document.getElementById('cancel-install').disabled,true);
assert.equal(calls.filter(x=>x==='install').length,1);
status={...status,outcome:'content_prepared',native:{...status.native,operation:{...status.native.operation,revision:3,
  operation:{...status.native.operation.operation,state:'succeeded'}}}};
await ui.app.refresh();await settle(ui.app);
assert.equal(ui.document.getElementById('install').textContent,'Content prepared');
assert.equal(ui.document.getElementById('install').disabled,true);
assert.equal(status.native.preferences.launcher_summary_consent,false);
await ui.app.dispose();
// Reopen failed native preparation states through the actual Effect decoder/view.
for(const [outcome,pattern] of [['rosetta_required',/Rosetta is required/],['runtime_unavailable',/compatibility could not be prepared/]]) {
 status={...status,outcome,native:{...status.native,operation:{...status.native.operation,revision:status.native.operation.revision+1,
  operation:{...status.native.operation.operation,state:'failed'}}}};
 ui=mount();await ui.app.ready;await settle(ui.app);
 assert.match(ui.document.getElementById('install-status').textContent,pattern);
 assert.equal(ui.document.getElementById('install').disabled,true);
 assert.equal(status.native.preferences.launcher_summary_consent,false);
 await ui.app.dispose();
}
status={...status,can_resume:false,can_reconcile:false,native:{...status.native,operation:{...status.native.operation,
 revision:status.native.operation.revision+1,operation:{...status.native.operation.operation,state:'reconciliation_required'}}}};
ui=mount();await ui.app.ready;await settle(ui.app);
assert.equal(ui.document.getElementById('resume-install').hidden,true);
ui.document.getElementById('inspect-install').dispatchEvent(new ui.window.Event('click'));await settle(ui.app);
assert.equal(calls.includes('reconcile'),false);assert.equal(calls.includes('resume'),false);
assert.equal(status.native.preferences.launcher_summary_consent,false);
await ui.app.dispose();
status={...status,can_reconcile:true};
ui=mount();await ui.app.ready;await settle(ui.app);
assert.equal(ui.document.getElementById('resume-install').hidden,true);
ui.document.getElementById('inspect-install').dispatchEvent(new ui.window.Event('click'));await settle(ui.app);
assert.equal(calls.filter(x=>x==='reconcile').length,1);
assert.equal(calls.includes('resume'),false);
assert.equal(status.native.operation.operation.state,'reconciliation_required');
assert.equal(status.native.preferences.launcher_summary_consent,false);
await ui.app.dispose();
status={...status,can_retry:false,native:{...status.native,operation:{...status.native.operation,
  operation:{...status.native.operation.operation,state:'failed'}}}};
ui=mount();await ui.app.ready;await settle(ui.app);
ui.document.getElementById('clean-failed-install').dispatchEvent(new ui.window.Event('click'));
assert.equal(ui.document.getElementById('cleanup-confirmation').hidden,false);
assert.equal(calls.includes('clean_failed'),false);
ui.document.getElementById('dismiss-cleanup').dispatchEvent(new ui.window.Event('click'));
assert.equal(ui.document.getElementById('cleanup-confirmation').hidden,true);
assert.equal(calls.includes('clean_failed'),false);
ui.document.getElementById('clean-failed-install').dispatchEvent(new ui.window.Event('click'));
ui.document.getElementById('confirm-cleanup').dispatchEvent(new ui.window.Event('click'));await settle(ui.app);
assert.equal(calls.filter(x=>x==='clean_failed').length,1);
assert.equal(ui.document.getElementById('install').disabled,false);
assert.equal(ui.document.getElementById('install').textContent,'Retry installation');
assert.equal(status.native.operation.operation.state,'failed');
assert.equal(status.native.preferences.launcher_summary_consent,false);
await ui.app.dispose();
const replacementId='877407cc-b79c-4ce5-8361-c0c8d3787463';
ui=mount(replacementId);await ui.app.ready;await settle(ui.app);
ui.document.getElementById('install').dispatchEvent(new ui.window.Event('click'));await settle(ui.app);
assert.equal(requests.filter(x=>x.command==='install').at(-1).operation_id,replacementId);
assert.equal(status.native.operation.operation.id,replacementId);
assert.equal(calls.filter(x=>x==='clean_failed').length,1);
await ui.app.dispose();
console.log('PASS: cleanup requires confirmation, dismissal preserves files, acknowledged cleanup enables explicit retry; enabled recovery explicitly reconciles without resume or completion inference; unsupported Wine recovery only inspects;  Rosetta/runtime failure decoding and reopened feedback;  install, progress, explicit cancel, reconnect without replay, completion wins cancellation, no Play/consent inference.');
console.log('NOT COVERED: native install IPC/filesystem, actual downloads/Wine, visual layout, OS dialogs, login/gameplay.');

status={...status,can_retry:false,uninstall:{installation_id:replacementId,directory:'/fixture/owned-install',recovery:false},
 native:{...status.native,operation:{...status.native.operation,revision:status.native.operation.revision+1,
 operation:{...status.native.operation.operation,state:'succeeded'}}}};
ui=mount('7a753523-bde2-4897-af71-6e1d73196681');await ui.app.ready;await settle(ui.app);
const click=element=>ui.document.getElementById(element).dispatchEvent(new ui.window.Event('click'));
click('uninstall');assert.equal(ui.document.getElementById('uninstall-confirmation').hidden,false);
assert.equal(ui.document.getElementById('uninstall-directory').textContent,'/fixture/owned-install');
assert.equal(calls.includes('uninstall'),false);click('dismiss-uninstall');
assert.equal(ui.document.getElementById('uninstall-confirmation').hidden,true);
click('uninstall');click('confirm-uninstall');click('confirm-uninstall');await settle(ui.app);
assert.equal(calls.filter(x=>x==='uninstall').length,1);
assert.equal(ui.document.getElementById('install').disabled,false);
assert.equal(ui.document.getElementById('install').textContent,'Install Stargate Worlds');
assert.equal(status.native.preferences.launcher_summary_consent,false);
await ui.app.dispose();
console.log('PASS: uninstall requires explicit confirmation of owned folder, dismissal sends nothing, double-click dispatches once, native acknowledgement enables reinstall, consent unchanged. Native disk persistence is separately covered by Rust host tests; this pass uses fixture IPC, not real deletion or visual UAT.');

for (const phase of ['running','succeeded','reconciliation_required']) {
 const before=calls.length;
 status={...status,can_retry:false,can_resume:false,can_reconcile:false,uninstall:null,
  native:{...status.native,operation:{schema_version:1,revision:status.native.operation.revision+1,
   operation:{id,kind:'prepare_runtime',state:phase,intent_digest:Array(32).fill(0)}}}};
 ui=mount();await ui.app.ready;await settle(ui.app);
 assert.equal(ui.document.getElementById('install').disabled,true);
 assert.equal(ui.document.getElementById('cancel-install').hidden,phase!=='running');
 assert.match(ui.document.getElementById('install-status').textContent,
  phase==='succeeded'?/Graphics and Play still need validation/:/compatibility|Compatibility/);
 ui.document.getElementById('install').dispatchEvent(new ui.window.Event('click'));
 await settle(ui.app);assert.ok(calls.slice(before).every(command=>command==='inspect'));
 assert.equal(status.native.preferences.launcher_summary_consent,false);
 await ui.app.dispose();
}
console.log('PASS: Effect decodes runtime setup states; running setup exposes cancellation; success/recovery never enable Play or reinstall; consent preserved. Fixture IPC only; native durable runtime state is covered separately by Rust tests. No visual UAT.');


// Exercise the actual Effect/view transition through controlled durable-state replies.
const setupId='5c1d6e5a-8c9d-442f-9740-0ca5d62568ac';
let journey={...status,runtime_setup:null,can_resume:false,can_reconcile:false,can_retry:false,
 native:{...status.native,operation:{schema_version:1,revision:0,operation:null}}};
const journeyCalls=[];let next=0;
const journeyInvoke=async(_command,{request})=>{
 journeyCalls.push(request);
 if(request.command==='install')journey={...journey,runtime_setup:id,outcome:'content_prepared',native:{...journey.native,
  operation:{schema_version:1,revision:3,operation:{id,kind:'install',state:'succeeded',intent_digest:Array(32).fill(0)}}}};
 if(request.command==='prepare_runtime'){
  assert.equal(request.installation_id,id);assert.equal(request.operation_revision,3);
  journey={...journey,runtime_setup:null,outcome:null,native:{...journey.native,
   operation:{schema_version:1,revision:4,operation:{id:setupId,kind:'prepare_runtime',state:'running',intent_digest:Array(32).fill(0)}}}};
 }
 if(request.command==='reconcile'){
  assert.equal(request.operation_id,setupId);assert.equal(request.operation_revision,7);
  journey={...journey,can_reconcile:false,native:{...journey.native,operation:{schema_version:1,revision:8,
   operation:{...journey.native.operation.operation,state:'succeeded'}}}};
 }
 return journey;
};
let screen=parseHTML(html);let journeyApp=mountInstall(screen.document,journeyInvoke,()=>next++===0?id:setupId);
await journeyApp.ready;await settle(journeyApp);
screen.document.getElementById('install').dispatchEvent(new screen.window.Event('click'));
for(let n=0;n<20&&!journeyCalls.some(c=>c.command==='prepare_runtime');n++)await settle(journeyApp);
await settle(journeyApp);
assert.equal(journeyCalls.filter(c=>c.command==='install').length,1);
assert.equal(journeyCalls.filter(c=>c.command==='prepare_runtime').length,1);
assert.equal(screen.document.getElementById('install').textContent,'Checking compatibility…');
assert.equal(screen.document.getElementById('cancel-install').hidden,false);
await journeyApp.dispose();
// A reopened successful-content state is a separate user intent, not automatic replay.
journey={...journey,runtime_setup:id,native:{...journey.native,operation:{schema_version:1,revision:3,
 operation:{id,kind:'install',state:'succeeded',intent_digest:Array(32).fill(0)}}}};
const beforeJourney=journeyCalls.length;screen=parseHTML(html);
journeyApp=mountInstall(screen.document,journeyInvoke,()=>setupId);
await journeyApp.ready;await settle(journeyApp);
assert.equal(screen.document.getElementById('install').textContent,'Continue installation');
assert.ok(journeyCalls.slice(beforeJourney).every(c=>c.command==='inspect'));
screen.document.getElementById('install').dispatchEvent(new screen.window.Event('click'));await settle(journeyApp);
assert.equal(journeyCalls.filter(c=>c.command==='prepare_runtime').length,2);
assert.equal(journey.native.preferences.launcher_summary_consent,false);
await journeyApp.dispose();
console.log('PASS: one Install intent sequences content into native setup once; reopen requires explicit Continue; setup exposes cancellation; consent unchanged. This REPL-style pass uses fixture native state, not real Wine, disk persistence, visual layout, login or gameplay.');


journey={...journey,can_reconcile:true,runtime_setup:null,native:{...journey.native,operation:{schema_version:1,revision:7,
 operation:{id:setupId,kind:'prepare_runtime',state:'reconciliation_required',intent_digest:Array(32).fill(0)}}}};
const beforeRecovery=journeyCalls.length;screen=parseHTML(html);
journeyApp=mountInstall(screen.document,journeyInvoke,()=>setupId);
await journeyApp.ready;await settle(journeyApp);
assert.equal(screen.document.getElementById('inspect-install').textContent,'Recover compatibility setup');
assert.ok(journeyCalls.slice(beforeRecovery).every(c=>c.command==='inspect'));
screen.document.getElementById('inspect-install').dispatchEvent(new screen.window.Event('click'));
await settle(journeyApp);
assert.equal(journeyCalls.slice(beforeRecovery).filter(c=>c.command==='reconcile').length,1);
assert.ok(journeyCalls.slice(beforeRecovery).every(c=>c.command!=='prepare_runtime'));
assert.equal(screen.document.getElementById('install').textContent,'Compatibility checked');
assert.equal(screen.document.getElementById('install').disabled,true);
assert.equal(journey.native.preferences.launcher_summary_consent,false);
await journeyApp.dispose();
console.log('PASS: observed compatibility recovery requires an explicit click, binds current operation/revision, never replays setup and never enables Play. Controlled IPC only; native stop/wait and persistence are tested separately.');
