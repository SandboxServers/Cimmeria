import test from 'node:test';
import assert from 'node:assert/strict';
import {parseHTML} from 'linkedom';
import {readFile} from 'node:fs/promises';
import {failureText,mountAdoption,unsafeItems} from './adoption-view';
import type {AdoptionStatus,Request} from './adoption-workflow';
const html=await readFile(new URL('../ui/index.html',import.meta.url),'utf8');
const native=(revision:number,operation:AdoptionStatus['native']['operation']['operation']=null,preferences=1)=>({schema_version:1 as const,requires_reopen:false,adopted_login_servers:null,launcher:{version:'0.1.0',tag:null},preferences:{schema_version:2 as const,revision:preferences,install_directory:null,launcher_summary_consent:false,login_servers:[],client_patches_enabled:true},operation:{schema_version:1 as const,revision,operation}});
const adopt=(state:'running'|'reconciliation_required'|'succeeded'|'cancelled')=>({id:'op',kind:'adopt' as const,intent_digest:Array(32).fill(0),state});
const idle=(revision=1):AdoptionStatus=>({schema_version:1,native:native(revision),backend:'available',modes:['copy'],imported:{launcher_directory:'/old-launcher',game_directory:'/old-game',blocker:null},activity:'idle',review:null,progress:null,cancellable:false,reconciliation:null,preparations:[],owned:false,completed:null,last_error:null,last_rejection:null,unsafe_entries:null});
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
  assert.equal(ui.text(),'Checking your game folder…','the first press is acknowledged at once');assert.equal(ui.disabled('choose-adoption-destination'),true);
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
  assert.match(ui.text(),/Adopted \/old-game in place.*uninstalling never removes them.*This folder is now your Cimmeria install\. If the game will not start, use Repair \(it downloads the full game, 4\.1 GB\)\./);
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
  [{...idle(),backend:'in_place',last_error:'duplicate_release_file'},/neither copy matches.*recovery stopped\. Nothing was deleted/],
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
 const expected:[AdoptionStatus['last_error'],RegExp][]=[['folder_not_writable',/cannot write to this folder/],['not_a_game_folder',/not a Stargate Worlds game folder/],['backup_conflict',/backup folder for replaced files already exists/],['unsupported_mode',/adoption mode is not available/],['import_required',/Import the old launcher’s settings first/]];
 for(const [code,text] of expected){
  const ui=mount(()=>({...idle(),last_error:code}));
  try{await ui.app.ready;await ui.settle();assert.match(ui.text(),text,code as string);assert.equal(ui.get('adoption-rejection').hidden,true);}finally{await ui.app.dispose();}
 }
 // A worker refusal is shown under the failure text: its check in words, and the file it named.
 const ui=mount(()=>({...idle(),last_error:'folder_not_writable',last_rejection:{check:'folder_not_writable',entry:'Stargate Worlds',expected:null,found:null,http_status:null}}));
 try{await ui.app.ready;await ui.settle();assert.match(ui.text(),/cannot write to this folder/);
  assert.equal(ui.get('adoption-rejection').textContent,'The launcher cannot write to this folder, so it cannot adopt or copy into it. File: Stargate Worlds.');
  assert.equal(ui.get('adoption-rejection').hidden,false);
 }finally{await ui.app.dispose();}
});
test('a worker refusal line is hidden while a review is shown',async()=>{
 const rejected={check:'folder_not_writable' as const,entry:'Stargate Worlds',expected:null,found:null,http_status:null};
 const ui=mount(()=>({...reviewing(),last_error:'folder_not_writable',last_rejection:rejected}));
 try{await ui.app.ready;await ui.settle();assert.equal(ui.get('adoption-review').hidden,false);assert.equal(ui.get('adoption-rejection').hidden,true);
 }finally{await ui.app.dispose();}
});
test('a worker refusal line is hidden after a synchronous error replaces the status text',async()=>{
 const rejected={check:'folder_not_writable' as const,entry:'Stargate Worlds',expected:null,found:null,http_status:null};
 let refuse=false;
 const ui=mount(request=>{if(request!=='choose'&&request.command==='inspect'&&refuse)return Promise.reject('busy');return {...idle(),last_error:'folder_not_writable',last_rejection:rejected};});
 try{await ui.app.ready;await ui.settle();assert.equal(ui.get('adoption-rejection').hidden,false);
  refuse=true;await ui.click('inspect-adoption');
  assert.equal(ui.get('adoption-rejection').hidden,true);
 }finally{await ui.app.dispose();}
});
test('an unknown failure code names its code instead of the platform text',()=>{
 assert.equal(failureText('future_code'),'Adoption stopped: future_code.');
});
test('a preparing download says why the archive is downloaded, for the seed, a patch or the built index',async()=>{
 for(const [item,phase,text] of [
  ['seed','download',/^Downloading the signed game archive to check your files \(no usable file index\)\.$/],
  ['patch','download',/^Downloading the signed game archive to check your files/],
  ['index_build','extraction',/^Downloading the signed game archive to check your files/],
  ['file_index','download',/^Downloading the signed reference: /],
  ['scan','extraction',/^Scanning your game folder: /],
 ] as const){
  const ui=mount(()=>({...idle(),activity:'preparing',progress:{phase,current:0,total:1,item}}));
  try{await ui.app.ready;await ui.settle();assert.match(ui.text(),text,item);}finally{await ui.app.dispose();}
 }
});
test('a review without a published file index says the launcher downloads the archive',async()=>{
 const ui=mount(()=>({...reviewing(),review:{...review(),index_fallback:{reason:'index_not_published',check:null,http_status:null,seed_bytes:4_100_000_000}}}));
 try{await ui.app.ready;await ui.settle();
  assert.equal(ui.get('adoption-index-fallback').hidden,false);
  assert.equal(ui.get('adoption-index-fallback').textContent,'This release has no published file index, so the launcher downloads the signed game archive (4.1 GB) to check your files. To skip that, use your game archive.');
 }finally{await ui.app.dispose();}
});
test('the status decoder accepts a rejection, a progress item and an index fallback',async()=>{
 const ui=mount(()=>({...reviewing(),last_rejection:{check:'download_http_status',entry:'seed',expected:null,found:null,http_status:500},progress:{phase:'download',current:1,total:2,item:'seed'},review:{...review(),index_fallback:{reason:'index_not_published',check:null,http_status:null,seed_bytes:1}}}));
 try{await ui.app.ready;await ui.settle();
  // A status the schema rejects shows the unconfirmed-outcome text instead of the review.
  assert.doesNotMatch(ui.text(),/could not be confirmed/);
 }finally{await ui.app.dispose();}
});
test('the mode choice is one row per mode, shows the pick, and the button names it',async()=>{
 const both=mount(()=>({...idle(),modes:['in_place','copy'] as AdoptionStatus['modes']})),one=mount(()=>idle());
 try{await both.app.ready;await both.settle();await one.app.ready;await one.settle();
  assert.equal(both.get('adoption-modes').hidden,false);assert.equal(one.get('adoption-modes').hidden,true);
  // The labels hold line breaks in the markup, so the match spans them.
  assert.match(both.get('adoption-modes').textContent!,/Use it where it is \(recommended\)[\s\S]*Its game files move into a “game” folder[\s\S]*Make a verified copy[\s\S]*left as it is/);
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
test('with copy unticked, the first offered mode is the one chosen (light, not in place)',async()=>{
 const ui=mount(()=>({...idle(),modes:['light','copy'] as AdoptionStatus['modes']}));
 try{await ui.app.ready;await ui.settle();
  // Copy is never ticked here, so the press chooses the first offered mode.
  await ui.click('choose-adoption-destination');
  assert.deepEqual(ui.modes,['light']);
 }finally{await ui.app.dispose();}
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
test('an unsafe refusal lists each entry, its reason and the fix',async()=>{
 const status:AdoptionStatus={...idle(),backend:'in_place',modes:['in_place','copy'],last_error:'unsafe_file',unsafe_entries:{entries:[{reason:'hard_link',path:'later.txt'},{reason:'cloud_file',path:'Working/a.pak'},{reason:'link',path:'.'},{reason:'link',path:null},{reason:'name',path:'<img src=x onerror=alert(1)>'}],total:7}};
 const ui=mount(()=>status);
 try{await ui.app.ready;await ui.settle();
  assert.equal(ui.text(),'These files or folders in the game folder cannot be adopted safely. Fix each one as described below, then choose the folder again. Nothing was copied, moved or removed.');
  const list=ui.get('adoption-unsafe');
  assert.equal(list.hidden,false);
  const items=Array.from(list.querySelectorAll('li'),li=>li.textContent??'');
  assert.equal(items.length,6);
  assert.equal(items[0],'later.txt: a hard link: the same file is stored under another name too. Replace it with a plain copy, or remove it, then try again.');
  assert.match(items[2],/^The game folder itself:/);
  assert.match(items[3],/^The game folder’s location:/);
  assert.equal(items[5],'…and 2 more not listed here.');
  assert.equal(list.querySelector('img'),null);
 }finally{await ui.app.dispose();}
});
test('every unsafe reason has its own fix text',()=>{
 const reasons=['link','cloud_file','hard_link','special_file','name','case_collision'] as const;
 const texts=reasons.map(reason=>{const items=unsafeItems({...idle(),unsafe_entries:{entries:[{reason,path:'x'}],total:1}});assert.equal(items.length,1);return items[0];});
 assert.equal(new Set(texts).size,6);
 for(const text of texts)assert.match(text,/^x: \S/);
});
test('an unsafe refusal without entries shows the generic text and no list',async()=>{
 const ui=mount(()=>({...idle(),last_error:'unsafe_file',unsafe_entries:null}));
 try{await ui.app.ready;await ui.settle();
  assert.match(ui.text(),/could not be checked safely, or it changed while it was checked.*Close the game and any program that syncs or backs up the folder/);
  assert.equal(ui.get('adoption-unsafe').hidden,true);
 }finally{await ui.app.dispose();}
});
test('the list is gone once a review is shown',async()=>{
 const ui=mount(()=>({...reviewing(),last_error:'unsafe_file',unsafe_entries:{entries:[{reason:'link',path:'x'}],total:1}}));
 try{await ui.app.ready;await ui.settle();
  assert.equal(ui.get('adoption-unsafe').hidden,true);
 }finally{await ui.app.dispose();}
});
const lightReview=(skipped_total=1):NonNullable<AdoptionStatus['review']>=>({...review(),source:'/games/SGW',in_place:true,source_kind:'folder',requires_replacement:false,requires_normalization:false,requires_telemetry_acceptance:false,blocked_files:[],
 light:{patches_total:3,patches_current:1,patches_pending:2,download_bytes:1048576,skipped:[{path:'Working/Data/a.lua',patch:'first-patch',reason:'modified'}],skipped_total,index_state:'compared',index_counts:{},index_files:[],index_files_total:0}});
test('the light review names the folder, its patch counts and the skipped changes, and Use this folder stays enabled',async()=>{
 const ui=mount(request=>request==='choose'?{...reviewing('light-1'),backend:'in_place',modes:['light','copy'],review:lightReview()}:idle());
 try{await ui.app.ready;await ui.settle();
  await ui.click('choose-adoption-destination');
  assert.equal(ui.get('adoption-light-found').textContent,'Stargate Worlds found in SGW.');assert.equal(ui.get('adoption-light-found').hidden,false);
  assert.equal(ui.get('adoption-light-patches').textContent,'Patches already applied: 1 of 3. To apply: 2 (1.0 MB to download).');
  assert.equal(ui.get('adoption-light-skipped-note').hidden,false);
  assert.match(ui.get('adoption-light-skipped-note').textContent!,/Some patch changes can't be applied: 1 file differs from the originals the patches expect\. They stay as they are\. You can still use this folder\./);
  assert.match(ui.get('adoption-light-skipped').textContent!,/Working\/Data\/a\.lua — first-patch/);
  assert.equal(ui.get('confirm-adoption').textContent,'Use this folder');
  await ui.tick('adoption-closed');assert.equal(ui.disabled('confirm-adoption'),false,'skipped changes never disable the confirm button');
 }finally{await ui.app.dispose();}
});
test('a light review with no skipped changes hides the skipped note and list',async()=>{
 const ui=mount(request=>request==='choose'?{...reviewing('light-2'),backend:'in_place',modes:['light','copy'],review:lightReview(0)}:idle());
 try{await ui.app.ready;await ui.settle();
  await ui.click('choose-adoption-destination');
  assert.equal(ui.get('adoption-light-skipped-note').hidden,true);assert.equal(ui.get('adoption-light-skipped').hidden,true);
  assert.equal(ui.get('adoption-light-found').hidden,false);
 }finally{await ui.app.dispose();}
});
test('a light review says every game file moves into game and stays in use',async()=>{
 const counts={matched:40,known_transform:2,modified:0,needs_patching:3,missing:0,extra:0};
 const ui=mount(request=>request==='choose'?{...reviewing('light-5'),backend:'in_place',modes:['light','copy'],review:{...lightReview(0),counts}}:idle());
 try{await ui.app.ready;await ui.settle();
  await ui.click('choose-adoption-destination');
  assert.equal(ui.get('adoption-counts').textContent,'40 files move into the “game” folder with no patch changes. 3 files are updated by the patches. 2 bundled data files move to the folder the game loads them from.');
  assert.equal(ui.get('adoption-consequences').textContent,'Your folder becomes the launcher’s installation. Its game files, including files you added such as PAKs placed by hand in SourceCache.en-us, are moved, not copied, into a “game” folder inside it, and the game keeps using them. Files that differ from what the patches expect are kept as they are. Desktop shortcuts and the old launcher stop working: start the game from this launcher afterwards. Don’t run the old launcher on this folder again; it may download the game a second time. Your settings and the client cache live in Documents and are not affected.');
 }finally{await ui.app.dispose();}
});
test('a completed light adoption says the added files moved with it, not that they were left behind',async()=>{
 const done:AdoptionStatus={...idle(9),native:native(9,adopt('succeeded')),backend:'in_place',modes:['light','copy'],owned:true,completed:{directory:'/games/SGW',light:true}};
 const ui=mount(()=>done);
 try{await ui.app.ready;await ui.settle();
  assert.equal(ui.text(),'Adopted /games/SGW in place. Its game files, including files you added, now sit in the launcher-owned “game” folder inside it, and the game keeps using them. This folder is now your Cimmeria install. If the game will not start, use Repair (it downloads the full game, 4.1 GB).');
  assert.doesNotMatch(ui.text(),/left where they were/);
 }finally{await ui.app.dispose();}
});
test('a light review with more skipped changes than the list shows says how many are not listed',async()=>{
 const ui=mount(request=>request==='choose'?{...reviewing('light-4'),backend:'in_place',modes:['light','copy'],review:lightReview(600)}:idle());
 try{await ui.app.ready;await ui.settle();
  await ui.click('choose-adoption-destination');
  assert.match(ui.get('adoption-light-skipped').textContent!,/…and 599 more not listed here\./);
  assert.match(ui.get('adoption-light-skipped-note').textContent!,/Some patch changes can't be applied: 600 files differ from the originals/);
 }finally{await ui.app.dispose();}
});
test('a copy review has no light lines and keeps Create verified copy',async()=>{
 const ui=mount(request=>request==='choose'?{...reviewing('copy-1'),backend:'available',modes:['copy'],review:review('copy-1')}:idle());
 try{await ui.app.ready;await ui.settle();
  await ui.click('choose-adoption-destination');
  assert.equal(ui.get('adoption-light-found').hidden,true);assert.equal(ui.get('adoption-light-patches').hidden,true);
  assert.notEqual(ui.get('confirm-adoption').textContent,'Use this folder');
 }finally{await ui.app.dispose();}
});
test('the first press says the folder is being checked before the preview resolves, and a cancelled picker says No folder chosen.',async()=>{
 let finish:(s:AdoptionStatus)=>void=()=>{};
 const ui=mount(request=>request==='choose'?new Promise<AdoptionStatus>(resolve=>{finish=resolve;}):idle());
 try{await ui.app.ready;await ui.settle();
  ui.press('choose-adoption-destination');
  assert.equal(ui.text(),'Checking your game folder…');
  finish({...reviewing('light-3'),backend:'in_place',modes:['light','copy'],review:lightReview()});
  await ui.settle();
  assert.notEqual(ui.text(),'Checking your game folder…');
 }finally{await ui.app.dispose();}
 const cancelled=mount(request=>request==='choose'?idle():idle());
 try{await cancelled.app.ready;await cancelled.settle();
  await cancelled.click('choose-adoption-destination');
  assert.equal(cancelled.text(),'No folder chosen.');
 }finally{await cancelled.app.dispose();}
});
test('a cancel notice is cleared by the next press',async()=>{
 let n=0;
 const ui=mount(request=>request==='choose'?(n++===0?idle():{...reviewing('again'),backend:'in_place',modes:['light','copy'],review:lightReview()}):idle());
 try{await ui.app.ready;await ui.settle();
  await ui.click('choose-adoption-destination');assert.equal(ui.text(),'No folder chosen.');
  await ui.click('choose-adoption-destination');assert.notEqual(ui.text(),'No folder chosen.');
 }finally{await ui.app.dispose();}
});
test('several game folders on the choose path: the status and the refusal line name the install, and choose is enabled again',async()=>{
 const ui=mount(request=>request==='choose'?{...idle(),backend:'in_place',modes:['light','copy'] as AdoptionStatus['modes'],last_error:'not_a_game_folder',last_rejection:{check:'several_game_folders',entry:'.',expected:'one game folder',found:'2',http_status:null}}:idle());
 try{await ui.app.ready;await ui.settle();
  await ui.click('choose-adoption-destination');
  assert.match(ui.text(),/more than one Stargate Worlds install/);assert.doesNotMatch(ui.text(),/is not a Stargate Worlds game folder/);
  assert.equal(ui.get('adoption-rejection').hidden,false);
  assert.equal(ui.get('adoption-rejection').textContent,'This folder holds more than one Stargate Worlds install. Pick the one you want. Expected one game folder, found 2.');
  assert.equal(ui.disabled('choose-adoption-destination'),false);
 }finally{await ui.app.dispose();}
});
test('no failure text points at a launcher log',async()=>{
 for(const last_error of ['duplicate_release_file','leftover_launcher_files'] as const){
  const ui=mount(()=>({...idle(),backend:'in_place',last_error}));
  try{await ui.app.ready;await ui.settle();assert.doesNotMatch(ui.text(),/log/);
  }finally{await ui.app.dispose();}
 }
});