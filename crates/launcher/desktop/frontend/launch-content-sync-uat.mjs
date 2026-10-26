// Logic UAT for the game data check before Play (LP-24/LP-28). The real launch view runs on linkedom with a
// scripted `launch_command`, so no shell, network, PAK or game process is touched (R9). Each step asserts the
// Play button, its disabled state and both status lines. Prints one line on success; any failure throws (exit 1).
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {parseHTML} from 'linkedom';
import {mountLaunch} from './.test-build/launch-view.mjs';
const html=await readFile(new URL('./ui/index.html',import.meta.url),'utf8');
// A launch status the native side would send: the same shape proton-uat.mjs decodes.
const status=(extra={})=>({schema_version:1,setup_complete:true,resources_available:true,launcher_update_required:false,patches_incomplete:0,ops_skipped:0,installation_id:'owner',observation:null,failure:null,...extra,native:{schema_version:1,requires_reopen:false,adopted_login_servers:null,display_modes:['windowed'],launcher:{version:'0.1.0',tag:null},preferences:{schema_version:2,revision:1,install_directory:'/fixture',launcher_summary_consent:false,login_servers:[],client_patches_enabled:true,display:{mode:'windowed',width:1024,height:768}},operation:{schema_version:1,revision:0,operation:null}}});
// Fixture game data check states; the server name and PAK are made up.
const sync=(phase,extra={})=>({phase,categories_done:0,categories_total:3,bytes_done:0,bytes_total:0,outcome:null,reason:null,server:'fixture-server',failure:null,...extra});
const failure=(extra={})=>({step:'download',reason:'http_error',category_id:null,http_status:500,os_code:null,retry_after_s:null,source_version:null,served_version:null,deadline_s:null,pak:'test.pak',io_kind:null,...extra});
const preparing={phase:'preparing'};
let launchStatus=status();
const plays=[];
// The fake IPC: `inspect` and `play` both answer with the scripted status; each play request is recorded.
const invoke=async(command,{request})=>{
 if(command!=='launch_command')throw new Error(`unexpected ${command}`);
 if(request.command==='play')plays.push(request);
 return structuredClone(launchStatus);
};
const {document,window}=parseHTML(html);
const el=id=>document.getElementById(id);
let uuids=0;
const app=mountLaunch(document,invoke,()=>{},()=>`00000000-0000-4000-8000-${String(++uuids).padStart(12,'0')}`);
const press=()=>el('launch').dispatchEvent(new window.Event('click'));
const settle=async()=>{await app.settled();await new Promise(resolve=>setImmediate(resolve));};
const refresh=async()=>{await app.refresh();await settle();};
// The line under Play says what the check does now; the Play status says what Play did.
const assertLines=(statusText,contentText)=>{
 assert.equal(el('launch-status').textContent,statusText);
 assert.equal(el('launch-content-sync').hidden,contentText===null);
 assert.equal(el('launch-content-sync').textContent,contentText??'');
};
const assertButton=(text,disabled=false)=>{assert.equal(el('launch').textContent,text);assert.equal(el('launch').disabled,disabled);};
const steps=[];
try{
 await app.ready;await settle();
 // 1. First press: Starting… at once, then the check starts on the polled status.
 launchStatus=status({observation:preparing,content_sync:sync('checking')});
 press();
 assertButton('Starting…',true);
 await settle();
 assert.equal(plays.length,1);
 assertButton('Play');
 assert.equal(el('launch-content-sync').textContent,'Checking game data with fixture-server…');
 steps.push('first press Starting…');
 // 2. Polled statuses: checking, downloading 1/3, writing 3/3, committing, then done/synced.
 const stages=[
  [sync('downloading',{categories_done:1,bytes_done:1000000,bytes_total:3000000}),'Updating game data: 1 of 3 categories (1.0 of 3.0 MB)…'],
  [sync('writing',{categories_done:3,bytes_done:3000000,bytes_total:3000000}),'Updating game data: 3 of 3 categories (3.0 of 3.0 MB)…'],
  [sync('committing',{categories_done:3,bytes_done:3000000,bytes_total:3000000}),'Saving updated game data…'],
  [sync('done',{categories_done:3,bytes_done:3000000,bytes_total:3000000,outcome:'synced'}),'Game data updated (3 categories).'],
 ];
 for(const [content,text] of stages){
  launchStatus=status({observation:preparing,content_sync:content});
  await refresh();
  assertButton('Play');
  assertLines('Preparing the game and waiting for its process…',text);
 }
 steps.push('polled checking, downloading, writing, committing, synced');
 // 3. A second Play fails at download with HTTP 500: Retry, the PAK, the server and the not-started line.
 launchStatus=status({observation:{phase:'not_started'},content_sync:sync('done',{outcome:'failed',failure:failure()})});
 await refresh();
 assertButton('Retry');
 press();
 assertButton('Starting…',true);
 await settle();
 assert.equal(plays.length,2);
 assertButton('Retry');
 assertLines('The game was not started because the game data check failed. Press Retry.',
  "Couldn't download test.pak from fixture-server: The server answered with an error (HTTP 500). Press Retry. If it keeps happening, tell the server's operator.\nDetails: download / http_error, HTTP 500");
 steps.push('failed download HTTP 500 with Retry');
 // 4. Retry: Starting… at once, a fresh operation_id, and the run ends up to date.
 const firstOperation=plays[0].operation_id;
 launchStatus=status({observation:preparing,content_sync:sync('done',{categories_done:3,bytes_done:3000000,bytes_total:3000000,outcome:'up_to_date'})});
 press();
 assertButton('Starting…',true);
 await settle();
 assert.equal(plays.length,3);
 assert.notEqual(plays[2].operation_id,plays[1].operation_id);
 assert.notEqual(plays[2].operation_id,firstOperation);
 assertButton('Play');
 assertLines('Preparing the game and waiting for its process…','Game data is up to date.');
 steps.push('Retry Starting… with a fresh operation_id, up to date');
 // 5. pak_in_use with a sharing violation: the file is in use, and the fix names Stargate Worlds.
 launchStatus=status({observation:{phase:'not_started'},content_sync:sync('done',{outcome:'failed',failure:failure({step:'commit',reason:'pak_in_use',http_status:null,os_code:32,io_kind:'sharing_violation'})})});
 await refresh();
 assertButton('Retry');
 assertLines('The game was not started because the game data check failed. Press Retry.',
  'Couldn\'t save the updated test.pak: test.pak is open in another program: the file is in use. Close Stargate Worlds and any tool that has the file open, then press Retry.\nDetails: commit / pak_in_use, Windows error 32');
 steps.push('pak_in_use names the file and Stargate Worlds');
 // 6. The server turned the check off: the note shows and Play reads Play.
 launchStatus=status({content_sync:sync('done',{outcome:'skipped',reason:'disabled'})});
 await refresh();
 assertButton('Play');
 assertLines('Ready to Play.','The server turned off the game data check. Character select may take about a minute.');
 steps.push('skipped disabled shows the note, Play');
 // 7. Skipped on this platform: the element is hidden.
 launchStatus=status({content_sync:sync('done',{outcome:'skipped',reason:'platform_unsupported'})});
 await refresh();
 assertButton('Play');
 assertLines('Ready to Play.',null);
 steps.push('platform_unsupported hidden');
}finally{await app.dispose();}
console.log(`Content sync logic UAT passed: ${steps.length} steps: ${steps.join('; ')}.`);
console.log('Excluded: the real network, the real PAKs, the game, a launcher restart.');
