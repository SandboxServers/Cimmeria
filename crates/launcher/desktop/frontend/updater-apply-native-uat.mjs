// Real Effect Apply command -> native temp swap: Mac bundle exchange + spawned fixture, or Windows zip swap + recorded relaunch.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {readFile} from 'node:fs/promises';
import {createInterface} from 'node:readline';
import {parseHTML} from 'linkedom';
import {mountUpdater} from './.test-build/updater-view.mjs';
const child=spawn(process.env.UPDATER_UAT_BINARY,['--ignored','--nocapture','updater_apply_native_uat_bridge'],{stdio:['pipe','pipe','inherit']});
const pending=[];let stderr='';
createInterface({input:child.stdout}).on('line',line=>{if(line.startsWith('UPDATER_APPLY_UAT ')){const result=JSON.parse(line.slice('UPDATER_APPLY_UAT '.length));const item=pending.shift();if(result.error)item.reject(result.error);else item.resolve(result.ok);}else stderr+=line+'\n';});
child.on('exit',code=>{for(const item of pending)item.reject(new Error(`native bridge exited ${code}: ${stderr}`));});
const send=request=>new Promise((resolve,reject)=>{pending.push({resolve,reject});child.stdin.write(JSON.stringify(request)+'\n');});
const {document,window}=parseHTML(await readFile(new URL('./ui/index.html',import.meta.url),'utf8'));
const calls=[];const app=mountUpdater(document,async(name,args)=>{assert.equal(name,'updater_command');calls.push(args.request);return send(args.request);});
const tick=()=>new Promise(resolve=>setImmediate(resolve));
const get=id=>document.getElementById(id);
try{
 await app.ready;await tick();assert.equal(get('apply-updater').disabled,false);
 get('apply-updater').dispatchEvent(new window.Event('click'));get('apply-updater').dispatchEvent(new window.Event('click'));
 await app.settled();await tick();assert.equal(calls.filter(v=>v.command==='apply').length,1);
 assert.match(get('updater-status').textContent,/completion is pending/);assert.equal(get('apply-updater').disabled,true);
 await assert.rejects(send({command:'try_game_mutation'}),e=>e.storage==='busy');
 await send({command:'reopen'});await app.refresh();await tick();assert.match(get('updater-status').textContent,/completion is not confirmed/);
 assert.equal(get('check-updater').disabled,true);assert.equal(get('apply-updater').disabled,true);
 await send({command:'acknowledge'});await app.refresh();await tick();assert.match(get('updater-status').textContent,/confirmed its compiled release version/);
 await send({command:'try_game_mutation'});
 const swap=process.platform==='win32'?'real isolated Windows zip swap (.old aside, new files in, executable last) with the relaunch recorded, and .old/stage cleanup on the acknowledging start':'real isolated Mac bundle replacement and spawned fixture executable';
 console.log(`Apply UAT passed: production Effect/view, one native Apply per double click, ${swap}, durable pending ownership, game exclusion, reopen without false success, simulated compiled-version acknowledgment.`);
 console.log('Not covered: packaged Tauri IPC/layout/exit, replacement of the actual running launcher, new production binary identity, the pid wait before the single-instance guard, OS signing/notarization, app health, release publication.');
}finally{await app.dispose();child.stdin.end();await new Promise(resolve=>child.once('exit',resolve));}
