// Production Effect workflow and view against the production native host. The
// Rust bridge owns a real isolated store, legacy game tree and signed loopback
// artifact origin; with CIMMERIA_WINE_HELPER set it extracts through the pinned
// Windows helper under Wine. Nothing here answers for the native side.
import {spawn} from 'node:child_process';
import {createInterface} from 'node:readline';
import {mkdtemp,readFile,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import assert from 'node:assert/strict';
import {parseHTML} from 'linkedom';
import {mountAdoption} from './.test-build/adoption-view.mjs';
const root=await mkdtemp(join(tmpdir(),'adoption-uat-'));
const TAG='ADOPTION_NATIVE_UAT ';
function spawnBridge() {
 const child=spawn(process.env.ADOPTION_UAT_BINARY,['--ignored','--nocapture','--exact','host::adoption::uat_bridge::adoption_native_uat_bridge'],{stdio:['pipe','pipe','inherit'],env:{...process.env,ADOPTION_UAT_ROOT:root}});
 const pending=[];let gone=false;
 const exit=new Promise(resolve=>{child.on('error',()=>resolve(-1));child.on('exit',code=>resolve(code));}).then(code=>{gone=true;for(const p of pending.splice(0))p.reject('transport');return code;});
 createInterface({input:child.stdout}).on('line',line=>{if(!line.startsWith(TAG))return;const response=pending.shift(),result=JSON.parse(line.slice(TAG.length));'error' in result?response.reject(result.error):response.resolve(result.ok);});
 const send=request=>new Promise((resolve,reject)=>{
  if(gone)return reject('transport');
  const timer=setTimeout(()=>reject(Error(`native reply deadline: ${JSON.stringify(request)}`)),120000);
  pending.push({resolve:x=>{clearTimeout(timer);resolve(x)},reject:x=>{clearTimeout(timer);reject(x)}});child.stdin.write(JSON.stringify(request)+'\n');});
 return {send,kill:async()=>{child.kill('SIGKILL');await exit;},end:async()=>{child.stdin.end();assert.equal(await exit,0,'native bridge exits cleanly');}};
}
let bridge=spawnBridge();
const {document,window}=parseHTML(await readFile(new URL('./ui/index.html',import.meta.url),'utf8'));
const calls=[];let loseConfirm=false,changes=0;
const app=mountAdoption(document,async(command,args)=>{
 const request=command==='choose_adoption_destination'?{command:'choose'}:(assert.equal(command,'adoption_command'),args.request);
 calls.push(request);
 const reply=await bridge.send(request);
 // The native side accepted the request; only its reply is lost.
 if(request.command==='confirm'&&loseConfirm){loseConfirm=false;throw 'transport';}
 return reply;
},()=>{changes++;});
const get=id=>document.getElementById(id),text=()=>get('adoption-status').textContent;
const sent=command=>calls.filter(c=>c.command===command);
const pause=ms=>new Promise(resolve=>setTimeout(resolve,ms));
const settle=async()=>{await app.settled();await pause(10);await app.settled();};
const press=id=>get(id).dispatchEvent(new window.Event('click'));
const click=async id=>{assert.equal(get(id).disabled,false,`${id} is enabled`);assert.equal(get(id).hidden,false,`${id} is visible`);press(id);await settle();};
const tick=async id=>{assert.equal(get(id).disabled,false,`${id} is enabled`);get(id).checked=true;get(id).dispatchEvent(new window.Event('change'));await settle();};
const inspect=()=>bridge.send({command:'inspect',schema_version:1});
const evidence=()=>bridge.send({command:'evidence'});
const operation=status=>status.native.operation.operation;
async function until(what,ready,limit=30000) {
 const deadline=Date.now()+limit;
 for(;;){const status=await inspect();
  // The view reads the same native state the script just observed.
  if(await ready(status)){await app.refresh();await settle();return status;}
  assert(Date.now()<deadline,`timed out waiting for ${what}: ${text()} ${JSON.stringify({activity:status.activity,operation:operation(status),last_error:status.last_error})}`);await pause(50);}
}
const idle=what=>until(what,s=>s.activity==='idle');
try {
 const {backend}=await bridge.send({command:'hello'});
 const slow=backend==='wine'?600000:30000;
 await app.ready;await settle();
 const before=await evidence();
 assert.equal(before.destination_entries,null);assert.equal(before.owned_directory,null);assert.equal(before.launcher_summary_consent,false);
 assert.match(text(),/Choose where to create a separate verified copy/);
 assert.equal(get('adoption-review').hidden,true);assert.equal(get('cancel-adoption').hidden,true);

 // 1. Failures are visible, change nothing, and keep nothing.
 await bridge.send({command:'origin',mode:'missing'});
 await click('choose-adoption-destination');
 await idle('the failed download');
 assert.match(text(),/signed game files could not be downloaded/);
 await bridge.send({command:'origin',mode:'oversized'});
 await click('choose-adoption-destination');
 await idle('the rejected download');
 assert.match(text(),/did not match the signed release and was discarded/);
 let now=await evidence();
 assert.deepEqual(now.artifacts,[]);assert.equal(now.references,0);assert.equal(now.destination_entries,null);assert.equal(now.source_sha256,before.source_sha256);
 assert.equal(get('choose-adoption-destination').disabled,false,'a clean failure can be retried');

 // 2. A running preparation shows progress, cannot be replaced, and cancels.
 await bridge.send({command:'origin',mode:'stalled'});
 press('choose-adoption-destination');press('choose-adoption-destination');await settle();
 assert.equal(sent('choose').length,3,'a double click starts one preparation');
 await until('download progress',s=>s.progress?.phase==='download');
 assert.match(text(),/Downloading the signed reference/);
 assert.equal(get('adoption-progress').hidden,false);assert.equal(get('cancel-adoption').hidden,false);
 assert.equal(get('choose-adoption-destination').disabled,true,'choosing again cannot orphan the download');
 await assert.rejects(bridge.send({command:'choose'}),e=>e==='busy');
 await click('cancel-adoption');
 let status=await idle('the cancelled preparation');
 assert.equal(operation(status).state,'cancelled');assert.match(text(),/^Cancelled\. The original game folder is unchanged/);
 now=await evidence();assert.equal(now.references,0);assert.deepEqual(now.artifacts,[]);

 // 3. The launcher dies mid-download. After reopening the same store nothing is
 // replayed; the interrupted preparation is offered for explicit removal.
 await bridge.send({command:'origin',mode:'stalled'});
 await click('choose-adoption-destination');
 await until('download progress',s=>s.progress?.phase==='download');
 const crashed=(await evidence()).references;assert.equal(crashed,1);
 await bridge.kill();
 bridge=spawnBridge();
 await bridge.send({command:'origin',mode:'signed'});
 await app.refresh();await settle();await app.refresh();await settle();
 status=await inspect();
 assert.equal(operation(status).state,'reconciliation_required');assert.equal(status.reconciliation.kind,'preparation');
 assert.match(text(),/Preparation was interrupted and is never repeated automatically/);
 assert.equal(get('choose-adoption-destination').disabled,true);
 assert.equal((await evidence()).references,1,'reopening removes nothing by itself');
 await click('remove-adoption-preparation');
 assert.match(get('adoption-maintenance-consequences').textContent,/Permanently remove the private reference files/);
 assert.equal(sent('abandon_preparation').length,0);
 await click('confirm-adoption-maintenance');
 status=await inspect();assert.equal(status.reconciliation,null);assert.equal(operation(status).state,'cancelled');
 now=await evidence();assert.equal(now.references,0);assert.equal(now.source_sha256,before.source_sha256);
 assert.equal(sent('abandon_preparation').length,1);

 // 4. A real review: readable, with every confirmation shown and unchecked.
 await click('choose-adoption-destination');
 status=await until('the review',s=>{assert.equal(s.last_error,null,'preparation failed');return !!s.review;},slow);
 const review=status.review;
 assert.equal(get('adoption-review').hidden,false);
 assert.equal(get('adoption-source').textContent,review.source);assert.equal(get('adoption-destination').textContent,before.destination);
 assert(get('adoption-release').textContent.includes(review.release.manifest_sha256));
 assert.match(get('adoption-counts').textContent,/2 files are identical.*1 file was modified.*0 files are missing.*2 files are not part/);
 assert.deepEqual([...get('adoption-differences').querySelectorAll('li')].map(li=>li.textContent),['Modified: later.txt','Not part of the release: launcher-installed.json','Not part of the release: unknown.dll']);
 assert.match(get('adoption-settings').textContent,/Second \(https:\/\/example.invalid\/second\); First \(https:\/\/example.invalid\/first\)\. Client patches: off/);
 assert.match(get('adoption-telemetry').textContent,/you opted in with the old launcher.*diagnostics stay off/);
 for(const id of ['adoption-normalize','adoption-closed','adoption-telemetry']){assert.equal(get(`${id}-row`).hidden,false);assert.equal(get(id).checked,false);}
 assert.equal(get('confirm-adoption').disabled,true);
 assert.equal((await evidence()).destination_entries,null,'a review copies nothing');

 // 5. Settings change under the review: it stays on screen and is not confirmed.
 for(const id of ['adoption-normalize','adoption-closed','adoption-telemetry'])await tick(id);
 await bridge.send({command:'toggle_diagnostics'});
 await click('confirm-adoption');
 assert.equal(sent('confirm').length,0,'a stale review is never sent');
 assert.match(text(),/can no longer be confirmed\. Dismiss it and prepare a new one/);
 assert.equal(get('adoption-review').hidden,false);assert.equal(get('adoption-source').textContent,review.source);
 assert.equal(get('confirm-adoption').disabled,true);
 assert.equal((await inspect()).review.preview_handle,review.preview_handle,'the native review is still held');
 await click('dismiss-adoption');
 status=await idle('the dismissed review');
 assert.equal(status.review,null);assert.equal(get('adoption-review').hidden,true);
 now=await evidence();assert.equal(now.references,0);assert.equal(now.destination_entries,null);
 assert.equal(now.launcher_summary_consent,true);

 // 6. Confirm once; the reply is lost; the retained copy is observed, not resent.
 await click('choose-adoption-destination');
 status=await until('the second review',s=>{assert.equal(s.last_error,null,'preparation failed');return !!s.review;},slow);
 assert.notEqual(status.review.preview_handle,review.preview_handle);
 for(const id of ['adoption-normalize','adoption-closed','adoption-telemetry']){assert.equal(get(id).checked,false,'a new review starts unchecked');await tick(id);}
 await bridge.send({command:'fault',mode:'hold'});
 loseConfirm=true;
 press('confirm-adoption');press('confirm-adoption');await settle();
 assert.equal(sent('confirm').length,1,'a double click confirms once');
 assert.match(text(),/could not be confirmed, and nothing was retried/);
 assert.equal(get('confirm-adoption').disabled,true);
 await click('inspect-adoption');
 status=await until('the running copy',s=>s.activity==='copying'&&operation(s).state==='running');
 assert.match(text(),/Starting the verified copy|Copying verified files/);
 assert.equal(get('adoption-progress').hidden,false);assert.equal(get('cancel-adoption').hidden,false);assert.equal(get('cancel-adoption').textContent,'Cancel copy');
 assert.equal(get('adoption-review').hidden,true);
 now=await evidence();assert.equal(now.game_executable,false);assert.equal(now.owned_directory,null);assert.equal(now.source_sha256,before.source_sha256);
 await bridge.send({command:'fault',mode:'release'});
 status=await idle('publication');
 assert.equal(status.last_error,null);assert.equal(operation(status).state,'succeeded');
 assert.equal(sent('confirm').length,1,'a lost reply never redispatches the copy');
 assert(text().includes(`Verified copy created in ${before.destination}`));
 assert.equal(get('choose-adoption-destination').disabled,true);assert.equal(get('cancel-adoption').hidden,true);

 // 7. What is on disk and in the store, after reopening it.
 status=await bridge.send({command:'reopen'});
 await app.refresh();await settle();
 assert.equal(status.completed.directory,before.destination);assert(text().includes(`Verified copy created in ${before.destination}`));
 const after=await evidence();
 assert.equal(after.source_sha256,before.source_sha256,'the original game and launcher folders are byte-identical');
 assert.equal(after.import_sha256,before.import_sha256,'the imported identity, configuration and consent record is unchanged');
 assert.equal(after.game_executable,true);assert.equal(after.owned_directory,before.destination);assert.equal(after.uninstall_directory,before.destination);
 assert.equal(after.install_directory,before.destination);
 assert.equal(after.backend,backend==='wine'?'wine':'native');
 assert.equal(after.launcher_summary_consent,true,'adoption keeps the diagnostics choice made before it');
 assert.equal(after.artifacts,null);assert.equal(after.references,0);assert.equal(after.plans,1);
 assert(changes>0,'saved revisions notified other views');
 console.log(`Native adoption logic UAT passed (${backend} extraction backend): production Effect workflow and view against the production host with a real isolated store, legacy game tree and signed loopback artifact origin.`);
 console.log('Exercised: unavailable and oversized signed downloads rejected with nothing kept; single preparation per double click; progress and cancel; no re-choice during preparation; process kill mid-download, reopen and explicit preparation removal without replay; rendered review (source, destination, signed release, classification counts and paths, ordered login servers, patch setting, telemetry and diagnostics statements); unchecked required confirmations; stale review preserved and never sent, then dismissed; single confirmation per double click; lost confirm reply inspected without redispatch; retained copy progress and cancel control; published copy, desktop ownership and backend identity read from disk after reopening; byte-identical source folders and import record.');
 console.log(`Observed prerequisite target for the adopted copy: ${after.prerequisite_target??'none offered by this build'}.`);
 console.log('Excluded: native folder dialog and Tauri IPC, the signed production catalog and HTTPS transport, the published multi-gigabyte client and its RAR/CAB seed, copy recovery and abandonment (covered by host tests), prerequisites and Play for the adopted copy, Windows, and packaged visual/focus/layout UAT.');
} finally {await app.dispose();await bridge.end().finally(()=>rm(root,{recursive:true,force:true}));}
