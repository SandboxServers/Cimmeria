// The light adoption view (LA-18) driven REPL-style, as adoption-in-place-uat.mjs drives
// its view: the choose press is answered by canned native replies, shaped as the shell
// returns them after LA-15, and the texts and enabled controls are asserted. No native host runs here.
import {readFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
import {parseHTML} from 'linkedom';
import {mountAdoption} from './.test-build/adoption-view.mjs';
import {mountLaunch} from './.test-build/launch-view.mjs';

const native={schema_version:1,requires_reopen:false,adopted_login_servers:null,launcher:{version:'0.1.0',tag:null},preferences:{schema_version:2,revision:1,install_directory:null,launcher_summary_consent:false,login_servers:[],client_patches_enabled:true},operation:{schema_version:1,revision:1,operation:null}};
const idle=()=>({schema_version:1,native,backend:'in_place',modes:['light','copy'],imported:{launcher_directory:'/old-launcher',game_directory:'/old-game',blocker:null},activity:'idle',review:null,progress:null,cancellable:false,reconciliation:null,preparations:[],owned:false,completed:null,last_error:null,last_rejection:null,unsafe_entries:null});
// A light review of a clean stock folder; the scenarios override the index and the skipped changes.
const review=(light)=>({preview_handle:'light-uat',operation_revision:3,preferences_revision:1,source:'/games/SGW',destination:'/games/SGW',source_kind:'folder',
 release:{manifest_sha256:'ab'.repeat(32),seed_sha256:'cd'.repeat(32),patches:['first-patch']},counts:{matched:40,known_transform:0,modified:0,needs_patching:0,missing:0,extra:0},requires_replacement:false,
 differences:[],differences_omitted:0,login_servers:[],client_patches_enabled:false,game_telemetry_opted_in:true,game_telemetry_available:true,requires_normalization:false,requires_telemetry_acceptance:false,user_data_remains_in_source:false,in_place:true,blocked_files:[],
 light:{patches_total:1,patches_current:1,patches_pending:0,download_bytes:0,skipped:[],skipped_total:0,index_state:'compared',index_counts:{stock:40},index_files:[],index_files_total:0,...light}});
const reviewed=(light)=>({...idle(),activity:'review',review:review(light)});
const cancelled=()=>idle();
const twoInstalls=()=>({...idle(),last_error:'not_a_game_folder',last_rejection:{check:'several_game_folders',entry:'.',expected:'one game folder',found:'2',http_status:null}});

const html=await readFile(new URL('./ui/index.html',import.meta.url),'utf8');let document,window,app;
// The reply the next choose press gets; the other requests get the idle status.
let next=idle();
const fresh=async()=>{if(app)await app.dispose();({document,window}=parseHTML(html));app=mountAdoption(document,async(command)=>command==='choose_existing_game_folder'?next:idle());await app.ready;await settle();};
const get=id=>document.getElementById(id),text=()=>get('adoption-status').textContent;
const pause=ms=>new Promise(resolve=>setTimeout(resolve,ms));
const settle=async()=>{await app.settled();await pause(10);await app.settled();};
const press=id=>get(id).dispatchEvent(new window.Event('click'));
const choose=async reply=>{next=reply;assert.equal(get('choose-adoption-destination').disabled,false,'the choose button is enabled');press('choose-adoption-destination');await settle();};
const scenario=async reply=>{await fresh();await choose(reply);};
const enabled=id=>assert.equal(get(id).disabled,false,`${id} is enabled`);
// The confirmations the review shows are ticked as the player would, then confirm must be enabled.
const confirmable=async()=>{for(const id of ['adoption-normalize','adoption-closed','adoption-telemetry'])if(!get(`${id}-row`).hidden){get(id).checked=true;get(id).dispatchEvent(new window.Event('change'));}await settle();enabled('confirm-adoption');};

try {
 await fresh();

 // 1. A clean stock folder: every file is original, nothing is skipped, confirm is enabled.
 await scenario(reviewed({index_counts:{stock:40}}));
 assert.equal(get('adoption-index').textContent,'Compared with the published file list: 40 original, 0 already patched, 0 prepared by the launcher, 0 changed, 0 missing, 0 extra.');
 assert.equal(get('adoption-index-files').hidden,true);
 assert.equal(get('adoption-light-skipped-note').hidden,true);
 await confirmable();
 console.log('ok: a clean stock folder names its counts and leaves confirm and Play enabled');

 // 2. Skipped changes: the note and the list name them, and confirm stays enabled.
 await scenario(reviewed({skipped_total:2,skipped:[{path:'Working/Data/a.lua',patch:'first-patch',reason:'modified'},{path:'Working/Data/b.lua',patch:'first-patch',reason:'modified'}]}));
 assert.equal(get('adoption-light-skipped-note').hidden,false);
 assert.match(get('adoption-light-skipped-note').textContent,/Some patch changes can't be applied: 2 files differ/);
 await confirmable();
 console.log('ok: skipped changes are named and do not block confirm');

 // 3. Each index state: its sentence, and no files list.
 const states=[
  ['index_not_published','No published file list to compare against yet.'],
  ['index_unreachable','The file list could not be downloaded; the folder was not compared.'],
  ['index_invalid','The published file list failed its check; the folder was not compared.'],
 ];
 for(const [index_state,sentence] of states){
  await scenario(reviewed({index_state,index_counts:{}}));
  assert.equal(get('adoption-index').textContent,sentence);
  assert.equal(get('adoption-index-files').hidden,true);
  await confirmable();
 }
 console.log('ok: each index state says why the folder was not compared, and does not block confirm');

 // 4. A picked folder holding two installs: the rejection is shown, the choose button is enabled on the next press, and a cancel says so.
 await scenario(twoInstalls());
 assert.match(text(),/did not pick one/);
 assert.match(get('adoption-rejection').textContent,/more than one Stargate Worlds install/);
 assert.equal(get('adoption-rejection').hidden,false);
 await choose(cancelled());
 assert.equal(text(),'No folder chosen.');
 console.log('ok: two installs are rejected with the rejection line; a cancelled picker says No folder chosen.');
} finally {await app.dispose();}

// 5. An install with incomplete patches: Play stays enabled and the main view names the skipped changes.
const launchDocument=parseHTML(await readFile(new URL('./ui/index.html',import.meta.url),'utf8')).document;
const status={schema_version:1,resources_available:true,launcher_update_required:false,patches_incomplete:1,ops_skipped:3,installation_id:'owner',observation:null,native};
const launch=mountLaunch(launchDocument,async()=>status,()=>{},()=>'attempt');
try {
 await launch.ready;await pause(10);
 assert.match(launchDocument.getElementById('launch-status').textContent,/3 patch changes could not be applied/);
 assert.equal(launchDocument.getElementById('launch').disabled,false,'Play is enabled');
 console.log('ok: an install with incomplete patches names the skipped changes and leaves Play enabled');
} finally {await launch.dispose();}
console.log('Light adoption view UAT passed: canned native replies against the production adoption and launch views.');
