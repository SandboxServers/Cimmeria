import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parseHTML} from 'linkedom';
import type {NativeSnapshot} from './contract';
import {makePanelContext} from './panel-context';
import {mountClientPatches} from './client-patches-view';
const html=readFileSync('ui/index.html','utf8');
const servers=[{name:'Home',url:'http://home.example:8081'},{name:'Lab',url:'https://lab.example'}];
const snapshot=(revision:number,client_patches_enabled=true):NativeSnapshot=>({
 schema_version:1,operation:{schema_version:1,revision:0,operation:null},requires_reopen:false,adopted_login_servers:null,display_modes:['windowed' as const],launcher:{version:'0.1.0',tag:null},
 preferences:{schema_version:2,revision,install_directory:null,launcher_summary_consent:false,login_servers:servers,client_patches_enabled,display:{mode:'windowed' as const,width:1024,height:768}},
});
function mounted(respond:(request:any)=>Promise<unknown>=async()=>null,refresh:()=>Promise<unknown>=async()=>{},first:NativeSnapshot|null=snapshot(3)) {
 const {document,window}=parseHTML(html);const calls:{command:string;request:any}[]=[];
 const feed=makePanelContext(()=>refresh());if(first)feed.publish(first);
 const app=mountClientPatches(document as unknown as Document,async(command,args)=>{const request=(args as {request:unknown}).request;calls.push({command,request});return respond(request);},feed.context);
 const box=document.getElementById('client-patches-enabled') as unknown as HTMLInputElement;
 const toggle=()=>{box.checked=!box.checked;box.dispatchEvent(new window.Event('change'));};
 const text=(id:string)=>document.getElementById(id)!.textContent;
 return {document,calls,feed,app,box,toggle,text};
}

test('the box shows the saved setting and follows later snapshots',async()=>{
 const m=mounted();
 assert.equal(m.text('client-patches-title'),'Client patches');
 assert.match(m.text('client-patches-label')!,/Load client patches/);
 assert.equal(m.box.checked,true);assert.equal(m.box.disabled,false);
 assert.equal(m.document.getElementById('client-patches-adopted')!.hidden,true);
 m.feed.publish(snapshot(4,false));assert.equal(m.box.checked,false);
 await m.app.dispose();assert.equal(m.document.getElementById('client-patches-title'),null);
});

test('the box is disabled until settings load',async()=>{
 const m=mounted(undefined,undefined,null);
 assert.equal(m.box.disabled,true);
 m.feed.publish(snapshot(1,false));assert.equal(m.box.disabled,false);assert.equal(m.box.checked,false);
 await m.app.dispose();
});

test('turning it off saves with the saved login servers and the current revision, shows busy, then Saved',async()=>{
 const m=mounted(async()=>null,async()=>{m.feed.publish(snapshot(4,false));});
 m.toggle();
 assert.equal(m.text('client-patches-status'),'Saving client patches setting…');
 assert.equal(m.box.disabled,true);
 m.toggle();await m.app.settled();
 assert.equal(m.calls.length,1,'a change while saving sends nothing');
 assert.equal(m.calls[0]!.command,'launcher_command');
 assert.deepEqual(m.calls[0]!.request,{command:'save_client_options',schema_version:1,expected_revision:3,
  login_servers:servers,client_patches_enabled:false});
 assert.equal(m.text('client-patches-status'),'Saved. The next Play starts the game without client patches.');
 assert.equal(m.box.checked,false);assert.equal(m.box.disabled,false);
 await m.app.dispose();
});

test('a stale save re-inspects, shows the saved setting again and says nothing was saved',async()=>{
 let refreshes=0;
 const m=mounted(async()=>{throw 'stale_revision';},async()=>{refreshes++;m.feed.publish(snapshot(5,true));});
 m.toggle();await m.app.settled();
 assert.equal(refreshes,1);
 assert.equal(m.text('client-patches-status'),'Settings changed while saving, so nothing was saved. The box shows the saved setting; change it again if you still want to.');
 assert.equal(m.box.checked,true,'the refused change is put back');
 // The next change carries the revision the refresh brought.
 m.toggle();await m.app.settled();assert.equal(m.calls[1]!.request.expected_revision,5);
 await m.app.dispose();
});

test('a refresh that fails after a stale save is reported to telemetry',async()=>{
 const uiCalls:unknown[]=[];
 Object.assign(globalThis,{window:{__TAURI__:{core:{invoke:async(command:string,args:unknown)=>{uiCalls.push([command,args]);}}}}});
 try{
  const m=mounted(async()=>{throw 'stale_revision';},async()=>{throw 'transport';});
  m.toggle();await m.app.settled();
  assert.deepEqual(uiCalls,[['ui_event',{area:'client_patches',event:'refresh_failed',code:'transport',level:'warn'}]]);
  await m.app.dispose();
 }finally{Reflect.deleteProperty(globalThis,'window');}
});

test('a busy refusal says why and puts the box back',async()=>{
 const m=mounted(async()=>{throw 'busy';});
 m.toggle();await m.app.settled();
 assert.equal(m.text('client-patches-status'),'The launcher is installing, repairing or updating. Change the setting again when it finishes.');
 assert.equal(m.box.checked,true);
 await m.app.dispose();
});

test('a failed reload keeps the saved choice on screen without claiming it reloaded',async()=>{
 const m=mounted(async()=>null,async()=>{throw 'transport';});
 m.toggle();await m.app.settled();
 assert.equal(m.text('client-patches-status'),'Saved, but the launcher could not reload settings. Restart the launcher to see the saved setting.');
 assert.equal(m.box.checked,false);assert.equal(m.box.disabled,false);
 await m.app.dispose();
});

test('a refresh that brings no newer snapshot keeps the saved choice and says it could not reload',async()=>{
 // The refresh resolves with the pre-save snapshot (revision 3) still current.
 const m=mounted(async()=>null,async()=>{m.feed.publish(snapshot(3,true));});
 m.toggle();await m.app.settled();
 assert.equal(m.text('client-patches-status'),'Saved, but the launcher could not reload settings. Restart the launcher to see the saved setting.');
 assert.equal(m.box.checked,false,'the old snapshot does not put the previous value back');assert.equal(m.box.disabled,false);
 await m.app.dispose();
});

test('an adopted copy says it keeps its adopted setting, and a save says so',async()=>{
 const adopted=(revision:number,on:boolean)=>({...snapshot(revision,on),adopted_login_servers:[{name:'Reviewed',url:'http://reviewed.example:8081'}]});
 const m=mounted(async()=>null,async()=>{m.feed.publish(adopted(4,false));},adopted(3,true));
 const kept=m.document.getElementById('client-patches-adopted')!;
 assert.equal(kept.hidden,false);
 assert.match(kept.textContent!,/keeps the client-patches setting it was adopted with/);
 m.toggle();await m.app.settled();
 assert.equal(m.calls.length,1,'the save is allowed: it is the setting for this launcher\'s own installs');
 assert.equal(m.text('client-patches-status'),'Saved, but this copy keeps the client-patches setting it was adopted with.');
 await m.app.dispose();
});
