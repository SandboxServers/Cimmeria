// Production Effect/view against the production native host. All persistence is a
// temporary fixture; no session is minted, no DLL is loaded and nothing is sent.
import {spawn} from 'node:child_process';
import {createInterface} from 'node:readline';
import {readFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
import {parseHTML} from 'linkedom';
import {mountGameTelemetry} from './.test-build/game-telemetry-view.mjs';
const child=spawn(process.env.GAME_TELEMETRY_UAT_BINARY,['--ignored','--nocapture','game_telemetry_uat_bridge'],{stdio:['pipe','pipe','inherit']});
const exit=new Promise((resolve,reject)=>{child.on('error',reject);child.on('exit',code=>code===0?resolve():reject(Error(`native fixture exit ${code}`)));});
const pending=[];
createInterface({input:child.stdout}).on('line',line=>{if(line.startsWith('GAME_TELEMETRY_UAT ')){const response=pending.shift();const result=JSON.parse(line.slice(19));result.error?response.reject(result.error):response.resolve(result.ok);}});
const send=request=>new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(Error('native reply deadline')),10000);pending.push({resolve:x=>{clearTimeout(timer);resolve(x)},reject:x=>{clearTimeout(timer);reject(x)}});child.stdin.write(JSON.stringify(request)+'\n');});
const html=await readFile(new URL('./ui/index.html',import.meta.url),'utf8');
const calls=[];let lose=false;
const invoke=async(command,{request})=>{assert.equal(command,'game_telemetry_command');calls.push(request.command);const result=await send(request);if(lose&&request.command==='set'){lose=false;throw 'transport';}return result;};
const mount=()=>{const {document,window}=parseHTML(html);const box=document.getElementById('game-telemetry');
 return {document,box,app:mountGameTelemetry(document,invoke),text:()=>document.getElementById('game-telemetry-status').textContent,
  toggle:on=>{box.checked=on;box.dispatchEvent(new window.Event('change'));},recheck:()=>document.getElementById('inspect-game-telemetry').dispatchEvent(new window.Event('click'))};};
const settle=async ui=>{await ui.app.settled();await new Promise(resolve=>setImmediate(resolve));};
const sets=()=>calls.filter(x=>x==='set').length;
let ui=mount();
try{
 await ui.app.ready;await settle(ui);const before=await send({command:'preferences'});
 assert.equal(ui.box.checked,false);assert.equal(ui.box.disabled,false);assert.match(ui.text(),/^Off\./);
 // First press: feedback before the native reply, then one saved choice.
 ui.toggle(true);ui.toggle(true);assert.match(ui.text(),/Turning game diagnostics on/);assert.equal(ui.box.getAttribute('aria-busy'),'true');await settle(ui);
 assert.equal(sets(),1);assert.equal(ui.box.checked,true);assert.match(ui.text(),/^On\. The next Play/);
 assert.equal((await send({command:'inspect',schema_version:1})).opted_in,true);
 // The launcher-summary choice is a different record: unchanged, revision included.
 assert.deepEqual(await send({command:'preferences'}),before);assert.equal(before.launcher_summary_consent,false);
 // Restart: the saved choice comes back from disk.
 await ui.app.dispose();assert.equal((await send({command:'reopen'})).opted_in,true);ui=mount();await ui.app.ready;await settle(ui);
 assert.equal(ui.box.checked,true);assert.match(ui.text(),/^On\./);
 // Lost reply: not replayed; a recheck shows what native storage holds.
 lose=true;ui.toggle(false);await settle(ui);assert.equal(sets(),2);assert.match(ui.text(),/could not be confirmed.*nothing was retried/);assert.equal(ui.box.disabled,true);
 ui.toggle(false);await settle(ui);assert.equal(sets(),2);
 ui.recheck();await settle(ui);assert.equal(sets(),2);assert.equal(ui.box.checked,false);assert.match(ui.text(),/^Off\./);
 assert.equal((await send({command:'inspect',schema_version:1})).opted_in,false);
 // A build without the module: an opt-in is refused natively and nothing is saved; opting out still works.
 ui.toggle(true);await settle(ui);assert.equal(sets(),3);
 await ui.app.dispose();const without=await send({command:'reopen_without_module'});assert.equal(without.available,false);assert.equal(without.opted_in,true);
 ui=mount();await ui.app.ready;await settle(ui);assert.match(ui.text(),/does not include the module\. Nothing is sent/);assert.equal(ui.box.disabled,false);
 ui.toggle(false);await settle(ui);assert.equal(sets(),4);assert.equal(ui.box.disabled,true);assert.match(ui.text(),/Not available in this build/);
 await assert.rejects(send({command:'set',schema_version:1,opted_in:true}),e=>e==='platform_unavailable');
 assert.equal((await send({command:'inspect',schema_version:1})).opted_in,false);
 assert.deepEqual(await send({command:'preferences'}),before);
 console.log('Game diagnostics logic UAT passed: off by default, first-press feedback, one native save per press, duplicate suppression, disk reopen, lost reply without replay and recheck, native refusal without the module, opt-out without the module, launcher-summary preferences untouched throughout.');
 console.log('Excluded: session mint against a server, the session marker, DLL injection, Wine, uploads, and packaged visual/keyboard/focus UAT.');
}finally{await ui.app.dispose();child.stdin.end();await exit;}
