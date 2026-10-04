import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parseHTML} from 'linkedom';
import {mountGameUpdate} from './game-update-view';
const html=readFileSync('ui/index.html','utf8');
const initial=()=>({schema_version:1,can_check:true,checked:true,offer:{id:'offer',installation_id:'owner',directory:'/fixture',current_digest:'old',target_digest:'new',current_patches:[],target_patches:[],launcher_update_required:false},progress:null,maintenance:null,native:{schema_version:1,requires_reopen:false,preferences:{schema_version:1,revision:1,install_directory:'/fixture',launcher_summary_consent:false},operation:{schema_version:1,revision:3,operation:null}}});
const tick=()=>new Promise<void>(resolve=>setImmediate(resolve));
test('changed native revision invalidates confirmation before any Apply command',async()=>{
 const status=initial(),calls:string[]=[];let change=false;
 const {document,window}=parseHTML(html);
 const app=mountGameUpdate(document,async(_,{request}:any)=>{calls.push(request.command);return change?{...status,native:{...status.native,operation:{...status.native.operation,revision:4}}}:status;});
 const click=async(id:string)=>{document.getElementById(id)!.dispatchEvent(new window.Event('click'));await app.settled();await tick();};
 try{
  await app.ready;await tick();await click('apply-game-update');
  assert.equal(document.getElementById('game-update-review')!.hidden,false);
  change=true;await click('confirm-game-update');
  assert(!calls.includes('apply'));
  assert.equal(document.getElementById('game-update-review')!.hidden,true);
  assert.match(document.getElementById('game-update-status')!.textContent!,/could not be confirmed/);
 }finally{await app.dispose();}
});
test('launcher minimum prevents game Apply and background inspection keeps check controls stable',async()=>{
 const status=initial();status.offer.launcher_update_required=true;
 let finish!:()=>void;const held=new Promise<void>(resolve=>{finish=resolve;});let reads=0;
 const {document}=parseHTML(html);
 const app=mountGameUpdate(document,async()=>{if(++reads===2)await held;return status;});
 try{
  await app.ready;await tick();const pending=app.refresh();await tick();
  assert.equal(document.getElementById('apply-game-update')!.hidden,true);
  assert.equal((document.getElementById('inspect-game-update') as HTMLButtonElement).disabled,false);
  assert.match(document.getElementById('game-update-status')!.textContent!,/Update the launcher/);
  finish();await pending;
 }finally{finish();await app.dispose();}
});
test('rollback binds both signed identities even when native revision is unchanged',async()=>{
 const status={...initial(),offer:null,maintenance:{operation_id:'completed',directory:'/fixture',previous_digest:'old',target_digest:'new',recovery:false,discard:false,rollback:true,backup:'retained'}};
 const calls:string[]=[];
 const {document,window}=parseHTML(html);
 const app=mountGameUpdate(document,async(_,{request}:any)=>{calls.push(request.command);return status;});
 const click=async(id:string)=>{document.getElementById(id)!.dispatchEvent(new window.Event('click'));await app.settled();await tick();};
 try{
  await app.ready;await tick();await click('rollback-game-update');
  assert.equal(document.getElementById('game-update-review')!.hidden,false);
  assert.match(document.getElementById('game-update-consequences')!.textContent!,/does not restore local modifications/);
  status.maintenance.previous_digest='changed-signed-release';
  await click('confirm-game-update');
  assert(!calls.includes('rollback'),'changed signed rollback destination invalidates confirmation');
  assert.equal(document.getElementById('game-update-review')!.hidden,true);
  assert.match(document.getElementById('game-update-status')!.textContent!,/could not be confirmed/);
 }finally{await app.dispose();}
});
