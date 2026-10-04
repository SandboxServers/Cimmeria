// Actual Effect/view with native DesktopState persistence. Lifecycle inputs are
// inert fixture observations; no Windows process, injection or Wine is exercised.
import {spawn} from 'node:child_process';
import {createInterface} from 'node:readline';
import {readFile} from 'node:fs/promises';
import {randomUUID} from 'node:crypto';
import assert from 'node:assert/strict';
import {parseHTML} from 'linkedom';
import {mountLaunch} from './.test-build/launch-view.mjs';
const child=spawn(process.env.LAUNCH_UAT_BINARY,['--ignored','--nocapture','launch_uat_bridge'],{stdio:['pipe','pipe','inherit']});
const pending=[];
createInterface({input:child.stdout}).on('line',line=>{if(line.startsWith('LAUNCH_UAT ')){const {resolve,reject}=pending.shift();const result=JSON.parse(line.slice(11));result.error?reject(result.error):resolve(result.ok);}});
const send=request=>new Promise((resolve,reject)=>{pending.push({resolve,reject});child.stdin.write(JSON.stringify(request)+'\n');});
const calls=[];let lose=false;
const invoke=async(_,{request})=>{calls.push(request.command);const result=await send(request);if(lose&&request.command==='play'){lose=false;throw 'transport';}return result;};
const html=await readFile(new URL('./ui/index.html',import.meta.url),'utf8');
const mount=()=>{const {document,window}=parseHTML(html);return {document,app:mountLaunch(document,invoke,()=>{},randomUUID),click:()=>document.getElementById('launch').dispatchEvent(new window.Event('click'))};};
const settle=async ui=>{await ui.app.settled();await new Promise(resolve=>setImmediate(resolve));};
let ui=mount();
try {
 await ui.app.ready;await settle(ui);const before=await send({command:'inspect',schema_version:1});assert.ok(before.installation_id);
 await send({command:'too_old'});
 for(let i=0;i<3;i++){await ui.app.refresh();await settle(ui);assert.match(ui.document.getElementById('launch-status').textContent,/Update the launcher/);assert.equal(ui.document.getElementById('launch').disabled,true);ui.click();}
 assert.equal(calls.filter(x=>x==='play').length,0);
 const blocked=await send({command:'inspect',schema_version:1});assert.deepEqual(blocked.native.preferences,before.native.preferences);assert.deepEqual(blocked.native.operation,before.native.operation);
 await send({command:'development'});await ui.app.refresh();await settle(ui);assert.equal(ui.document.getElementById('launch').disabled,false);
 ui.click();ui.click();assert.match(ui.document.getElementById('launch-status').textContent,/Starting/);await settle(ui);
 assert.equal(calls.filter(x=>x==='play').length,1);assert.equal(ui.document.getElementById('launch').disabled,true);assert.match(ui.document.getElementById('launch-status').textContent,/not verified/);
 await send({command:'exit'});await ui.app.refresh();await settle(ui);assert.match(ui.document.getElementById('launch-status').textContent,/shortly after/);assert.equal(ui.document.getElementById('launch').disabled,false);
 lose=true;ui.click();await settle(ui);assert.match(ui.document.getElementById('launch-status').textContent,/not retried/);await ui.app.refresh();await settle(ui);assert.equal(calls.filter(x=>x==='play').length,2);assert.equal(ui.document.getElementById('launch').disabled,true);
 await ui.app.dispose();await send({command:'reopen'});ui=mount();await ui.app.ready;await settle(ui);assert.match(ui.document.getElementById('launch-status').textContent,/unknown/);ui.click();assert.equal(calls.filter(x=>x==='play').length,2);
 const after=await send({command:'inspect',schema_version:1});assert.deepEqual(after.native.preferences,before.native.preferences);assert.equal(after.native.operation.operation.state,'reconciliation_required');
 console.log('Play logic UAT passed: native signed-minimum gate across polls/reopen with unchanged preferences and operation, admission/persistence, first-click feedback, duplicate click, running gate, early exit, lost reply without replay, reopen unknown, consent unchanged.');
 console.log('Excluded: real helper processes/injection, Windows/Wine/D3D9/x87, login/world entry, packaged visual/keyboard UAT.');
}finally{await ui.app.dispose();child.stdin.end();await new Promise(resolve=>child.once('exit',resolve));}
