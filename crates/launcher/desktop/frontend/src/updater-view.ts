import {Context,Effect,Fiber,Layer,ManagedRuntime,Stream} from 'effect';
import {makeUpdaterWorkflow,updaterBridgeLayer,updaterFailureCode,UpdaterState} from './updater-workflow';
import type {Invoke} from './view';
class Updater extends Context.Service<Updater,Effect.Success<typeof makeUpdaterWorkflow>>()('launcher/Updater') {}
export function updaterText(s:UpdaterState):string {
 if(s.pending==='check')return 'Checking for launcher updates…';
 if(s.pending==='prepare')return 'Downloading and verifying the signed launcher package. Wait for this to finish before starting the game or setup. Closing this view does not cancel the download.';
 if(s.pending==='apply')return 'Installing the verified launcher update. The launcher will close after replacement starts.';
 if(s.pending==='inspect')return 'Reading saved update status…';
 if(s.status?.requires_reopen)return 'Restart the launcher before continuing with updates.';
 const error=s.error??(s.status?.failure?updaterFailureCode(s.status.failure):null);
 if(s.status?.phase==='disabled')return 'Launcher updates are unavailable in this development build.';
 if(s.status?.phase==='reconciliation_required')return 'Launcher update completion is not confirmed. Game and setup changes are blocked. Reopen the updated launcher to confirm it; do not retry the update while the new launcher may still be starting.';
 if(error){
  if(error==='busy')return 'Wait for the game or setup operation to finish, then refresh update status.';
  if(error==='signature'||error==='signed_version')return 'The package signature or signed release version could not be verified. Nothing was installed. Check again for a fresh offer.';
  if(error==='interrupted')return 'The download was interrupted. Nothing was installed. Check again to retry.';
  if(error==='stale_offer'||error==='stale_revision')return 'The update offer or launcher activity changed. Refresh status before continuing.';
  if(error==='spawn'||error==='replace')return 'The update could not start. The original launcher was restored when its files could be verified. Refresh status for recovery details.';
  return 'The update could not be confirmed. Refresh status before continuing.';
 }
 switch(s.status?.phase){
  case 'available':return `Launcher ${s.status.offer?.version} is available. Download verifies the signature and signed version before saving a package.`;
  case 'up_to_date':return 'No newer launcher release was found.';
  case 'checking':return 'Checking for launcher updates. Refresh to see the result.';
  case 'downloading':case 'verifying':return 'The package is downloading or its signature is being verified. Refresh to see the result.';
  case 'ready':return 'Signed launcher package verified and saved. The running launcher has not changed. Apply and restart installs this update and closes the launcher.';
  case 'installing':return 'Installing the verified launcher update. Wait for the replacement to start.';
  case 'restart_required':return 'New launcher files are in place and the new launcher was started. Update completion is pending its startup.';
  case 'installed':return 'The updated launcher started and confirmed its compiled release version.';
  default:return 'Check for a signed launcher release. Apply and restart becomes available after package verification.';
 }
}
export function mountUpdater(document:Document,invoke:Invoke,onChange:()=>void=()=>{},options:{startup?:boolean;onPlayGate?:(blocked:boolean)=>void}={}) {
 const runtime=ManagedRuntime.make(Layer.effect(Updater,makeUpdaterWorkflow).pipe(Layer.provide(updaterBridgeLayer(request=>invoke('updater_command',{request})))));
 const get=(id:string)=>document.getElementById(id)!;
 const button=(id:string)=>get(id) as HTMLButtonElement;
 let current:UpdaterState={status:null,pending:null,uncertain:true,error:null},pending=false,disposed=false,starting=!!options.startup,updating=false;
 let mutation:Promise<void>=Promise.resolve();
 const render=()=>{
  if(disposed)return;
  const phase=current.status?.phase,busy=pending||!!current.pending,active=phase==='checking'||phase==='downloading'||phase==='verifying'||phase==='installing'||phase==='restart_required'||phase==='reconciliation_required';
  const known=!!current.status?.offer&&['available','ready','failed'].includes(phase??'');
  options.onPlayGate?.(starting||busy||active||known||!!current.status?.requires_reopen||!current.status);
  const main=document.getElementById('update-launcher') as HTMLButtonElement|null;
  if(main){main.hidden=!(known||active||current.error||current.status?.failure&&phase!=='disabled');main.disabled=busy||active||!!current.status?.requires_reopen;main.setAttribute('aria-busy',String(busy));
   main.textContent=updating?'Updating launcher…':known||active?'Update and restart':'Check for updates';}
  const text=updating?'Updating the launcher. Downloading, verifying and restarting; keep the launcher open…':updaterText(current);
  const mainStatus=document.getElementById('launcher-update-status');if(mainStatus)mainStatus.textContent=known?`Update the launcher before playing. ${text}`:text;
  get('updater-status').textContent=text;
  get('updater-notes').textContent=current.status?.offer?.notes??'';
  button('inspect-updater').disabled=busy;
  button('check-updater').disabled=busy||active||current.uncertain||!current.status||phase==='disabled'||!!current.status.requires_reopen;
  button('apply-updater').disabled=busy||current.uncertain||phase!=='ready'||!current.status?.offer||!!current.status?.requires_reopen;
  button('prepare-updater').disabled=busy||current.uncertain||phase!=='available'||!current.status?.offer||!!current.status?.requires_reopen;
 };
 const watcher=runtime.runFork(Effect.flatMap(Updater,s=>Stream.runForEach(s.changes,value=>Effect.sync(()=>{current=value;render();}))));
 const run=(action:'inspect'|'check'|'prepare'|'apply'|'startup'|'update')=>{
  if(disposed||pending)return mutation;pending=true;updating=action==='update';render();
  mutation=runtime.runPromise(Effect.flatMap(Updater,s=>Effect.result(Effect.gen(function*(){
   if(action==='startup'){
    const status=yield* s.run('inspect');
    if(!status.requires_reopen&&['idle','up_to_date','installed','available','failed'].includes(status.phase))yield* s.run('check');
   }else if(action==='update'){
    let status=yield* s.run('inspect');
    if(['idle','up_to_date','installed','failed'].includes(status.phase))status=yield* s.run('check');
    if(status.phase==='available')status=yield* s.run('prepare');
    if(status.phase==='ready')yield* s.run('apply');
   }else yield* s.run(action);
  })))).then(()=>onChange()).finally(()=>{starting=false;pending=false;updating=false;render();});return mutation;
 };
 const listeners=([['inspect-updater','inspect'],['check-updater','check'],['prepare-updater','prepare'],['apply-updater','apply']] as const).map(([id,action])=>{const listener=()=>{if(!button(id).disabled)void run(action);};button(id).addEventListener('click',listener);return ()=>button(id).removeEventListener('click',listener);});
 const main=document.getElementById('update-launcher') as HTMLButtonElement|null;
 const update=()=>{if(main&&!main.disabled)void run('update');};main?.addEventListener('click',update);
 if(main)listeners.push(()=>main.removeEventListener('click',update));
 const ready=run(options.startup?'startup':'inspect');
 return {ready,refresh:()=>run('inspect'),settled:()=>mutation,dispose:async()=>{disposed=true;listeners.forEach(remove=>remove());await Effect.runPromise(Fiber.interrupt(watcher));await mutation;await runtime.dispose();}};
}
