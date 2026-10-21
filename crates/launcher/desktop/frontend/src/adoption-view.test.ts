import test from 'node:test';
import assert from 'node:assert/strict';
import {parseHTML} from 'linkedom';
import {readFile} from 'node:fs/promises';
import {mountAdoption} from './adoption-view';
import type {AdoptionStatus,Request} from './adoption-workflow';
const html=await readFile(new URL('../ui/index.html',import.meta.url),'utf8');
const native=(revision:number,operation:AdoptionStatus['native']['operation']['operation']=null,preferences=1)=>({schema_version:1 as const,requires_reopen:false,adopted_login_servers:null,launcher:{version:'0.1.0',tag:null},preferences:{schema_version:2 as const,revision:preferences,install_directory:null,launcher_summary_consent:false,login_servers:[],client_patches_enabled:true},operation:{schema_version:1 as const,revision,operation}});
const adopt=(state:'running'|'reconciliation_required'|'succeeded'|'cancelled')=>({id:'op',kind:'adopt' as const,intent_digest:Array(32).fill(0),state});
const idle=(revision=1):AdoptionStatus=>({schema_version:1,native:native(revision),backend:'available',modes:['copy'],imported:{launcher_directory:'/old-launcher',game_directory:'/old-game',blocker:null},activity:'idle',review:null,progress:null,cancellable:false,reconciliation:null,preparations:[],owned:false,completed:null,last_error:null,unsafe_entries:null});
const review=(handle='preview-1'):NonNullable<AdoptionStatus['review']>=>({preview_handle:handle,operation_revision:3,preferences_revision:1,source:'/old-game',destination:'/library/Stargate Worlds',source_kind:'legacy',
 release:{manifest_sha256:'ab'.repeat(32),seed_sha256:'cd'.repeat(32),patches:['first-patch','second-patch']},counts:{matched:40,known_transform:1,modified:2,needs_patching:0,missing:1,extra:3},requires_replacement:true,
 differences:[{path:'Working/Binaries/SGW.exe',source_path:'Working/Binaries/SGW.exe',classification:'known_transform'},{path:'<img src=x onerror=alert(1)>.lua',source_path:'x',classification:'modified'},{path:'mod.dll',source_path:'mod.dll',classification:'extra'}],differences_omitted:4,
 login_servers:[{name:'Second',url:'https://example.invalid/second'},{name:'First',url:'https://example.invalid/first'}],client_patches_enabled:false,
 game_telemetry_opted_in:true,game_telemetry_available:false,requires_normalization:true,requires_telemetry_acceptance:true,user_data_remains_in_source:true,in_place:false,blocked_files:[]});
const reviewing=(handle?:string):AdoptionStatus=>({...idle(3),native:native(3,adopt('running')),activity:'review',review:review(handle)});
type Reply=AdoptionStatus|Promise<AdoptionStatus>;
function mount(reply:(request:Request|'choose')=>Reply,onChange=()=>{}) {
 const {document,window}=parseHTML(html),calls:(Request|'choose')[]=[],modes:string[]=[];
 const app=mountAdoption(document,async(command,args)=>{if(command==='choose_existing_game_folder')modes.push((args as {mode:string}).mode);const request=command==='choose_existing_game_folder'?'choose' as const:(args as {request:Request}).request;calls.push(request);return reply(request);},onChange);
 const get=(id:string)=>document.getElementById(id)!;
 const settle=async()=>{await app.settled();await new Promise(resolve=>setTimeout(resolve,5));await app.settled();};
 return {app,calls,modes,get,settle,sent:(command:string)=>calls.filter(c=>c!=='choose'&&c.command===command) as Request[],
  press:(id:string)=>{get(id).dispatchEvent(new window.Event('click'));},
  click:async(id:string)=>{get(id).dispatchEvent(new window.Event('click'));await settle();},
  tick:async(id:string,checked=true)=>{(get(id) as HTMLInputElement).checked=checked;get(id).dispatchEvent(new window.Event('change'));await settle();},
  text:()=>get('adoption-status').textContent!,disabled:(id:string)=>(get(id) as HTMLButtonElement).disabled};
}
test('review is readable, every required confirmation is visible and unchecked, and confirm is sent once',async()=>{
 let status=reviewing();
 const ui=mount(request=>{if(request!=='choose'&&request.command==='confirm')status={...idle(5),native:native(5,adopt('running')),activity:'copying',cancellable:true,progress:{phase:'copy',current:1,total:44}};return status;});
 try{await ui.app.ready;await ui.settle();
  assert.equal(ui.get('adoption-review').hidden,false);
  assert.equal(ui.get('adoption-source').textContent,'/old-game');assert.equal(ui.get('adoption-destination').textContent,'/library/Stargate Worlds');
  assert.match(ui.get('adoption-release').textContent!,new RegExp(`Signed release ${'ab'.repeat(32)}.*first-patch, second-patch`));
  assert.match(ui.get('adoption-counts').textContent!,/40 files are identical.*1 file differs only.*2 files were modified.*1 file is missing.*3 files are not part/);
  const listed=Array.from(ui.get('adoption-differences').querySelectorAll('li'),li=>li.textContent);
  assert.deepEqual(listed,['Old launcher setup change: Working/Binaries/SGW.exe','Modified: <img src=x onerror=alert(1)>.lua','Not part of the release: mod.dll','…and 4 more differences not listed here.']);
  assert.equal(ui.get('adoption-differences').querySelectorAll('img').length,0,'paths are text, never markup');
  assert.match(ui.get('adoption-consequences').textContent!,/not copied or migrated: they stay in the original folder/);
  assert.match(ui.get('adoption-settings').textContent!,/Second \(https:\/\/example.invalid\/second\); First .*Client patches: off, as imported/);
  assert.match(ui.get('adoption-telemetry').textContent!,/you opted in.*cannot send game telemetry.*diagnostics stay off; adoption does not change/);
  for(const id of ['adoption-normalize','adoption-closed','adoption-telemetry']){
   assert.equal(ui.get(`${id}-row`).hidden,false,`${id} is shown`);assert.equal((ui.get(id) as HTMLInputElement).checked,false);assert.equal(ui.disabled(id),false);
  }
  assert.equal(ui.disabled('confirm-adoption'),true);
  await ui.click('confirm-adoption');assert.equal(ui.sent('confirm').length,0);
  await ui.tick('adoption-closed');await ui.tick('adoption-normalize');assert.equal(ui.disabled('confirm-adoption'),true,'telemetry acceptance is still required');
  await ui.tick('adoption-telemetry');assert.equal(ui.disabled('confirm-adoption'),true,'the replacement box is still required');
  await ui.tick('adoption-replace');assert.equal(ui.disabled('confirm-adoption'),false);
  ui.press('confirm-adoption');await ui.click('confirm-adoption');
  const [sent]=ui.sent('confirm');assert.equal(ui.sent('confirm').length,1);
  assert.deepEqual({...sent,work_id:'id'},{command:'confirm',schema_version:1,work_id:'id',preview_handle:'preview-1',operation_revision:3,preferences_revision:1,normalize_managed_files:true,accept_unavailable_game_telemetry:true,old_game_closed:true,replace_modified_files:true,confirmed:true});
  assert.match(ui.text(),/Copying verified files: 1 of 44/);
  assert.equal(ui.get('adoption-progress').hidden,false);assert.equal(ui.get('cancel-adoption').hidden,false);assert.equal(ui.get('cancel-adoption').textContent,'Cancel copy');
  assert.equal(ui.get('adoption-review').hidden,true);
 }finally{await ui.app.dispose();}
});
test('confirmations that do not apply are hidden and not required',async()=>{
 const status={...reviewing(),review:{...review(),requires_normalization:false,requires_telemetry_acceptance:false,game_telemetry_opted_in:false}};
 const ui=mount(()=>status);
 try{await ui.app.ready;await ui.settle();
  assert.equal(ui.get('adoption-normalize-row').hidden,true);assert.equal(ui.get('adoption-telemetry-row').hidden,true);assert.equal(ui.get('adoption-closed-row').hidden,false);
  await ui.tick('adoption-closed');await ui.tick('adoption-replace');assert.equal(ui.disabled('confirm-adoption'),false);
  await ui.click('confirm-adoption');
  assert.deepEqual(ui.sent('confirm').map(r=>r.command==='confirm'&&[r.normalize_managed_files,r.accept_unavailable_game_telemetry]),[[false,false]]);
 }finally{await ui.app.dispose();}
});
test('preparation shows progress and cancel, and cannot be replaced by choosing again',async()=>{
 let status:AdoptionStatus=idle();
 const ui=mount(request=>{if(request==='choose')status={...idle(3),native:native(3,adopt('running')),activity:'preparing',cancellable:true,progress:{phase:'download',current:1048576,total:4194304}};
  else if(request.command==='cancel')status={...idle(4),native:native(4,adopt('cancelled')),last_error:'cancelled'};return status;});
 try{await ui.app.ready;await ui.settle();
  assert.match(ui.text(),/Choose where to create a separate verified copy/);assert.equal(ui.disabled('choose-adoption-destination'),false);
  assert.equal(ui.get('cancel-adoption').hidden,true);assert.equal(ui.get('adoption-progress').hidden,true);
  ui.press('choose-adoption-destination');
  assert.equal(ui.text(),'Working on verified adoption…','the first press is acknowledged at once');assert.equal(ui.disabled('choose-adoption-destination'),true);
  await ui.settle();
  assert.match(ui.text(),/Downloading the signed reference: 1.0 MB of 4.0 MB/);
  const progress=ui.get('adoption-progress') as HTMLProgressElement;
  assert.equal(progress.hidden,false);assert.equal(progress.value,1048576);assert.equal(progress.max,4194304);
  assert.equal(ui.get('cancel-adoption').hidden,false);assert.equal(ui.disabled('cancel-adoption'),false);assert.equal(ui.get('cancel-adoption').textContent,'Cancel preparation');
  assert.equal(ui.disabled('choose-adoption-destination'),true);
  await ui.click('choose-adoption-destination');assert.equal(ui.calls.filter(c=>c==='choose').length,1);
  await ui.click('cancel-adoption');
  assert.match(ui.text(),/^Cancelled\. The original game folder is unchanged/);
  assert.equal(ui.get('cancel-adoption').hidden,true);assert.equal(ui.disabled('choose-adoption-destination'),false);
 }finally{await ui.app.dispose();}
});
test('a refused or failed press is shown, and a stale review stays on screen unconfirmed',async()=>{
 let status=reviewing(),refuse:string|null='in_use';
 const ui=mount(request=>{if(request!=='choose'&&request.command==='confirm'&&refuse)return Promise.reject(refuse);return status;});
 try{await ui.app.ready;await ui.settle();
  for(const id of ['adoption-closed','adoption-normalize','adoption-telemetry','adoption-replace'])await ui.tick(id);
  await ui.click('confirm-adoption');
  assert.match(ui.text(),/still holds its lock, or the destination already exists/);
  assert.equal(ui.get('adoption-review').hidden,false);assert.equal(ui.disabled('confirm-adoption'),false,'a clear refusal can be corrected and retried by the user');
  // Preferences moved after the review was prepared.
  status={...status,native:native(3,adopt('running'),2)};
  await ui.click('confirm-adoption');
  assert.equal(ui.sent('confirm').length,1,'a stale review is never sent');
  assert.match(ui.text(),/can no longer be confirmed\. Dismiss it and prepare a new one/);
  assert.equal(ui.get('adoption-review').hidden,false);assert.equal(ui.get('adoption-source').textContent,'/old-game');
  assert.equal(ui.disabled('confirm-adoption'),true);assert.equal(ui.disabled('adoption-closed'),true);assert.equal(ui.disabled('dismiss-adoption'),false);
  await ui.click('inspect-adoption');assert.match(ui.text(),/can no longer be confirmed/);assert.equal(ui.disabled('confirm-adoption'),true);
  // A newly prepared review starts unchecked.
  status=reviewing('preview-2');
  await ui.click('inspect-adoption');
  assert.match(ui.text(),/Review the comparison and confirmations below/);
  for(const id of ['adoption-closed','adoption-normalize','adoption-telemetry'])assert.equal((ui.get(id) as HTMLInputElement).checked,false);
 }finally{await ui.app.dispose();}
});
test('a lost confirm reply is reported and never sent again',async()=>{
 let status=reviewing(),lose=true;
 const ui=mount(request=>{if(request!=='choose'&&request.command==='confirm'){status={...idle(6),native:native(6,adopt('succeeded')),owned:true,completed:{directory:'/library/Stargate Worlds'}};if(lose){lose=false;return Promise.reject('transport');}}return status;});
 try{await ui.app.ready;await ui.settle();
  for(const id of ['adoption-closed','adoption-normalize','adoption-telemetry','adoption-replace'])await ui.tick(id);
  await ui.click('confirm-adoption');
  assert.match(ui.text(),/could not be confirmed, and nothing was retried/);
  assert.equal(ui.disabled('confirm-adoption'),true);assert.equal(ui.disabled('choose-adoption-destination'),true);assert.equal(ui.disabled('inspect-adoption'),false);
  await ui.click('confirm-adoption');assert.equal(ui.sent('confirm').length,1);
  await ui.click('inspect-adoption');
  assert.match(ui.text(),/Verified copy created in \/library\/Stargate Worlds/);assert.equal(ui.sent('confirm').length,1);
  assert.equal(ui.get('adoption-review').hidden,true);assert.equal(ui.disabled('choose-adoption-destination'),true);
 }finally{await ui.app.dispose();}
});
test('interrupted work offers only its native recovery, behind an explicit confirmation',async()=>{
 let status:AdoptionStatus={...idle(4),native:native(4,adopt('reconciliation_required')),reconciliation:{kind:'preparation',preparation_id:'prep-1'}};
 const ui=mount(request=>{if(request!=='choose'&&request.command==='abandon_preparation')status={...idle(6),native:native(6,adopt('cancelled'))};
  if(request!=='choose'&&request.command==='recover')status={...idle(8),native:native(8,adopt('succeeded')),completed:{directory:'/copy'}};return status;});
 try{await ui.app.ready;await ui.settle();
  assert.match(ui.text(),/Preparation was interrupted and is never repeated automatically/);
  assert.equal(ui.disabled('choose-adoption-destination'),true);
  assert.equal(ui.get('remove-adoption-preparation').hidden,false);assert.equal(ui.get('recover-adoption').hidden,true);assert.equal(ui.get('abandon-adoption').hidden,true);
  await ui.click('remove-adoption-preparation');
  assert.equal(ui.get('adoption-maintenance').hidden,false);assert.match(ui.get('adoption-maintenance-consequences').textContent!,/Permanently remove the private reference files/);
  assert.equal(ui.sent('abandon_preparation').length,0,'opening the confirmation changes nothing');
  await ui.click('dismiss-adoption-maintenance');assert.equal(ui.get('adoption-maintenance').hidden,true);
  await ui.click('remove-adoption-preparation');await ui.click('confirm-adoption-maintenance');await ui.click('confirm-adoption-maintenance');
  assert.deepEqual(ui.sent('abandon_preparation'),[{command:'abandon_preparation',schema_version:1,preparation_id:'prep-1',operation_revision:4,confirmed:true}]);
  assert.equal(ui.get('adoption-maintenance').hidden,true);assert.equal(ui.disabled('choose-adoption-destination'),false);
  status={...idle(7),native:native(7,adopt('reconciliation_required')),reconciliation:{kind:'copy',operation_id:'copy-1',directory:'/copy',can_recover:true,can_abandon:false}};
  await ui.click('inspect-adoption');
  assert.match(ui.text(),/copy into \/copy was interrupted.*recover to finish publishing/);
  assert.equal(ui.get('recover-adoption').hidden,false);assert.equal(ui.get('abandon-adoption').hidden,true);assert.equal(ui.get('remove-adoption-preparation').hidden,true);
  await ui.click('recover-adoption');assert.match(ui.get('adoption-maintenance-consequences').textContent!,/Nothing is downloaded or copied again/);
  await ui.click('confirm-adoption-maintenance');
  assert.deepEqual(ui.sent('recover'),[{command:'recover',schema_version:1,operation_id:'copy-1',operation_revision:7,confirmed:true}]);
  assert.match(ui.text(),/Verified copy created in \/copy/);
 }finally{await ui.app.dispose();}
});
test('in place: the imported folder is offered, a file that does not verify is named and blocks confirm, then the folder is adopted',async()=>{
 const placed=(status:AdoptionStatus):AdoptionStatus=>({...status,backend:'in_place'});
 const folder=(blocked:string[])=>({...review(),destination:'/old-game',in_place:true,blocked_files:blocked,requires_normalization:false,requires_telemetry_acceptance:false,game_telemetry_opted_in:false});
 let status=placed(idle()),reviews=0;
 const ui=mount(request=>{
  if(request==='choose')status=placed({...reviewing(`preview-${++reviews}`),review:folder(reviews===1?['later.txt']:[])});
  else if(request.command==='dismiss')status=placed(idle(3));
  else if(request.command==='confirm')status=placed({...idle(6),native:native(6,adopt('running')),activity:'copying',cancellable:false,progress:{phase:'copy',current:1,total:3}});
  return status;});
 try{await ui.app.ready;await ui.settle();
  assert.match(ui.text(),/Adopt your existing game folder \/old-game in place.*without copying it/);
  assert.equal(ui.get('adoption-title').textContent,'Use an existing game folder');
  assert.equal(ui.get('choose-adoption-destination').textContent,'Use an existing game folder…');
  await ui.click('choose-adoption-destination');
  assert.match(ui.text(),/cannot be adopted in place: later\.txt\. Restore them/);
  assert.equal(ui.get('adoption-source-label').textContent,'Game folder to adopt:');
  assert.equal(ui.get('adoption-destination-label').textContent,'Launcher-owned installation:');
  assert.equal(ui.get('confirm-adoption').textContent,'Adopt this folder');
  // The owner-approved warning, shown before Confirm.
  const warning=ui.get('adoption-consequences').textContent!;
  for(const pinned of [/SGW\.exe and every release file are moved, not copied, into a “game” folder/,/desktop shortcuts and the old launcher stop working/,/Don’t run the old launcher on this folder again; it may download the game a second time/,/PAKs placed by hand in SourceCache\.en-us, stay where they are and the game will no longer use them/,/settings and the client cache live in Documents and are not affected/])assert.match(warning,pinned);
  await ui.tick('adoption-closed');
  assert.equal(ui.disabled('confirm-adoption'),true,'a folder that does not verify is never confirmed');
  await ui.click('confirm-adoption');assert.equal(ui.sent('confirm').length,0);
  await ui.click('dismiss-adoption');
  // The player restored the file; a new review verifies.
  await ui.click('choose-adoption-destination');
  assert.match(ui.text(),/Nothing is moved until you confirm/);
  assert.equal((ui.get('adoption-closed') as HTMLInputElement).checked,false,'a new review starts unchecked');
  await ui.tick('adoption-closed');await ui.tick('adoption-replace');assert.equal(ui.disabled('confirm-adoption'),false);
  await ui.click('confirm-adoption');assert.equal(ui.sent('confirm').length,1);
  assert.match(ui.text(),/Moving the verified release files into the launcher-owned game folder: 1 of 3\. Nothing is copied/);
  assert.equal(ui.get('cancel-adoption').hidden,true,'admitted moves are never cancelled part-way');
  status=placed({...idle(7),native:native(7,adopt('succeeded')),owned:true,completed:{directory:'/old-game'}});
  await ui.click('inspect-adoption');
  assert.match(ui.text(),/Adopted \/old-game in place.*uninstalling never removes them/);
  status=placed({...idle(8),native:native(8,adopt('reconciliation_required')),reconciliation:{kind:'copy',operation_id:'adopt-1',directory:'/old-game',can_recover:true,can_abandon:true}});
  await ui.click('inspect-adoption');
  assert.match(ui.text(),/in-place adoption of \/old-game was interrupted\. Recover to finish moving.*Abandon to put every release file back/);
  await ui.click('abandon-adoption');
  assert.match(ui.get('adoption-maintenance-consequences').textContent!,/move every release file back where it was/);
 }finally{await ui.app.dispose();}
});
test('other views are refreshed only when a saved revision changes',async()=>{
 let status:AdoptionStatus=idle(1),changes=0;
 const ui=mount(()=>status,()=>{changes++;});
 try{await ui.app.ready;await ui.settle();assert.equal(changes,0,'the first read is a baseline');
  await ui.click('inspect-adoption');await ui.click('inspect-adoption');assert.equal(changes,0);
  status={...idle(1),activity:'preparing',cancellable:true,progress:{phase:'download',current:1,total:2}};
  await ui.click('inspect-adoption');assert.equal(changes,0,'progress alone is not a saved change');
  status=idle(2);await ui.click('inspect-adoption');assert.equal(changes,1);
  await ui.click('inspect-adoption');assert.equal(changes,1);
  status={...idle(2),native:native(2,null,2)};await ui.click('inspect-adoption');assert.equal(changes,2);
 }finally{await ui.app.dispose();}
});
test('unavailable backends and unsupported imported settings are stated plainly',async()=>{
 for(const [status,expected] of [
  [{...idle(),backend:'unsupported_platform'},/not available on this operating system yet\. Keep using the old launcher/],
  [{...idle(),backend:'helper_unavailable'},/does not include the verified archive helper/],
  [{...idle(),backend:'in_place'},/Adopt your existing game folder \/old-game in place/],
  [{...idle(),backend:'in_place',last_error:'unsupported_layout'},/older layout.*cannot adopt in place.*Install the game fresh/],
  [{...idle(),backend:'in_place',last_error:'game_running'},/game is running from this folder\. Close the game first/],
  [{...idle(),backend:'in_place',last_error:'duplicate_release_file'},/neither copy matches.*log names the file\. Nothing was deleted/],
  [{...idle(),backend:'in_place',last_error:'leftover_launcher_files'},/already holds launcher files.*cannot be adopted/],
  [{...idle(),imported:{...idle().imported!,blocker:'unsupported_catalog'}},/custom content catalog.*will not rewrite your settings/],
  [{...idle(),imported:{...idle().imported!,blocker:'unsupported_configuration'}},/custom patch DLL.*will not silently drop/],
  [{...idle(),imported:null},/Choose your existing game folder to start/],
  [{...idle(),owned:true},/desktop-owned installation already exists/],
  [{...idle(),last_error:'invalid_artifact'},/did not match the signed release and was discarded/],
  [{...idle(),last_error:'network'},/could not be downloaded/],
 ] as [AdoptionStatus,RegExp][]){
  const ui=mount(()=>status);
  try{await ui.app.ready;await ui.settle();assert.match(ui.text(),expected);
   assert.equal(ui.disabled('choose-adoption-destination'),!['available','in_place'].includes(status.backend)||status.owned);
  }finally{await ui.app.dispose();}
 }
});
test('a failed read is shown and disables every mutation until a recheck succeeds',async()=>{
 let fail=true;
 const ui=mount(()=>fail?Promise.reject('transport'):reviewing());
 try{await ui.app.ready;await ui.settle();
  assert.match(ui.text(),/could not be confirmed/);
  for(const id of ['choose-adoption-destination','confirm-adoption','cancel-adoption'])assert.equal(ui.disabled(id),true);
  fail=false;await ui.click('inspect-adoption');
  assert.match(ui.text(),/Review the comparison/);
 }finally{await ui.app.dispose();}
});
test('a folder source with files that need patching shows its review, backup and download',async()=>{
 const folder:NonNullable<AdoptionStatus['review']>={...review(),source_kind:'folder',backup_directory:'/old-game.backup',download:{seed:true,patches:['first-patch'],bytes:1024},counts:{...review().counts,needs_patching:2}};
 const ui=mount(()=>({...reviewing(),modes:['in_place','copy'],review:folder}));
 try{await ui.app.ready;await ui.settle();
  assert.equal(ui.get('adoption-review').hidden,false);
  assert.equal(ui.get('adoption-source').textContent,'/old-game');assert.equal(ui.get('adoption-destination').textContent,'/library/Stargate Worlds');
 }finally{await ui.app.dispose();}
});
test('a review whose seed comes from the chosen game archive says so, with its patches',async()=>{
 const archive=(patches:string[]):NonNullable<AdoptionStatus['review']>=>({...review(),source_kind:'folder',download:{seed:false,patches,bytes:1048576,local_seed:true}});
 for(const [patches,line] of [[['first-patch'],'Uses your game archive and patches first-patch (1.0 MB).'],[[],'Uses your game archive.']] as const){
  const ui=mount(()=>({...reviewing(),modes:['in_place','copy'],review:archive([...patches])}));
  try{await ui.app.ready;await ui.settle();
   assert.equal(ui.get('adoption-download').textContent,line);assert.equal(ui.get('adoption-seed-archive').hidden,true);
  }finally{await ui.app.dispose();}
 }
});
test('a folder review without imported settings decodes and says the launcher defaults apply',async()=>{
 // A folder preview has no legacy import, so the requested-config fields are absent.
 const imported=['login_servers','client_patches_enabled','game_telemetry_opted_in'];
 const folder=Object.fromEntries(Object.entries({...review(),source_kind:'folder'}).filter(([key])=>!imported.includes(key))) as NonNullable<AdoptionStatus['review']>;
 const ui=mount(()=>({...reviewing(),review:folder}));
 try{await ui.app.ready;await ui.settle();
  assert.equal(ui.get('adoption-review').hidden,false);
  assert.match(ui.get('adoption-settings').textContent!,/^Settings: this launcher's own server list/);
  assert.match(ui.get('adoption-telemetry').textContent!,/^Game telemetry: launcher defaults/);
  assert.doesNotMatch(ui.get('adoption-settings').textContent!,/Login servers/);
 }finally{await ui.app.dispose();}
});
test('the replacement box starts unchecked and Confirm stays disabled until it is ticked',async()=>{
 const status={...reviewing(),review:{...review(),requires_normalization:false,requires_telemetry_acceptance:false}};
 const ui=mount(()=>status);
 try{await ui.app.ready;await ui.settle();
  assert.equal(ui.get('adoption-replace-row').hidden,false);assert.equal((ui.get('adoption-replace') as HTMLInputElement).checked,false,'unchecked by default');
  await ui.tick('adoption-closed');assert.equal(ui.disabled('confirm-adoption'),true,'the replacement box is still required');
  await ui.tick('adoption-replace');assert.equal(ui.disabled('confirm-adoption'),false);
 }finally{await ui.app.dispose();}
});
test('a folder review imports nothing: only the launcher’s own settings line is shown',async()=>{
 // A folder review carries none of the imported settings fields.
 const folder:Record<string,unknown>={...review()};for(const key of ['login_servers','client_patches_enabled','game_telemetry_opted_in'])delete folder[key];
 const status={...reviewing(),review:{...folder,source_kind:'folder',requires_replacement:false,download:{seed:false,patches:['first-patch'],bytes:1048576}} as unknown as NonNullable<AdoptionStatus['review']>};
 const ui=mount(()=>status);
 try{await ui.app.ready;await ui.settle();
  assert.equal(ui.get('adoption-settings').textContent,'Settings: this launcher\'s own server list and client-patch setting.');
  assert.doesNotMatch(ui.get('adoption-settings').textContent!,/Second|First|Client patches: off/);
  assert.doesNotMatch(ui.get('adoption-telemetry').textContent!,/you opted in/);
  assert.equal(ui.get('adoption-replace-row').hidden,true);assert.equal(ui.get('adoption-download').textContent,'Downloads: patches first-patch (1.0 MB).');
  assert.equal(ui.get('adoption-seed-archive').hidden,true);
 }finally{await ui.app.dispose();}
});
test('each new failure code renders its own text',async()=>{
 const expected:[AdoptionStatus['last_error'],RegExp][]=[['folder_not_writable',/cannot write to this folder/],['not_a_game_folder',/not a Stargate Worlds game folder/],['backup_conflict',/backup folder for replaced files already exists/],['unsupported_mode',/adoption mode is not available/]];
 for(const [code,text] of expected){
  const ui=mount(()=>({...idle(),last_error:code}));
  try{await ui.app.ready;await ui.settle();assert.match(ui.text(),text,code as string);}finally{await ui.app.dispose();}
 }
});
test('the mode choice is one row per mode, shows the pick, and the button names it',async()=>{
 const both=mount(()=>({...idle(),modes:['in_place','copy'] as AdoptionStatus['modes']})),one=mount(()=>idle());
 try{await both.app.ready;await both.settle();await one.app.ready;await one.settle();
  assert.equal(both.get('adoption-modes').hidden,false);assert.equal(one.get('adoption-modes').hidden,true);
  // The labels hold line breaks in the markup, so the match spans them.
  assert.match(both.get('adoption-modes').textContent!,/Adopt it in place.*becomes the launcher’s installation[\s\S]*Make a verified copy[\s\S]*left as it is/);
  assert.match(both.get('adoption-mode-in-place-row').className,/\bselected\b/);assert.doesNotMatch(both.get('adoption-mode-copy-row').className,/\bselected\b/);
  assert.equal(both.get('choose-adoption-destination').textContent,'Choose the folder to adopt in place…');
  // linkedom does not uncheck the other radio, so both are set explicitly.
  await both.tick('adoption-mode-in-place',false);await both.tick('adoption-mode-copy');
  assert.doesNotMatch(both.get('adoption-mode-in-place-row').className,/\bselected\b/);assert.match(both.get('adoption-mode-copy-row').className,/\bselected\b/);
  assert.equal(both.get('choose-adoption-destination').textContent,'Choose the folder to copy…');
  await both.click('choose-adoption-destination');
  assert.deepEqual(both.modes,['copy']);
  assert.equal(one.get('choose-adoption-destination').textContent,'Use an existing game folder…');
 }finally{await both.app.dispose();await one.app.dispose();}
});
test('the mode rows are laid out one per line',async()=>{
 const css=await readFile(new URL('../ui/style.css',import.meta.url),'utf8');
 assert.ok(css.includes('.mode-option{display:flex'));assert.ok(css.includes('.mode-option.selected{'));
});
test('every button shows busy on the first press',async()=>{
 let release!:()=>void;const gate=new Promise<void>(resolve=>{release=resolve;});
 const status=reviewing();
 const ui=mount(async request=>{if(request!=='choose'&&request.command==='dismiss')await gate;return status;});
 try{await ui.app.ready;await ui.settle();
  ui.press('dismiss-adoption');
  assert.equal(ui.text(),'Working on verified adoption…','the first press is acknowledged at once');
  for(const id of ['dismiss-adoption','confirm-adoption','inspect-adoption','choose-adoption-destination','dismiss-adoption-maintenance'])assert.equal(ui.disabled(id),true,id);
  release();await ui.settle();
 }finally{release();await ui.app.dispose();}
});