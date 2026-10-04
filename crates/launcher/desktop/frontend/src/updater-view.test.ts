import test from 'node:test';
import assert from 'node:assert/strict';
import {parseHTML} from 'linkedom';
import {mountUpdater,updaterText} from './updater-view';
import type {UpdaterState,UpdaterStatus} from './updater-workflow';
const status:UpdaterStatus={schema_version:1,revision:0,operation_revision:0,phase:'disabled',offer:null,failure:'disabled',requires_reopen:false};
const state:UpdaterState={status,pending:null,uncertain:false,error:null};
test('unconfigured release policy shows disabled and cannot dispatch check/download',async()=>{
 const {document,window}=parseHTML('<p id="updater-status"></p><pre id="updater-notes"></pre><button id="check-updater"></button><button id="prepare-updater"></button><button id="apply-updater"></button><button id="inspect-updater"></button>');
 const calls:string[]=[];
 const app=mountUpdater(document as unknown as Document,async(_name,args)=>{calls.push((args?.request as {command:string}).command);return status;});
 try{
  await app.ready;await new Promise(resolve=>setImmediate(resolve));
  assert.match(document.getElementById('updater-status')!.textContent!,/updates are unavailable/);
  for(const id of ['check-updater','prepare-updater'])document.getElementById(id)!.dispatchEvent(new window.Event('click'));
  await app.settled();assert.deepEqual(calls,['inspect']);
 }finally{await app.dispose();}
});
test('ready explicitly means verified package, not installed update',()=>{
 assert.match(updaterText({...state,status:{...status,phase:'ready',failure:null}}),/running launcher has not changed/);
 assert.match(updaterText({...state,status:{...status,phase:'failed',failure:'signed_version'}}),/Nothing was installed/);
});
test('lost Apply reply never repeats replacement and inspection preserves pending ownership',async()=>{
 const {document,window}=parseHTML('<p id="updater-status"></p><pre id="updater-notes"></pre><button id="check-updater"></button><button id="prepare-updater"></button><button id="apply-updater"></button><button id="inspect-updater"></button>');
 let saved:UpdaterStatus={...status,phase:'ready',failure:null,offer:{id:'native-offer',version:'1.1.0',notes:''}};
 const calls:string[]=[];
 const app=mountUpdater(document as unknown as Document,async(_name,args)=>{
  const command=(args?.request as {command:string}).command;calls.push(command);
  if(command==='apply'){saved={...saved,revision:1,phase:'restart_required'};throw 'transport';}
  return saved;
 });
 try{
  await app.ready;await new Promise(resolve=>setImmediate(resolve));
  const apply=document.getElementById('apply-updater')!;
  apply.dispatchEvent(new window.Event('click'));apply.dispatchEvent(new window.Event('click'));
  await app.settled();await new Promise(resolve=>setImmediate(resolve));
  assert.equal(calls.filter(x=>x==='apply').length,1);
  assert.equal((apply as unknown as HTMLButtonElement).disabled,true);
  await app.refresh();await new Promise(resolve=>setImmediate(resolve));
  assert.match(document.getElementById('updater-status')!.textContent!,/completion is pending/);
  assert.equal((document.getElementById('check-updater') as unknown as HTMLButtonElement).disabled,true);
  assert.equal(calls.filter(x=>x==='apply').length,1);
 }finally{await app.dispose();}
});
