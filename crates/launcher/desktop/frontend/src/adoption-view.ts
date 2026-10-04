import {Context,Effect,Fiber,Layer,ManagedRuntime,Stream} from 'effect';
import {AdoptionState,AdoptionStatus,Choice,Maintenance,adoptionBridgeLayer,consented,makeAdoptionWorkflow,unknownOutcome} from './adoption-workflow';
import type {Invoke} from './view';
class Adoption extends Context.Service<Adoption,Effect.Success<typeof makeAdoptionWorkflow>>()('launcher/Adoption') {}
type Review=NonNullable<AdoptionStatus['review']>;
const unchanged='The original game folder is unchanged.';
const failures:Record<string,string>={
 busy:'Another launcher operation is running. Wait for it to finish, then recheck.',
 in_use:'The old launcher or game still holds its lock, or the destination already exists. Close them, then recheck.',
 invalid_directory:'That location cannot be used. Choose a folder separate from the old game and launcher that does not already contain a “Stargate Worlds” folder.',
 unsupported_catalog:'The imported settings use a custom content catalog. Verified adoption supports only the built-in signed catalog and will not rewrite your settings. Keep using the old launcher for this installation.',
 unsupported_configuration:'The imported settings load a custom patch DLL, which the desktop launcher does not support and will not silently drop. Keep using the old launcher for this installation.',
 source_changed:`The old game folder or its launcher records changed during preparation. Close the old game and launcher, then choose a location again. ${unchanged}`,
 no_reusable_files:'No file in the old game folder matches the signed release, so there is nothing to adopt. Install the game normally instead.',
 consent_required:'Tick every confirmation shown in the review first.',
 stale_revision:'Settings or launcher operations changed after this review was prepared, so it can no longer be confirmed. Dismiss it and prepare a new one.',
 review_unavailable:'That review is no longer held. A copy is never started twice; recheck the status.',
 network:`The signed game files could not be downloaded. Check your connection, then choose a location again. ${unchanged}`,
 release_unavailable:'The signed release could not be read from the catalog. Check your connection, then try again.',
 invalid_artifact:`A downloaded file did not match the signed release and was discarded. Try again later. ${unchanged}`,
 unsupported_archive:`The signed release uses an archive this build cannot verify. ${unchanged}`,
 cancelled:`Cancelled. ${unchanged} If a copy had started, its partial folder was left in place and is not used; choose a different location to try again.`,
 launcher_too_old:'Update the launcher before adopting this release.',
 unsafe_file:`A file or folder could not be verified safely (a link, a special file, or a folder that was replaced), so nothing was copied or removed. ${unchanged}`,
 identity_conflict:'A desktop-owned installation already exists, or this request was already used.',
 recovery_required:'An interrupted adoption must be resolved first, using the actions offered here.',
 import_required:'Import legacy launcher settings first.',
 platform_unavailable:'Verified copy adoption is not available in this build.',
 unsupported_schema:'This launcher build does not understand that adoption request.',
};
const failureText=(code:string)=>unknownOutcome(code)?'The result could not be confirmed, and nothing was retried. Recheck adoption status before continuing.':failures[code]??failures.platform_unavailable;
const megabytes=(bytes:number)=>`${(bytes/1048576).toFixed(1)} MB`;
const activityText=(status:AdoptionStatus)=>{
 const p=status.progress;
 if(status.activity==='copying')return !p?`Starting the verified copy… ${unchanged}`:p.current<p.total?`Copying verified files: ${p.current} of ${p.total}. ${unchanged}`:'Verifying and publishing the copy. This step cannot be cancelled.';
 if(p?.phase==='download')return `Downloading the signed reference: ${megabytes(p.current)} of ${megabytes(p.total)}.`;
 if(p?.phase==='extraction')return `Extracting the signed reference: ${p.current} of ${p.total}.`;
 return 'Preparing the signed reference and comparing your files…';
};
export function adoptionText(s:AdoptionState):string {
 if(s.pending)return 'Working on verified adoption…';
 const status=s.status;
 if(!status)return s.error?failureText(s.error):'Checking adoption availability…';
 if(status.native.requires_reopen)return 'Restart the launcher to inspect saved adoption state.';
 if(s.error)return failureText(s.error);
 if(status.backend==='unsupported_platform')return 'Verified copy adoption is not available on this operating system yet. Keep using the old launcher for this installation.';
 if(status.backend==='helper_unavailable')return 'This build does not include the verified archive helper, so verified copy adoption is unavailable.';
 if(status.activity==='copying'||status.activity==='preparing')return activityText(status);
 const r=status.reconciliation;
 if(r?.kind==='preparation')return `Preparation was interrupted and is never repeated automatically. Remove its private files to continue. ${unchanged}`;
 if(r?.kind==='copy')return `The copy into ${r.directory} was interrupted. ${r.can_recover?'Its verified files are staged: recover to finish publishing them.':r.can_abandon?'Nothing was published: abandon it, then choose a different location.':'Recheck after restarting the launcher.'} ${unchanged}`;
 if(status.review)return s.stale?failures.stale_revision:`Review the comparison and confirmations below. Nothing is copied until you confirm. ${unchanged}`;
 if(status.last_error)return failureText(status.last_error);
 if(status.completed)return `Verified copy created in ${status.completed.directory}. The original game folder and old launcher are unchanged and no longer needed by this copy. Game setup and Play for adopted copies continue in the Play area when this build offers them.`;
 if(status.owned)return 'A desktop-owned installation already exists, so there is nothing further to adopt.';
 if(status.imported?.blocker)return failureText(status.imported.blocker);
 if(status.preparations.length)return 'Leftover preparation files from an earlier attempt can be removed.';
 if(status.imported)return 'Legacy settings are saved. Choose where to create a separate verified copy; the launcher makes a new “Stargate Worlds” folder there and never changes the old one.';
 return 'Import legacy launcher settings above first, then start verified adoption here.';
}
const labels={matched:'Identical',known_transform:'Old launcher setup change',modified:'Modified',missing:'Missing',extra:'Not part of the release'} as const;
const maintenanceText:Record<Maintenance['action'],[string,string]>={
 recover:['Recover adoption','Finish publishing the already verified staged copy. Nothing is downloaded or copied again; recovery is refused if the staged files no longer verify.'],
 abandon:['Abandon adoption','End this interrupted adoption. Files already copied stay in the destination folder and are not used; choose a different location afterwards. The original game folder is unchanged.'],
 abandon_preparation:['Remove preparation files','Permanently remove the private reference files of the interrupted preparation. The original game folder and your settings are unchanged.'],
};
function reviewLines(r:Review,summaryConsent:boolean) {
 const c=r.counts,plural=(n:number,one:string,many:string)=>`${n} ${n===1?one:many}`;
 return {
  release:`Signed release ${r.release.manifest_sha256} (seed ${r.release.seed_sha256.slice(0,16)}…, ${r.release.patches.length?`patches in signed order: ${r.release.patches.join(', ')}`:'no patches'}).`,
  counts:`${plural(c.matched,'file is','files are')} identical and copied from your game. ${plural(c.known_transform,'file differs','files differ')} only by the old launcher’s own setup change. ${plural(c.modified,'file was','files were')} modified. ${plural(c.missing,'file is','files are')} missing. ${plural(c.extra,'file is','files are')} not part of the signed release.`,
  consequences:`The copy will hold exactly the signed release. Modified, missing and setup-changed files are taken from the signed reference instead of your game${r.requires_normalization?'':' (none in this comparison)'}. Files that are not part of the release, saved game settings and other user files are not copied or migrated${r.user_data_remains_in_source?': they stay in the original folder':''}. ${unchanged}`,
  settings:`Login servers, in your order: ${r.login_servers.length?r.login_servers.map(s=>`${s.name} (${s.url})`).join('; '):'none'}. Client patches: ${r.client_patches_enabled?'on':'off'}, as imported.`,
  telemetry:`Game telemetry: ${r.game_telemetry_opted_in?'you opted in with the old launcher':'not opted in'}. ${r.game_telemetry_available?'':'This desktop build cannot send game telemetry; your saved choice is kept and nothing is sent. '}Launcher setup diagnostics stay ${summaryConsent?'on':'off'}; adoption does not change that choice.`,
 };
}
export function mountAdoption(document:Document,invoke:Invoke,onChange:()=>void=()=>{}) {
 const runtime=ManagedRuntime.make(Layer.effect(Adoption,makeAdoptionWorkflow).pipe(Layer.provide(adoptionBridgeLayer(request=>request==='choose'?invoke('choose_adoption_destination'):invoke('adoption_command',{request})))));
 const get=(id:string)=>document.getElementById(id)!;
 const button=(id:string)=>get(id) as HTMLButtonElement;
 const box=(id:string)=>get(id) as HTMLInputElement;
 const choices:[string,Choice][]=[['adoption-normalize','normalize'],['adoption-closed','closed'],['adoption-telemetry','telemetry']];
 let current:AdoptionState={status:null,pending:false,uncertain:true,error:null,stale:false,maintenance:null,normalize:false,telemetry:false,closed:false};
 let disposed=false,mutating=false,reading=false,pollable=true,notified:string|null=null;
 let mutation:Promise<void>=Promise.resolve(),inspection:Promise<void>=Promise.resolve();
 // Recovery actions the current native status offers, by button.
 let offers:Record<string,Maintenance|null>={};
 const render=()=>{
  if(disposed)return;
  const s=current.status,review=s?.review,r=s?.reconciliation,busy=mutating||current.pending,enabled=!busy&&!current.uncertain;
  const working=s?.activity==='preparing'||s?.activity==='copying';
  // The press itself is acknowledged before the workflow publishes anything.
  get('adoption-status').textContent=adoptionText({...current,pending:busy});
  const idle=!!s&&s.activity==='idle'&&!r&&!s.owned&&s.backend==='available'&&!!s.imported&&!s.imported.blocker&&(s.native.operation.operation?['succeeded','failed','cancelled'].includes(s.native.operation.operation.state):true);
  button('choose-adoption-destination').disabled=!enabled||!idle;
  button('inspect-adoption').disabled=busy;
  button('cancel-adoption').hidden=!s?.cancellable;
  button('cancel-adoption').disabled=!enabled;
  button('cancel-adoption').textContent=s?.activity==='copying'?'Cancel copy':'Cancel preparation';
  const progress=get('adoption-progress') as HTMLProgressElement;
  progress.hidden=!working;
  if(working&&s.progress&&s.progress.current<s.progress.total){progress.max=Math.max(1,s.progress.total);progress.value=s.progress.current;}else progress.removeAttribute('value');
  const removable=r?.kind==='preparation'?r.preparation_id:s?.preparations[0];
  offers={'recover-adoption':r?.kind==='copy'&&r.can_recover?{action:'recover',id:r.operation_id}:null,
   'abandon-adoption':r?.kind==='copy'&&r.can_abandon?{action:'abandon',id:r.operation_id}:null,
   'remove-adoption-preparation':removable&&!working&&!review?{action:'abandon_preparation',id:removable}:null};
  for(const [id,offer] of Object.entries(offers)){button(id).hidden=!offer;button(id).disabled=!enabled||!!current.maintenance;}
  const m=current.maintenance;
  get('adoption-maintenance').hidden=!m;
  if(m){get('adoption-maintenance-title').textContent=maintenanceText[m.action][0];get('adoption-maintenance-consequences').textContent=maintenanceText[m.action][1];button('confirm-adoption-maintenance').textContent=maintenanceText[m.action][0];}
  button('confirm-adoption-maintenance').disabled=!enabled||!m;
  button('dismiss-adoption-maintenance').disabled=busy;
  get('adoption-review').hidden=!review;
  if(review){
   // Paths and settings come from disk and old configuration: text only, never HTML.
   const lines=reviewLines(review,s!.native.preferences.launcher_summary_consent);
   get('adoption-source').textContent=review.source;get('adoption-destination').textContent=review.destination;
   for(const key of ['release','counts','consequences','settings','telemetry'] as const)get(`adoption-${key}`).textContent=lines[key];
   const list=get('adoption-differences');
   const items=review.differences.map(d=>`${labels[d.classification]}: ${d.path}`);
   if(review.differences_omitted)items.push(`…and ${review.differences_omitted} more differences not listed here.`);
   list.replaceChildren(...items.map(text=>{const item=document.createElement('li');item.textContent=text;return item;}));
   list.hidden=!items.length;
   get('adoption-normalize-row').hidden=!review.requires_normalization;
   get('adoption-telemetry-row').hidden=!review.requires_telemetry_acceptance;
  }
  for(const [id,choice] of choices){box(id).checked=current[choice];box(id).disabled=!enabled||!review||current.stale;}
  button('confirm-adoption').disabled=!enabled||current.stale||!consented(current);
  button('dismiss-adoption').disabled=busy||!review;
 };
 // Other views follow saved revisions, not this view's polling.
 const notify=()=>{const n=current.status?.native;if(!n)return;const key=`${n.operation.revision}:${n.preferences.revision}`;const first=notified===null;if(key===notified)return;notified=key;if(!first)onChange();};
 const watcher=runtime.runFork(Effect.flatMap(Adoption,w=>Stream.runForEach(w.changes,s=>Effect.sync(()=>{current=s;render();notify();}))));
 const refresh=()=>{
  if(disposed||reading||mutating)return inspection;
  reading=true;
  inspection=runtime.runPromise(Effect.flatMap(Adoption,w=>Effect.result(w.inspect))).then(result=>{pollable=result._tag==='Success';}).finally(()=>{reading=false;render();});
  return inspection;
 };
 const run=(effect:Effect.Effect<unknown,unknown,Adoption>)=>{
  if(disposed||mutating)return mutation;
  mutating=true;render();
  mutation=runtime.runPromise(Effect.result(effect)).then(()=>{}).finally(()=>{mutating=false;render();});
  return mutation;
 };
 const listeners:(()=>void)[]=[];
 const listen=(id:string,type:string,fn:(target:HTMLElement)=>void)=>{const target=get(id),listener=()=>fn(target);target.addEventListener(type,listener);listeners.push(()=>target.removeEventListener(type,listener));};
 const on=(id:string,fn:()=>void)=>listen(id,'click',target=>{if(!(target as HTMLButtonElement).disabled)fn();});
 on('inspect-adoption',()=>{void refresh();});
 on('choose-adoption-destination',()=>{void run(Effect.flatMap(Adoption,w=>w.choose));});
 on('cancel-adoption',()=>{void run(Effect.flatMap(Adoption,w=>w.cancel));});
 on('dismiss-adoption',()=>{void run(Effect.flatMap(Adoption,w=>w.dismiss));});
 on('confirm-adoption',()=>{void run(Effect.flatMap(Adoption,w=>w.confirm));});
 for(const id of ['recover-adoption','abandon-adoption','remove-adoption-preparation'])
  on(id,()=>{const offer=offers[id];if(offer)void run(Effect.flatMap(Adoption,w=>w.propose(offer)));});
 on('confirm-adoption-maintenance',()=>{void run(Effect.flatMap(Adoption,w=>w.maintain));});
 on('dismiss-adoption-maintenance',()=>{void run(Effect.flatMap(Adoption,w=>w.withdraw));});
 for(const [id,choice] of choices)listen(id,'change',target=>{void runtime.runPromise(Effect.flatMap(Adoption,w=>w.set(choice,(target as HTMLInputElement).checked)));});
 const ready=refresh();
 // A failed read stops polling until an explicit recheck succeeds.
 const timer=setInterval(()=>{const s=current.status,op=s?.native.operation.operation;
  if(pollable&&!current.uncertain&&s&&(s.activity==='preparing'||s.activity==='copying'||(op?.kind==='adopt'&&['starting','running','cancel_requested'].includes(op.state))))void refresh();},1000);
 return {ready,refresh,settled:()=>Promise.all([inspection,mutation]),dispose:async()=>{disposed=true;clearInterval(timer);listeners.forEach(remove=>remove());await Effect.runPromise(Fiber.interrupt(watcher));await Promise.all([inspection,mutation]);await runtime.dispose();}};
}
