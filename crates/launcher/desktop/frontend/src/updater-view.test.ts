import test from 'node:test';
import assert from 'node:assert/strict';
import {parseHTML} from 'linkedom';
import {mountUpdater,updaterText} from './updater-view';
import type {UpdaterState,UpdaterStatus} from './updater-workflow';
const status:UpdaterStatus={schema_version:1,revision:0,operation_revision:0,phase:'disabled',offer:null,failure:'disabled',requires_reopen:false};
const state:UpdaterState={status,pending:null,uncertain:false,error:null};
test('unconfigured release policy shows disabled and cannot dispatch check/download',async()=>{
 const {document,window}=parseHTML('<p id="updater-status"></p><pre id="updater-notes"></pre><button id="check-updater"></button><button id="prepare-updater"></button><button id="inspect-updater"></button>');
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
