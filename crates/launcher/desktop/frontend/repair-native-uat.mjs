// Real Effect/view program + production host dispatch, retained commit and cleanup.
import {spawn} from 'node:child_process';
import {createInterface} from 'node:readline';
import {readFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
import {randomUUID} from 'node:crypto';
import {parseHTML} from 'linkedom';
import {mountInstall} from './.test-build/install-view.mjs';
const child=spawn(process.env.REPAIR_UAT_BINARY,['--ignored','--nocapture','repair_native_uat_bridge'],{stdio:['pipe','pipe','inherit']});
const pending=[];
createInterface({input:child.stdout}).on('line',line=>{if(line.startsWith('REPAIR_NATIVE_UAT ')){const {resolve,reject}=pending.shift();const result=JSON.parse(line.slice(18));result.error?reject(result.error):resolve(result.ok);}});
const send=request=>new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(Error('native reply deadline')),10000);pending.push({resolve:x=>{clearTimeout(timer);resolve(x)},reject:x=>{clearTimeout(timer);reject(x)}});child.stdin.write(JSON.stringify(request)+'\n');});
const html=await readFile(new URL('./ui/index.html',import.meta.url),'utf8');
const {document,window}=parseHTML(html);
const calls=[];
const app=mountInstall(document,async(_,{request})=>{calls.push(request.command);return send(request)},randomUUID);
const get=id=>document.getElementById(id),click=id=>get(id).dispatchEvent(new window.Event('click'));
try {
 await app.ready;await app.settled();
 const initial=await send({command:'inspect',schema_version:1});
 click('repair');click('confirm-repair');click('confirm-repair');await app.settled();
 assert.equal(calls.filter(x=>x==='repair').length,1);
 let status=await send({command:'wait'});assert.equal(status.repair.backup,'retained');
 await app.refresh();await app.settled();
 assert.match(get('install-status').textContent,/old backup is retained/);assert.equal(get('cleanup-repair').hidden,false);
 click('cleanup-repair');click('confirm-repair');await app.settled();
 assert.equal(get('cleanup-repair').hidden,true);assert.match(get('install-status').textContent,/old backup has been removed/);
 status=await send({command:'reopen'});assert.equal(status.repair.backup,'removed');assert.equal(status.repair.cleanup,false);
 assert.deepEqual(status.native.preferences,initial.native.preferences);
 await app.refresh();await app.settled();assert.match(get('install-status').textContent,/old backup has been removed/);
 console.log('Native repair logic UAT passed: signed ZIP preparation, real retained host commit, duplicate suppression, explicit backup cleanup, accurate copy/capability after cleanup and reopen, unchanged preferences.');
 console.log('Excluded: real Wine/SGW, Windows locks, power loss, packaged visual/focus/layout UAT.');
} finally {await app.dispose();child.stdin.end();}
