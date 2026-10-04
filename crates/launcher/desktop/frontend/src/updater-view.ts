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
 if(s.status?.phase==='reconciliation_required')return 'Launcher update completion is not confirmed. Game and setup changes are blocked. Finish the installer or reopen the updated launcher; do not retry installation while an installer may still be running.';
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
  case 'restart_required':return 'Replacement or installer started. Update completion is pending startup of the new launcher.';
  case 'installed':return 'The updated launcher started and confirmed its compiled release version.';
  default:return 'Check for a signed launcher release. Apply and restart becomes available after package verification.';
 }
}
export function mountUpdater(document:Document,invoke:Invoke,onChange:()=>void=()=>{}) {
 const runtime=ManagedRuntime.make(Layer.effect(Updater,makeUpdaterWorkflow).pipe(Layer.provide(updaterBridgeLayer(request=>invoke('updater_command',{request})))));
 const get=(id:string)=>document.getElementById(id)!;
 const button=(id:string)=>get(id) as HTMLButtonElement;
 let current:UpdaterState={status:null,pending:null,uncertain:true,error:null},pending=false,disposed=false;
 let mutation:Promise<void>=Promise.resolve();
 const render=()=>{
  if(disposed)return;
  const phase=current.status?.phase,busy=pending||!!current.pending,active=phase==='checking'||phase==='downloading'||phase==='verifying'||phase==='installing'||phase==='restart_required'||phase==='reconciliation_required';
  get('updater-status').textContent=updaterText(current);
  get('updater-notes').textContent=current.status?.offer?.notes??'';
  button('inspect-updater').disabled=busy;
  button('check-updater').disabled=busy||active||current.uncertain||!current.status||phase==='disabled'||!!current.status.requires_reopen;
  button('apply-updater').disabled=busy||current.uncertain||phase!=='ready'||!current.status?.offer||!!current.status?.requires_reopen;
  button('prepare-updater').disabled=busy||current.uncertain||phase!=='available'||!current.status?.offer||!!current.status?.requires_reopen;
 };
 const watcher=runtime.runFork(Effect.flatMap(Updater,s=>Stream.runForEach(s.changes,value=>Effect.sync(()=>{current=value;render();}))));
 const run=(action:'inspect'|'check'|'prepare'|'apply')=>{if(disposed||pending)return mutation;pending=true;render();mutation=runtime.runPromise(Effect.flatMap(Updater,s=>Effect.result(s.run(action)))).then(()=>onChange()).finally(()=>{pending=false;render();});return mutation;};
 const listeners=([['inspect-updater','inspect'],['check-updater','check'],['prepare-updater','prepare'],['apply-updater','apply']] as const).map(([id,action])=>{const listener=()=>{if(!button(id).disabled)void run(action);};button(id).addEventListener('click',listener);return ()=>button(id).removeEventListener('click',listener);});
 const ready=run('inspect');
 return {ready,refresh:()=>run('inspect'),settled:()=>mutation,dispose:async()=>{disposed=true;listeners.forEach(remove=>remove());await Effect.runPromise(Fiber.interrupt(watcher));await mutation;await runtime.dispose();}};
}
