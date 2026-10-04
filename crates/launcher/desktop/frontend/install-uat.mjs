// Sequential headless UI logic exercise. Native install IPC is a controlled fixture;
// native filesystem/worker behavior is covered separately by Rust tests.
import { readFile } from 'node:fs/promises';
import assert from 'node:assert/strict';
import { parseHTML } from 'linkedom';
import { mountInstall } from './.test-build/install-view.mjs';
const html=await readFile(new URL('./ui/index.html',import.meta.url),'utf8');
const id='7e438f46-9b99-450d-83b6-3c12436b403c';
let status={schema_version:1,install_supported:true,can_resume:true,can_reconcile:true,progress:null,outcome:null,native:{schema_version:1,requires_reopen:false,
  preferences:{schema_version:1,revision:1,install_directory:'/fixture/owned',launcher_summary_consent:false},
  operation:{schema_version:1,revision:0,operation:null}}};
const calls=[];
const invoke=async(_command,{request})=>{
  calls.push(request.command);
  if(request.command==='install')status={...status,progress:{phase:'download',current:32,total:64},native:{...status.native,
    operation:{schema_version:1,revision:1,operation:{id,kind:'install',intent_digest:Array(32).fill(0),state:'running'}}}};
  if(request.command==='cancel')status={...status,native:{...status.native,operation:{...status.native.operation,revision:2,
    operation:{...status.native.operation.operation,state:'cancel_requested'}}}};
  return status;
};
const mount=()=>{const {document,window}=parseHTML(html);return {document,window,app:mountInstall(document,invoke,()=>id)};};
const settle=async app=>{await app.settled();await new Promise(resolve=>setImmediate(resolve));};
let ui=mount();await ui.app.ready;await settle(ui.app);
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
console.log('PASS: unsupported Wine recovery only inspects;  Rosetta/runtime failure decoding and reopened feedback;  install, progress, explicit cancel, reconnect without replay, completion wins cancellation, no Play/consent inference.');
console.log('NOT COVERED: native install IPC/filesystem, actual downloads/Wine, visual layout, OS dialogs, login/gameplay.');
