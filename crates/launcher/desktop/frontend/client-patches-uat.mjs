// Logic UAT (LX-04): the real Settings page with the client-patches panel, against a
// stateful native fake that keeps preferences in a temporary file the way the
// engine does (revision check, both options saved together). A "reload" mounts
// a fresh page and a fresh fake over the same file.
import {mkdtemp,readFile,rm,writeFile} from 'node:fs/promises';
import {existsSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import assert from 'node:assert/strict';
import {parseHTML} from 'linkedom';
import {mountClientPatches} from './.test-build/client-patches-view.mjs';
const page=await readFile(new URL('./ui/index.html',import.meta.url),'utf8');
const dir=await mkdtemp(join(tmpdir(),'client-patches-uat-'));
const file=join(dir,'preferences.json');
const servers=[{name:'Home',url:'http://home.example:8081'}];

function nativeFake() {
 const load=async()=>existsSync(file)?JSON.parse(await readFile(file,'utf8')):
  {schema_version:2,revision:0,install_directory:null,launcher_summary_consent:false,login_servers:[],client_patches_enabled:true};
 const snapshot=preferences=>({schema_version:1,operation:{schema_version:1,revision:0,operation:null},preferences,requires_reopen:false,adopted_login_servers:null,launcher:{version:'0.1.0',tag:null}});
 const calls=[];
 const invoke=async(command,args)=>{
  assert.equal(command,'launcher_command');const request=args.request;calls.push(request.command);
  const preferences=await load();
  if(request.command==='inspect')return snapshot(preferences);
  assert.equal(request.command,'save_client_options');
  if(request.expected_revision!==preferences.revision)throw 'stale_revision';
  const next={...preferences,revision:preferences.revision+1,login_servers:request.login_servers,client_patches_enabled:request.client_patches_enabled};
  await writeFile(file,JSON.stringify(next));return snapshot(next);
 };
 return {invoke,calls};
}
async function open() {
 const {document,window}=parseHTML(page);const fake=nativeFake();
 let latest;const listeners=new Set();
 const refresh=async()=>{latest=await fake.invoke('launcher_command',{request:{command:'inspect',schema_version:1}});for(const l of listeners)l(latest);};
 const context={snapshot:()=>latest,onSnapshot:l=>{listeners.add(l);return()=>listeners.delete(l);},refresh};
 const app=mountClientPatches(document,fake.invoke,context);
 await refresh();await app.ready;
 const box=document.getElementById('client-patches-enabled');
 const toggle=async()=>{box.checked=!box.checked;box.dispatchEvent(new window.Event('change'));
  const busy=document.getElementById('client-patches-status').textContent;await app.settled();return busy;};
 const text=id=>document.getElementById(id).textContent;
 return {document,fake,app,box,toggle,text};
}

try {
 // A server list saved earlier (LX-03) must survive this panel's saves.
 await writeFile(file,JSON.stringify({schema_version:2,revision:1,install_directory:null,launcher_summary_consent:false,login_servers:servers,client_patches_enabled:true}));
 let p=await open();
 assert.ok(p.document.getElementById('client-patches-panel').closest('#gear'),'the panel sits in Settings');
 assert.equal(p.text('client-patches-title'),'Client patches');
 assert.match(p.text('client-patches-label'),/Load client patches/);
 assert.equal(p.box.checked,true,'patches load by default');

 assert.equal(await p.toggle(),'Saving client patches setting…');
 assert.equal(p.text('client-patches-status'),'Saved. The next Play starts the game without client patches.');
 assert.deepEqual(p.fake.calls,['inspect','save_client_options','inspect']);
 await p.app.dispose();

 p=await open();
 assert.equal(p.box.checked,false,'off persists across a reload');
 const saved=JSON.parse(await readFile(file,'utf8'));
 assert.equal(saved.client_patches_enabled,false);
 assert.deepEqual(saved.login_servers,servers,'the saved server list is kept');

 await p.toggle();
 assert.equal(p.text('client-patches-status'),'Saved. The next Play loads client patches.');
 await p.app.dispose();
 p=await open();
 assert.equal(p.box.checked,true,'on persists across a reload');

 // Another window saves first: this save is refused and the box shows the file's value.
 await writeFile(file,JSON.stringify({...JSON.parse(await readFile(file,'utf8')),revision:9,client_patches_enabled:true}));
 await p.toggle();
 assert.match(p.text('client-patches-status'),/^Settings changed while saving, so nothing was saved\./);
 assert.equal(p.box.checked,true);
 assert.equal(JSON.parse(await readFile(file,'utf8')).revision,9,'the stale save wrote nothing');
 await p.app.dispose();

 console.log('Client patches UAT passed: the box defaults on; turning it off shows busy then Saved and persists across a reload with the server list kept; turning it on persists; a stale save is refused, re-read and shown.');
 console.log('Excluded: the real engine and Tauri bridge (engine tests cover the launch plan without the patches DLL and the adopted-copy binding); the "Changes to your client" panel (engine test dll_rows_match_what_play_would_inject); the packaged window and visual layout.');
}finally{await rm(dir,{recursive:true,force:true});}
