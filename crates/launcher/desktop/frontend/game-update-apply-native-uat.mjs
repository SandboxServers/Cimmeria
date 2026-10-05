import {spawn} from 'node:child_process';
import {createInterface} from 'node:readline';
import {readFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
import {parseHTML} from 'linkedom';
import {mountGameUpdate} from './.test-build/game-update-view.mjs';
const child=spawn(process.env.GAME_UPDATE_UAT_BINARY,['--ignored','--nocapture','game_update_apply_uat_bridge'],{stdio:['pipe','pipe','inherit']});
const exit=new Promise((resolve,reject)=>{child.on('error',reject);child.on('exit',code=>code===0?resolve():reject(Error(`native fixture exit ${code}`)));});
const pending=[];
createInterface({input:child.stdout}).on('line',line=>{
 if(!line.startsWith('GAME_UPDATE_APPLY_UAT '))return;
 const response=pending.shift(),result=JSON.parse(line.slice('GAME_UPDATE_APPLY_UAT '.length));
 result.error?response.reject(result.error):response.resolve(result.ok);
});
const send=request=>new Promise((resolve,reject)=>{
 const timer=setTimeout(()=>reject(Error('native reply deadline')),15000);
 pending.push({resolve:x=>{clearTimeout(timer);resolve(x)},reject:x=>{clearTimeout(timer);reject(x)}});
 child.stdin.write(JSON.stringify(request)+'\n');
});
const {document,window}=parseHTML(await readFile(new URL('./ui/index.html',import.meta.url),'utf8'));
const get=id=>document.getElementById(id),calls=[];
let loseApply=true,loseRollback=true;
const app=mountGameUpdate(document,async(name,args)=>{
 assert.equal(name,'game_update_command');calls.push(args.request);
 const result=await send(args.request);
 if(args.request.command==='apply'&&loseApply){loseApply=false;throw 'transport';}
 if(args.request.command==='rollback'&&loseRollback){loseRollback=false;throw 'transport';}
 return result;
});
const tick=()=>new Promise(resolve=>setTimeout(resolve,10));
const click=async id=>{get(id).dispatchEvent(new window.Event('click'));await app.settled();await tick();};
try {
 await app.ready;await tick();
 const initial=await send({command:'inspect',schema_version:1});
 assert(initial.offer);assert.equal(get('apply-game-update').hidden,false);
 await click('apply-game-update');
 assert.equal(get('game-update-review').hidden,false);
 assert.match(get('game-update-consequences').textContent,/not merged/);
 assert(get('game-update-identities').textContent.includes(initial.offer.target_digest));
 assert.equal(calls.filter(x=>x.command==='apply').length,0,'review never mutates the game');
 await click('confirm-game-update');
 assert.match(get('game-update-status').textContent,/could not be confirmed/);
 assert.equal(calls.filter(x=>x.command==='apply').length,1);
 const completed=await send({command:'wait'});
 await app.refresh();await tick();
 assert.equal(completed.native.operation.operation.state,'succeeded');
 assert.deepEqual(completed.native.preferences,initial.native.preferences);
 assert.equal(get('cleanup-game-update').hidden,false);
 assert.equal(get('rollback-game-update').hidden,false);
 await send({command:'reopen'});await app.refresh();await tick();
 await click('rollback-game-update');
 assert.match(get('game-update-consequences').textContent,/does not restore local modifications/);
 assert.equal(calls.filter(x=>x.command==='rollback').length,0);
 await click('confirm-game-update');
 assert.match(get('game-update-status').textContent,/could not be confirmed/);
 const rolledBack=await send({command:'wait'});
 assert.equal(rolledBack.native.operation.operation.state,'succeeded');
 assert.equal(rolledBack.maintenance.target_digest,initial.offer.current_digest);
 assert.equal(rolledBack.maintenance.previous_digest,initial.offer.target_digest);
 assert.notEqual(rolledBack.maintenance.operation_id,completed.maintenance.operation_id);
 await send({command:'reopen'});await app.refresh();await tick();
 assert.equal(calls.filter(x=>x.command==='rollback').length,1,'lost rollback reply never redispatches');
 await click('cleanup-game-update');
 assert.match(get('game-update-consequences').textContent,/Permanently remove/);
 await click('dismiss-game-update');
 assert.equal(calls.filter(x=>x.command==='maintain').length,0);
 await click('cleanup-game-update');await click('confirm-game-update');
 assert.equal(get('cleanup-game-update').hidden,true);
 await send({command:'reopen'});await app.refresh();await tick();
 const reopened=await send({command:'inspect',schema_version:1});
 assert.equal(reopened.maintenance.backup,'removed');
 assert.equal(reopened.native.operation.operation.state,'succeeded');
 assert.equal(calls.filter(x=>x.command==='apply').length,1,'lost apply reply never redispatches');
 console.log('Game Update Apply UAT passed: rendered confirmation, signed identities, actual download/replacement, lost-reply inspection, confirmed signed rollback with lost reply, confirmed cleanup, persistent reopen. Actual game, platform-specific helpers and visual webview not covered.');
} finally {await app.dispose();child.stdin.end();await exit;}
