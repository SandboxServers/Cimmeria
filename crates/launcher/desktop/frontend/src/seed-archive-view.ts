import {Context,Effect,Fiber,Layer,ManagedRuntime,Result,Stream} from 'effect';
import type {Invoke} from './view';
import type {PanelContext} from './panel-context';
import {releaseKeyMessage} from './catalog-errors';
import type {SeedArchiveStatus} from './contract';
import {makeSeedArchiveWorkflow,seedArchiveBridgeLayer,type SeedArchiveFailure,type SeedArchiveState} from './seed-archive-workflow';
export class SeedArchive extends Context.Service<SeedArchive,Effect.Success<ReturnType<typeof makeSeedArchiveWorkflow>>>()('launcher/SeedArchive') {}
const mb=(bytes:number)=>Math.round(bytes/(1024*1024));
const errors:Record<string,string>={
 not_a_file:'That is not a file the launcher can read. Choose the archive you downloaded. Nothing was changed.',
 size_mismatch:"That file is not the game archive: its size does not match the signed release's archive. Nothing was changed.",
 hash_mismatch:'That file is the right size but its contents do not match the signed release, so it was not used. Download it again, or let the launcher download it. Nothing was changed.',
 changed:'Your archive changed after it was checked. Choose it again.',
 cancelled:'The check was cancelled. Nothing was changed.',
 busy:'Another install, repair, update or archive check is running. Try again when it finishes.',
 io:'The launcher could not read that file. Check that no other program is using it, then try again. Nothing was changed.',
 release_unavailable:'The launcher could not reach the signed release, so it cannot check your archive. Try again later, or let the launcher download the game.',
 signing_key_unavailable:releaseKeyMessage+' Your archive was not checked or changed.',
 release_verification_failed:'The signed game release failed verification, so your archive was not checked. Use the published launcher ZIP or report this launcher version. Your archive was not changed.',
 transport:'The launcher could not confirm the archive check. Recheck status before choosing again; nothing was retried.',
 schema:'The launcher could not confirm the archive check. Recheck status before choosing again; nothing was retried.',
};
/** Text for the status line; a failed check never shows the "Using" line. */
export function seedArchiveText(s:SeedArchiveState):string {
 if(s.phase==='choosing')return 'Choose your archive in the file window…';
 if(s.phase==='forgetting')return 'Forgetting your archive…';
 if(s.phase==='inspecting')return 'Checking archive status…';
 if(s.cancelling)return 'Cancelling the check…';
 const verifying=s.status?.verifying;
 if(s.phase==='checking'||verifying){
  return verifying&&verifying.phase==='download'?`Checking your archive: ${mb(verifying.current)} of ${mb(verifying.total)} MB.`:'Checking your archive…';
 }
 if(s.error)return errors[s.error]??errors.transport;
 if(s.status?.picker_cancelled)return 'Archive selection cancelled. Nothing was installed or changed.';
 if(!s.status)return 'Checking archive status…';
 const chosen=s.status.chosen;
 if(chosen)return `Using ${chosen.file_name} (verified). The launcher copies it when it needs the game files and never changes or deletes your file.`;
 return 'Already have the game archive (the original 2009 beta .rar)? Choose it to skip the 4.1 GB download.';
}
/** True when the install area should show the archive line (idle needs no line there). */
export const seedArchiveShowsLine=(s:SeedArchiveState)=>s.phase!==null||s.error!==null||!!s.status?.chosen||!!s.status?.picker_cancelled;
/** Only a successful explicit choice/offer acceptance can hand off to Install. */
export const archiveSelectionVerified=(s:SeedArchiveStatus)=>!!s.chosen&&!s.verifying&&!s.last_error&&!s.picker_cancelled;
/** The Downloads question Settings and the install area both ask (AI-16), or null when no offer is asked. */
export function seedArchiveOfferText(s:SeedArchiveState):string|null{
 const found=s.status?.offer;
 if(!found||s.status?.chosen||s.error||s.phase!==null)return null;
 return `Found ${found.file_name} in Downloads. Use it?`;
}
export const makeSeedArchiveRuntime=(invoke:Invoke,pollMillis:number)=>ManagedRuntime.make(Layer.effect(SeedArchive,makeSeedArchiveWorkflow(pollMillis)).pipe(
 Layer.provide(seedArchiveBridgeLayer((command,args)=>invoke(command,args)))));
export type SeedArchiveRuntime=ReturnType<typeof makeSeedArchiveRuntime>;
let shared:{invoke:Invoke;runtime:SeedArchiveRuntime;holders:number}|null=null;
/** Settings and the install area share one runtime (AI-07), so a choice shows in both at once. Each holder releases it when disposed. */
export function acquireSeedArchive(invoke:Invoke,pollMillis:number=500) {
 if(!shared||shared.invoke!==invoke)shared={invoke,runtime:makeSeedArchiveRuntime(invoke,pollMillis),holders:0};
 const held=shared;held.holders++;
 return {runtime:held.runtime,release:async()=>{held.holders--;if(held.holders>0)return;if(shared===held)shared=null;await held.runtime.dispose();}};
}
/** Settings panel (AI-07): choose a downloaded archive, watch its check, cancel it or forget it. */
export function mountSeedArchive(document:Document,invoke:Invoke,context:PanelContext,pollMillis:number=500,onVerified:()=>Promise<unknown>=async()=>{}) {
 const panel=document.getElementById('seed-archive-panel');
 if(!panel)throw new Error('Missing launcher element seed-archive-panel');
 const make=<K extends keyof HTMLElementTagNameMap>(tag:K,id:string)=>{const e=document.createElement(tag);e.id=id;return e;};
 const heading=make('h3','seed-archive-title');heading.textContent='Downloaded game archive';
 const status=make('p','seed-archive-status');status.setAttribute('role','status');status.setAttribute('aria-live','polite');
 const detail=make('p','seed-archive-file');detail.className='caption';
 const offer=make('p','seed-archive-offer');offer.className='caption';offer.hidden=true;
 const yes=make('button','seed-archive-accept');yes.textContent='Yes';yes.hidden=true;
 const choose=make('button','seed-archive-choose');choose.textContent='Use a downloaded game archive…';
 const cancel=make('button','seed-archive-cancel');cancel.textContent='Cancel check';cancel.hidden=true;
 const forget=make('button','seed-archive-forget');forget.textContent='Forget archive';forget.hidden=true;
 panel.textContent='';panel.append(heading,status,detail,offer,yes,choose,cancel,forget);
 const archive=acquireSeedArchive(invoke,pollMillis);const runtime=archive.runtime;
 let current:SeedArchiveState={status:null,phase:null,cancelling:false,error:null};let disposed=false;
 // The button that was pressed shows busy at once, before the workflow confirms it.
 let pressed:'choose'|'accept'|'cancel'|'forget'|null=null;let mutation:Promise<void>=Promise.resolve();
 const render=()=>{if(disposed)return;const s=current;const busy=s.phase!==null||pressed!==null;
  status.textContent=seedArchiveText(s);
  const chosen=s.status?.chosen;
  detail.textContent=chosen&&!s.error&&s.phase===null?`${chosen.file_name}, ${mb(chosen.size)} MB`:'';
  const question=seedArchiveOfferText(s);
  offer.hidden=question===null;offer.textContent=question??'';
  yes.hidden=question===null;yes.disabled=busy;yes.textContent=pressed==='accept'?'Checking…':'Yes';
  yes.setAttribute('aria-busy',String(busy));
  choose.disabled=busy;choose.textContent=s.phase==='choosing'||pressed==='choose'?'Choosing…':'Use a downloaded game archive…';
  choose.setAttribute('aria-busy',String(busy));
  cancel.hidden=s.phase!=='checking';cancel.disabled=s.cancelling||pressed==='cancel';
  forget.hidden=!chosen||s.phase==='checking';forget.disabled=busy;
 };
 const watcher=runtime.runFork(Effect.flatMap(SeedArchive,s=>Stream.runForEach(s.changes,value=>Effect.sync(()=>{current=value;render();}))));
 const run=(kind:'choose'|'accept'|'cancel'|'forget'|null,action:(s:Effect.Success<ReturnType<typeof makeSeedArchiveWorkflow>>)=>Effect.Effect<SeedArchiveStatus,SeedArchiveFailure>,after:(status:SeedArchiveStatus)=>unknown=()=>{})=>{
  if(disposed)return mutation;pressed=kind;render();
  mutation=runtime.runPromise(Effect.flatMap(SeedArchive,s=>Effect.result(action(s)))).then(async result=>{if(Result.isSuccess(result)&&!disposed)await after(result.success);})
   .finally(()=>{pressed=null;render();})
   // A call cut short by disposal rejects with the runtime's interruption; nothing is shown for it.
   .catch(()=>{});
  return mutation;
 };
 const selected=async(s:SeedArchiveStatus)=>{void context.refresh();if(archiveSelectionVerified(s))await onVerified();};
 choose.addEventListener('click',()=>{if(!choose.disabled)void run('choose',s=>s.choose,selected);});
 yes.addEventListener('click',()=>{if(!yes.disabled)void run('accept',s=>s.accept,selected);});
 cancel.addEventListener('click',()=>{if(!cancel.disabled)void run('cancel',s=>s.cancel);});
 forget.addEventListener('click',()=>{if(!forget.disabled)void run('forget',s=>s.forget,()=>{void context.refresh();});});
 const ready=run(null,s=>s.inspect);
 // Disposal does not wait for a native reply that may never come; a late reply is ignored (disposed guards every render and after()).
 return {ready,settled:()=>mutation,dispose:async()=>{disposed=true;await Effect.runPromise(Fiber.interrupt(watcher));await archive.release();}};
}
