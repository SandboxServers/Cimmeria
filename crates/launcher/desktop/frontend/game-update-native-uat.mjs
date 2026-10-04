import {spawn} from 'node:child_process';
import {createInterface} from 'node:readline';
import assert from 'node:assert/strict';
import {Effect} from 'effect';
import {makeGameUpdateWorkflow,gameUpdateBridgeLayer} from './.test-build/game-update-workflow.mjs';
const child=spawn(process.env.GAME_UPDATE_UAT_BINARY,['--ignored','--nocapture','game_update_native_uat_bridge'],{stdio:['pipe','pipe','inherit']});
const exit=new Promise((resolve,reject)=>{child.on('error',reject);child.on('exit',code=>code===0?resolve():reject(Error(`native fixture exit ${code}`)));});
const pending=[];
createInterface({input:child.stdout}).on('line',line=>{
 if(!line.startsWith('GAME_UPDATE_UAT '))return;
 const response=pending.shift(),result=JSON.parse(line.slice(16));
 result.error?response.reject(result.error):response.resolve(result.ok);
});
const send=request=>new Promise((resolve,reject)=>{
 const timer=setTimeout(()=>reject(Error('native reply deadline')),10000);
 pending.push({resolve:x=>{clearTimeout(timer);resolve(x)},reject:x=>{clearTimeout(timer);reject(x)}});
 child.stdin.write(JSON.stringify(request)+'\n');
});
const calls=[];
let loseReply=false;
try {
 await Effect.runPromise(Effect.scoped(Effect.gen(function*(){
  const workflow=yield* makeGameUpdateWorkflow;
  const initial=yield* workflow.inspect;
  assert(initial.can_check&&!initial.checked&&!initial.offer);
  const reviewed=yield* workflow.check;
  assert(reviewed.checked&&reviewed.offer);
  assert.notEqual(reviewed.offer.current_digest,reviewed.offer.target_digest);
  assert.deepEqual(reviewed.native,initial.native);
  loseReply=true;
  assert((yield* Effect.result(workflow.check))._tag==='Failure');
  const failedCalls=calls.length;
  assert((yield* Effect.result(workflow.check))._tag==='Failure');
  assert.equal(calls.length,failedCalls,'uncertain check requires inspection');
  const inspected=yield* workflow.inspect;
  assert(inspected.offer,'lost reply leaves native authenticated offer observable');
  yield* Effect.promise(()=>send({command:'reopen'}));
  const reopened=yield* workflow.inspect;
  assert(!reopened.checked&&!reopened.offer,'process restart invalidates transient confirmation');
  assert.deepEqual(reopened.native,initial.native,'checking never changes durable operation or preferences');
 }).pipe(Effect.provide(gameUpdateBridgeLayer(async request=>{
  calls.push(request.command);
  const result=await send(request);
  if(request.command==='check'&&loseReply){loseReply=false;throw 'transport';}
  return result;
 })))));
 console.log('Game Update offer UAT passed: signed identities, unchanged persistent state, lost reply inspection, reopen invalidates review. No reconstruction, network or visual UI exercised.');
} finally {child.stdin.end();await exit;}
