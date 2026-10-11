// Production Effect workflow and view against the production native host where
// adoption is in place (D-LX12). The Rust bridge owns a real isolated store, an
// imported old-launcher game folder and a signed loopback artifact origin.
// Nothing here answers for the native side.
import {spawn} from 'node:child_process';
import {createInterface} from 'node:readline';
import {mkdtemp,readFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import assert from 'node:assert/strict';
import {parseHTML} from 'linkedom';
import {mountAdoption} from './.test-build/adoption-view.mjs';
const root=await mkdtemp(join(tmpdir(),'adoption-in-place-uat-'));
const TAG='ADOPTION_NATIVE_UAT ';
const child=spawn(process.env.ADOPTION_UAT_BINARY,['--ignored','--nocapture','--exact','host::adoption::uat_bridge_in_place::adoption_in_place_uat_bridge'],{stdio:['pipe','pipe','inherit'],env:{...process.env,ADOPTION_UAT_ROOT:root}});
const pending=[];
const exit=new Promise(resolve=>{child.on('error',()=>resolve(-1));child.on('exit',code=>resolve(code));}).then(code=>{for(const p of pending.splice(0))p.reject('transport');return code;});
createInterface({input:child.stdout}).on('line',line=>{if(!line.startsWith(TAG))return;const response=pending.shift(),result=JSON.parse(line.slice(TAG.length));'error' in result?response.reject(result.error):response.resolve(result.ok);});
const send=request=>new Promise((resolve,reject)=>{
 const timer=setTimeout(()=>reject(Error(`native reply deadline: ${JSON.stringify(request)}`)),120000);
 pending.push({resolve:x=>{clearTimeout(timer);resolve(x)},reject:x=>{clearTimeout(timer);reject(x)}});child.stdin.write(JSON.stringify(request)+'\n');});
const {document,window}=parseHTML(await readFile(new URL('./ui/index.html',import.meta.url),'utf8'));
const calls=[];
const app=mountAdoption(document,async(command,args)=>{
 const request=command==='choose_adoption_destination'?{command:'choose'}:(assert.equal(command,'adoption_command'),args.request);
 calls.push(request);
 return send(request);
});
const get=id=>document.getElementById(id),text=()=>get('adoption-status').textContent;
const sent=command=>calls.filter(c=>c.command===command);
const pause=ms=>new Promise(resolve=>setTimeout(resolve,ms));
const settle=async()=>{await app.settled();await pause(10);await app.settled();};
const press=id=>get(id).dispatchEvent(new window.Event('click'));
const click=async id=>{assert.equal(get(id).disabled,false,`${id} is enabled`);assert.equal(get(id).hidden,false,`${id} is visible`);press(id);await settle();};
const tick=async id=>{assert.equal(get(id).disabled,false,`${id} is enabled`);get(id).checked=true;get(id).dispatchEvent(new window.Event('change'));await settle();};
const tickShown=async()=>{for(const id of ['adoption-normalize','adoption-closed','adoption-telemetry'])if(!get(`${id}-row`).hidden)await tick(id);};
const inspect=()=>send({command:'inspect',schema_version:1});
const evidence=()=>send({command:'evidence'});
async function until(what,ready,limit=60000) {
 const deadline=Date.now()+limit;
 for(;;){const status=await inspect();
  // The view reads the same native state the script just observed.
  if(await ready(status)){await app.refresh();await settle();return status;}
  assert(Date.now()<deadline,`timed out waiting for ${what}: ${text()} ${JSON.stringify({activity:status.activity,last_error:status.last_error})}`);await pause(50);}
}
try {
 assert.equal((await send({command:'hello'})).backend,'in_place');
 await app.ready;await settle();
 const before=await evidence();
 assert.equal(before.owner_marker,false);assert.equal(before.owned_directory,null);assert.equal(before.executable_in_old_place,true);
 assert.match(text(),/Adopt your existing game folder .* in place/);
 assert.equal(get('choose-adoption-destination').textContent,'Verify and adopt this folder…');

 // 1. The player edited one release file: the review names it, confirm stays
 // disabled even with every box ticked, and nothing in the folder changes.
 await click('choose-adoption-destination');
 let status=await until('the first review',s=>{assert.equal(s.last_error,null,'preparation failed');return !!s.review;});
 assert.equal(status.review.in_place,true);assert.deepEqual(status.review.blocked_files,['later.txt']);
 assert.match(text(),/cannot be adopted in place: later\.txt/);
 assert.equal(get('adoption-source-label').textContent,'Game folder to adopt:');assert.equal(get('adoption-source').textContent,before.folder);
 await tickShown();
 assert.equal(get('confirm-adoption').disabled,true,'a folder that does not verify is never confirmed');
 press('confirm-adoption');await settle();assert.equal(sent('confirm').length,0);
 let now=await evidence();assert.equal(now.owner_marker,false);assert.equal(now.executable_in_old_place,true);
 await click('dismiss-adoption');
 await until('the dismissed review',s=>s.activity==='idle'&&!s.review);

 // 2. Restored, the folder verifies; one confirmation adopts it in place.
 await send({command:'restore'});
 await click('choose-adoption-destination');
 status=await until('the second review',s=>{assert.equal(s.last_error,null,'preparation failed');return !!s.review;});
 assert.deepEqual(status.review.blocked_files,[]);assert.match(text(),/Nothing is moved until you confirm/);
 assert.match(get('adoption-consequences').textContent,/moved, not copied.*old launcher stop working.*Don’t run the old launcher on this folder again.*no longer use them.*live in Documents and are not affected/);
 await tickShown();
 press('confirm-adoption');press('confirm-adoption');await settle();
 assert.equal(sent('confirm').length,1,'a double click confirms once');
 status=await until('publication',s=>s.activity==='idle');
 assert.equal(status.last_error,null);assert.equal(status.completed.directory,before.folder);
 assert.match(text(),/Adopted .* in place.*uninstalling never removes them/);
 assert.equal(get('choose-adoption-destination').disabled,true);
 now=await evidence();
 assert.equal(now.owner_marker,true);assert.equal(now.owned_directory,before.folder);assert.equal(now.uninstall_directory,before.folder);
 assert.equal(now.executable_in_game,true);assert.equal(now.executable_in_old_place,false);
 assert.equal(now.player_file,'untrusted plugin','a file that is not part of the release stays where it was');

 // 3. Uninstalling the adopted folder removes what the launcher manages only.
 await send({command:'uninstall'});
 now=await evidence();
 assert.equal(now.owner_marker,false);assert.equal(now.owned_directory,null);assert.equal(now.executable_in_game,false);
 assert.equal(now.player_file,'untrusted plugin');
 console.log('In-place adoption logic UAT passed: production Effect workflow and view against the production host with a real isolated store, imported old-launcher folder and signed loopback artifact origin.');
 console.log('Exercised: in-place offer text and button; a modified release file named in the review with confirm held disabled and the folder unchanged; dismiss; restored folder re-reviewed unchecked; single confirmation per double click; adoption published with owner marker, owned directory, uninstall target, executable moved into the owned tree and the unlisted file left in place; engine uninstall of the adopted folder keeping the unlisted file.');
 console.log('Excluded: Tauri IPC and the absent folder dialog, the signed production catalog and HTTPS transport, the published client and its RAR/CAB seed, interrupted-move recovery and abandonment (engine tests), the Uninstall view (G-06, LX-26), Play on the adopted folder (LX-26), and packaged visual UAT.');
} finally {await app.dispose();child.stdin.end();assert.equal(await exit,0,'native bridge exits cleanly');await rm(root,{recursive:true,force:true});}
