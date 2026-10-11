import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parseHTML} from 'linkedom';
import type {NativeSnapshot,PhysxStatus} from './contract';
import {makePanelContext} from './panel-context';
import {mountPhysx,physxWarning} from './physx-view';
const html=readFileSync('ui/index.html','utf8');
const snapshot:NativeSnapshot={schema_version:1,operation:{schema_version:1,revision:0,operation:null},requires_reopen:false,adopted_login_servers:null,
 preferences:{schema_version:2,revision:1,install_directory:null,launcher_summary_consent:false,login_servers:[],client_patches_enabled:true}};
const status=(over:Partial<PhysxStatus>={}):PhysxStatus=>({schema_version:1,supported:true,detection:{loader_present:false,core_registered:false},
 installer_available:true,installing:false,last_result:null,...over});
const flush=()=>new Promise<void>(resolve=>setImmediate(resolve));
async function mounted(respond:(request:{command:string})=>Promise<unknown>) {
 const {document,window}=parseHTML(html);const calls:string[]=[];
 const feed=makePanelContext();feed.publish(snapshot);
 const app=mountPhysx(document as unknown as Document,async(command,args)=>{
  assert.equal(command,'physx_command');const request=(args as {request:{command:string}}).request;calls.push(request.command);return respond(request);},feed.context);
 await app.ready;await flush();
 const get=(id:string)=>document.getElementById(id) as unknown as HTMLButtonElement;
 const press=()=>get('install-physx').dispatchEvent(new window.Event('click'));
 const text=(id:string)=>document.getElementById(id)!.textContent;
 return {calls,app,get,press,text,feed};
}

test('off Windows the panel and the Play warning stay hidden',async()=>{
 const p=await mounted(async()=>status({supported:false,detection:null,installer_available:false}));
 try{
  assert.equal(p.get('physx-panel').hidden,true);
  assert.equal(p.get('physx-play-warning').hidden,true);
  assert.deepEqual(p.calls,['inspect']);
 }finally{await p.app.dispose();}
});

test('missing PhysX warns beside Play without disabling it, and Install shows busy on the first press',async()=>{
 let answer!:(value:unknown)=>void;
 const p=await mounted(request=>request.command==='inspect'?Promise.resolve(status()):new Promise(resolve=>{answer=resolve;}));
 try{
  assert.equal(p.get('physx-panel').hidden,false);
  assert.equal(p.get('physx-play-warning').hidden,false);
  assert.equal(p.text('physx-play-warning'),physxWarning);
  assert.equal(p.get('physx-play-warning').closest('footer')!.querySelector('#launch')!==null,true,'the warning sits beside Play');
  assert.equal(p.get('install-physx').hidden,false);assert.equal(p.get('install-physx').disabled,false);
  p.press();p.press();
  assert.equal(p.get('install-physx').disabled,true,'busy before the native reply');
  assert.equal(p.get('install-physx').textContent,'Installing…');
  assert.equal(p.get('install-physx').getAttribute('aria-busy'),'true');
  assert.match(p.text('physx-status')!,/Windows permission prompt/);
  await flush();assert.deepEqual(p.calls,['inspect','install'],'a double click runs the installer once');
  answer(status({last_result:{state:'cancelled_at_uac'}}));await p.app.settled();await flush();
  assert.equal(p.text('physx-status'),'Install cancelled at the Windows permission prompt. Nothing was changed.');
  assert.equal(p.get('install-physx').disabled,false);assert.equal(p.get('install-physx').getAttribute('aria-busy'),'false');
  assert.equal(p.get('physx-play-warning').hidden,false,'still missing after a cancel');
 }finally{await p.app.dispose();}
});

test('a finished install clears the warning and shows the restart line',async()=>{
 let installed=false;
 const p=await mounted(async request=>{
  if(request.command==='install')installed=true;
  return installed?status({detection:{loader_present:true,core_registered:true},last_result:{state:'installed',restart_required:true}}):status();
 });
 try{
  p.press();await p.app.settled();await flush();
  assert.equal(p.text('physx-status'),'PhysX System Software installed. Restart Windows before you play.');
  assert.equal(p.get('physx-play-warning').hidden,true);
  assert.equal(p.get('install-physx').hidden,true);
  assert.equal(p.text('physx-state'),'Installed.');
  assert.deepEqual(p.calls,['inspect','install','inspect'],'the panel rechecks after the installer');
 }finally{await p.app.dispose();}
});

test('refusals and failures each get their own result line',async()=>{
 for(const [reply,line] of [
  [()=>Promise.reject('package_mismatch'),/not the expected file, so it was not run/],
  [()=>Promise.reject('busy'),/Another install, repair, update or game is running/],
  [()=>Promise.resolve(status({last_result:{state:'failed',exit_code:1603}})),/failed \(exit code 1603\)/],
  [()=>Promise.resolve(status({last_result:{state:'cancelled_in_installer'}})),/installer was cancelled/],
  [()=>Promise.resolve(status({last_result:{state:'not_detected',exit_code:0}})),/still not detected/],
  [()=>Promise.resolve(status({last_result:{state:'start_failed',win32_error:5}})),/could not start the PhysX installer \(error 5\)/],
 ] as const){
  const p=await mounted(request=>request.command==='inspect'?Promise.resolve(status()):reply());
  try{p.press();await p.app.settled();await flush();assert.match(p.text('physx-status')!,line);}finally{await p.app.dispose();}
 }
});

test('without the retained installer the button is disabled and says why',async()=>{
 const p=await mounted(async()=>status({installer_available:false}));
 try{
  assert.equal(p.get('install-physx').disabled,true);
  assert.match(p.text('physx-state')!,/not in this game folder/);
  p.press();await flush();assert.deepEqual(p.calls,['inspect'],'a disabled button sends nothing');
 }finally{await p.app.dispose();}
});
