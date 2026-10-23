import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {parseHTML} from 'linkedom';
import {mountDesktop} from './composition';
import type {AdoptionStatus} from './adoption-workflow';
const html=await readFile(new URL('../ui/index.html',import.meta.url),'utf8');
const native=(revision=0)=>({schema_version:1 as const,requires_reopen:false,adopted_login_servers:null,launcher:{version:'0.1.0',tag:null},preferences:{schema_version:2 as const,revision:0,install_directory:null,launcher_summary_consent:false,login_servers:[],client_patches_enabled:true},operation:{schema_version:1 as const,revision,operation:null}});
const idle=():AdoptionStatus=>({schema_version:1,native:native(),backend:'in_place',imported:null,modes:['light','in_place'],activity:'idle',review:null,progress:null,cancellable:false,reconciliation:null,preparations:[],has_valid_install:false,owned:false,completed:null,last_error:null,last_rejection:null,unsafe_entries:null});
const flush=()=>new Promise<void>(resolve=>setTimeout(resolve,10));
function fixture(initial=idle()){
 const {document,window}=parseHTML(html);let saved=initial,choose:()=>Promise<unknown>=async()=>saved;const calls:string[]=[];
 const app=mountDesktop(document,async(command)=>{
  calls.push(command);
  if(command==='adoption_command')return saved;
  if(command==='choose_existing_game_folder')return choose();
  if(command==='launcher_command')return saved.native;
  if(command==='launch_command')return {schema_version:1,native:saved.native,installation_id:null,resources_available:false,launcher_update_required:true,patches_incomplete:0,ops_skipped:0,observation:null};
  // Other panels keep their own strict decoders; unavailable capabilities do not gate adoption.
  if(command==='install_command')return {schema_version:1,native:saved.native,install_supported:false,can_resume:false,can_reconcile:false,can_retry:false,uninstall:null,runtime_setup:null,progress:null,outcome:null};
  return native();
 });
 const get=(id:string)=>document.getElementById(id)!;
 const settle=async()=>{await app.app.settled();await app.adoption!.settled();await flush();await app.adoption!.settled();};
 return {app,get,calls,save:(s:AdoptionStatus)=>{saved=s;},picker:(f:()=>Promise<unknown>)=>{choose=f;},click:(id:string)=>get(id).dispatchEvent(new window.Event('click')),settle,
 ready:async()=>{await Promise.all([app.app.ready,app.adoption!.ready]);await settle();}};
}
for(const kind of ['managed','adopted','busy'])test(`valid ${kind} content hides adoption despite runtime and launcher-minimum gates`,async()=>{
 const base:AdoptionStatus=kind==='busy'?{...idle(),native:{...native(),operation:{schema_version:1,revision:1,operation:{id:'other',kind:'repair',state:'running',intent_digest:Array(32).fill(0)}}}}:idle();
 const ui=fixture({...base,owned:true,has_valid_install:true});
 try{await ui.ready();assert.equal(ui.get('adopt-install').hidden,true);
  assert.equal((ui.get('launch') as HTMLButtonElement).disabled,true,'Play gates do not define content validity');
 }finally{await ui.app.dispose();}
});
test('invalid registered or uncertain state is visible with actionable disabled feedback',async()=>{
 const ui=fixture({...idle(),owned:true});
 try{await ui.ready();assert.equal(ui.get('adopt-install').hidden,false);assert.equal((ui.get('adopt-install') as HTMLButtonElement).disabled,true);assert.match(ui.get('adopt-install-status').textContent!,/Repair it or uninstall/);
  ui.save({...idle(),native:{...native(1),requires_reopen:true}});await ui.app.adoption!.refresh();await ui.settle();
  assert.equal((ui.get('adopt-install') as HTMLButtonElement).disabled,true);assert.match(ui.get('adopt-install-status').textContent!,/Recheck adoption status/);
 }finally{await ui.app.dispose();}
});
test('first footer press reveals the single workflow before awaiting picker; repeat, cancel, and failure are visible',async()=>{
 const ui=fixture();let finish!:(s:AdoptionStatus)=>void;
 ui.picker(()=>new Promise(resolve=>{finish=resolve;}));
 try{await ui.ready();assert.equal(ui.get('adopt-install').hidden,false);
  ui.click('adopt-install');
  assert.equal(ui.get('gear').hidden,false);assert.equal(ui.get('settings').getAttribute('aria-expanded'),'true');assert.equal(ui.get('play').hidden,true);
  assert.equal(ui.get('adoption-status').textContent,'Checking your game folder…');
  ui.click('adopt-install');ui.click('choose-adoption-destination');await flush();
  assert.equal(ui.calls.filter(c=>c==='choose_existing_game_folder').length,1);
  finish(idle());await ui.settle();assert.equal(ui.get('adoption-status').textContent,'No folder chosen.');
  ui.picker(async()=>{throw 'not_a_game_folder';});ui.click('choose-adoption-destination');await ui.settle();
  assert.match(ui.get('adoption-status').textContent!,/not a Stargate Worlds game folder/);
 }finally{finish?.(idle());await ui.app.dispose();}
});
test('closed Settings follows durable install/uninstall revisions and reopening derives visibility anew',async()=>{
 const ui=fixture();
 try{await ui.ready();assert.equal(ui.get('gear').hidden,true);
  ui.save({...idle(),native:native(1),owned:true,has_valid_install:true});await ui.app.app.refresh();await ui.settle();
  assert.equal(ui.get('adopt-install').hidden,true);assert.equal(ui.get('gear').hidden,true);
  ui.save({...idle(),native:native(2)});await ui.app.app.refresh();await ui.settle();assert.equal(ui.get('adopt-install').hidden,false);
 }finally{await ui.app.dispose();}
 const reopened=fixture({...idle(),owned:true,has_valid_install:true});
 try{await reopened.ready();assert.equal(reopened.get('adopt-install').hidden,true);}finally{await reopened.app.dispose();}
});
