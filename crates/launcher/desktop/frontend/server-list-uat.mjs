// Logic UAT (LX-03): the real Settings page with the server-list panel, against a
// stateful native fake that keeps preferences in a temporary file the way the
// engine does (revision check, empty list allowed, invalid list refused). A
// "reload" mounts a fresh page and a fresh fake over the same file.
import {mkdtemp,readFile,rm,writeFile} from 'node:fs/promises';
import {existsSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import assert from 'node:assert/strict';
import {parseHTML} from 'linkedom';
import {mountServerList} from './.test-build/server-list-view.mjs';
const page=await readFile(new URL('./ui/index.html',import.meta.url),'utf8');
const dir=await mkdtemp(join(tmpdir(),'server-list-uat-'));
const file=join(dir,'preferences.json');

function nativeFake() {
 const load=async()=>existsSync(file)?JSON.parse(await readFile(file,'utf8')):
  {schema_version:2,revision:0,install_directory:null,launcher_summary_consent:false,login_servers:[],client_patches_enabled:true};
 const valid=s=>typeof s==='string'&&s.length>0&&s.length<=200&&/^[\x20-\x7e]*$/.test(s)&&!/["\\]/.test(s);
 const snapshot=preferences=>({schema_version:1,operation:{schema_version:1,revision:0,operation:null},preferences,requires_reopen:false,adopted_login_servers:null,launcher:{version:'0.1.0',tag:null}});
 const calls=[];
 const invoke=async(command,args)=>{
  assert.equal(command,'launcher_command');const request=args.request;calls.push(request.command);
  const preferences=await load();
  if(request.command==='inspect')return snapshot(preferences);
  assert.equal(request.command,'save_client_options');
  if(request.expected_revision!==preferences.revision)throw 'stale_revision';
  if(request.login_servers.some(s=>!valid(s.name)||!valid(s.url)||!/^https?:\/\//.test(s.url)))throw 'invalid_server_list';
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
 const app=mountServerList(document,fake.invoke,context);
 await refresh();await app.ready;
 const box=document.getElementById('server-list-text');
 const edit=text=>{box.value=text;box.dispatchEvent(new window.Event('input'));};
 const save=async()=>{document.getElementById('save-server-list').dispatchEvent(new window.Event('click'));
  const busy=document.getElementById('server-list-status').textContent;await app.settled();return busy;};
 const text=id=>document.getElementById(id).textContent;
 return {document,fake,app,box,edit,save,text};
}

try {
 let p=await open();
 assert.ok(p.document.getElementById('server-list-panel').closest('#gear'),'the panel sits in Settings');
 assert.equal(p.text('server-list-title'),'Login servers');
 assert.equal(p.box.value,'','nothing saved yet, so the box is empty (the default server)');

 p.edit('Home = http://home.example:8081\nLab = lab.example');
 await p.save();
 assert.equal(p.text('server-list-error'),'line 2: the URL must start with http:// or https://');
 assert.deepEqual(p.fake.calls,['inspect'],'a bad line sends no save');
 assert.equal(existsSync(file),false);

 p.edit('# my servers\nHome = http://home.example:8081\nLab = https://lab.example');
 assert.equal(await p.save(),'Saving login servers…');
 assert.equal(p.text('server-list-status'),'Saved. The next Play uses this list.');
 assert.deepEqual(p.fake.calls,['inspect','save_client_options','inspect']);
 assert.equal(p.box.value,'# my servers\nHome = http://home.example:8081\nLab = https://lab.example','the player text that means the saved list is kept, comment included');
 await p.app.dispose();

 p=await open();
 assert.equal(p.box.value,'Home = http://home.example:8081\nLab = https://lab.example','the list persists across a reload');
 assert.deepEqual(JSON.parse(await readFile(file,'utf8')).login_servers,
  [{name:'Home',url:'http://home.example:8081'},{name:'Lab',url:'https://lab.example'}]);

 p.edit('');await p.save();
 assert.equal(p.text('server-list-status'),'Saved. The next Play uses the default server.');
 await p.app.dispose();
 p=await open();
 assert.equal(p.box.value,'','an emptied list persists as the default server');
 await p.app.dispose();

 console.log('Login servers UAT passed: a bad line is shown and sends nothing; Save shows busy then Saved; the list persists across a reload; an emptied list saves as the default server.');
 console.log('Excluded: the real engine and Tauri bridge (engine tests cover the refusal and LoginInternal.lua); the packaged window and visual layout.');
}finally{await rm(dir,{recursive:true,force:true});}
