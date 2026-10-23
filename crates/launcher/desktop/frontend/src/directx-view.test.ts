import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parseHTML} from 'linkedom';
import {mountDirectx} from './directx-view';
import {makePanelContext} from './panel-context';
import type {DirectxStatus,NativeSnapshot} from './contract';
const html=readFileSync('ui/index.html','utf8');
const snapshot:NativeSnapshot={schema_version:1,operation:{schema_version:1,revision:0,operation:null},requires_reopen:false,adopted_login_servers:null,launcher:{version:'test',tag:null},
 preferences:{schema_version:2,revision:0,install_directory:null,launcher_summary_consent:false,login_servers:[],client_patches_enabled:true}};
const status=(over:Partial<DirectxStatus>={}):DirectxStatus=>({schema_version:1,supported:true,missing:['d3dx9_40.dll','xinput1_3.dll'],installing:false,last_result:null,...over});
async function mounted(respond:(command:string)=>Promise<unknown>){
 const {document,window}=parseHTML(html);const feed=makePanelContext();feed.publish(snapshot);const calls:string[]=[];
 const app=mountDirectx(document as unknown as Document,async(command,args)=>{assert.equal(command,'directx_command');const request=(args as any).request;calls.push(request.command);return respond(request.command);},feed.context);
 await app.ready;
 const get=(id:string)=>document.getElementById(id) as unknown as HTMLButtonElement;
 return {app,feed,calls,get,press:()=>get('install-directx').dispatchEvent(new window.Event('click'))};
}
test('DirectX checks are visible during setup and re-run after install or adoption publishes a snapshot',async()=>{
 let installed=false;
 const p=await mounted(async()=>status({missing:installed?[]:['xinput1_3.dll']}));
 try{
  assert.equal(p.get('directx-panel').hidden,false);assert.ok(p.get('directx-panel').closest('#play'));
  assert.match(p.get('directx-state').textContent!,/xinput1_3.dll/);
  installed=true;p.feed.publish({...snapshot,operation:{...snapshot.operation,revision:1}});await p.app.settled();
  assert.equal(p.get('install-directx').hidden,true);
  installed=false;await p.app.refresh();assert.equal(p.get('install-directx').hidden,false,'readiness is re-probed, never trusted from a saved success');
 }finally{await p.app.dispose();}
});
test('setup acknowledges the first click, never queues a double click, and rechecks success',async()=>{
 let installed=false;let finish!:(value:unknown)=>void;
 const p=await mounted(async command=>command==='install'?new Promise(resolve=>{finish=resolve;}):status({missing:installed?[]:['d3dx9_40.dll']}));
 try{
  p.press();p.press();assert.equal(p.get('install-directx').disabled,true);assert.equal(p.get('install-directx').getAttribute('aria-busy'),'true');
  await new Promise(resolve=>setImmediate(resolve));assert.equal(p.calls.filter(x=>x==='install').length,1);
  installed=true;finish(status({missing:[],last_result:{state:'installed',restart_required:false}}));await p.app.settled();
  assert.equal(p.get('install-directx').hidden,true);assert.match(p.get('directx-status').textContent!,/installed/);
  assert.deepEqual(p.calls,['inspect','install','inspect']);
 }finally{await p.app.dispose();}
});
test('cancel, download refusal and uncertain replies remain retryable without replaying installation',async()=>{
 for(const failure of ['download_failed','package_mismatch','transport']){
  const p=await mounted(async command=>{if(command==='install')throw failure;return status();});
  try{p.press();await p.app.settled();assert.equal(p.get('install-directx').disabled,false);assert.equal(p.calls.filter(x=>x==='install').length,1);assert.ok(p.get('directx-status').textContent!.length>0);}finally{await p.app.dispose();}
 }
 const p=await mounted(async command=>status({last_result:command==='install'?{state:'cancelled_at_uac'}:null}));
 try{p.press();await p.app.settled();assert.match(p.get('directx-status').textContent!,/cancelled/);assert.equal(p.get('install-directx').disabled,false);}finally{await p.app.dispose();}
});
test('off Windows the DirectX panel is hidden',async()=>{
 const p=await mounted(async()=>status({supported:false,missing:[]}));
 try{assert.equal(p.get('directx-panel').hidden,true);}finally{await p.app.dispose();}
});
