import {Context,Effect,Fiber,Layer,ManagedRuntime,Stream} from 'effect';
import {makeMigrationWorkflow,migrationBridgeLayer,MigrationState} from './migration-workflow';
import type {Invoke} from './view';
class Migration extends Context.Service<Migration,Effect.Success<typeof makeMigrationWorkflow>>()('launcher/Migration') {}
export function migrationText(s:MigrationState):string {
 if(s.pending)return 'Checking legacy settings…';
 if(s.error){
  if(s.error==='source_changed'||s.error==='stale_revision')return 'The source or settings changed. Select the folders again and review a fresh preview.';
  if(s.error==='busy'||s.error==='in_use')return 'Close the old launcher and wait for other operations to finish, then recheck.';
  if(s.error==='conflict')return 'Import conflicts with saved identity or desktop installation history. Existing settings and files are preserved.';
  if(['corrupt','missing_source','invalid_source','too_large','unsafe_file','unsupported_schema'].includes(s.error))return 'These folders do not contain supported legacy records. Check launcher-config.json, install.json and launcher-installed.json, then choose again.';
  return 'Import result could not be confirmed. Recheck saved status before continuing; import was not retried. Restart the launcher if storage requires reopening.';
 }
 if(s.status?.native.requires_reopen)return 'Restart the launcher to recover saved import state before continuing.';
 if(s.status?.preview)return 'Review the folders, identity, configuration and ordered historical patch claims below. Import changes the selected game folder and saves these records; it does not modify game files.';
 if(s.status?.imported)return 'Legacy settings and identity saved. Historical content is unverified: this import does not enable Play, Repair or Uninstall. Keep using the old launcher for this installation until verified adoption is available, or choose a separate empty folder for a new desktop install.';
 return 'Import settings from an existing launcher. Select its folder, then explicitly select the corresponding game root. Windows paths are not automatically converted.';
}
export function mountMigration(document:Document,invoke:Invoke,onChange:()=>void=()=>{}) {
 const runtime=ManagedRuntime.make(Layer.effect(Migration,makeMigrationWorkflow).pipe(Layer.provide(migrationBridgeLayer(request=>request==='choose'?invoke('choose_legacy_source'):invoke('migration_command',{request})))));
 const get=(id:string)=>document.getElementById(id)!;
 const button=(id:string)=>get(id) as HTMLButtonElement;
 let current:MigrationState={status:null,pending:false,uncertain:true,error:null};let pending=false;let disposed=false;let mutation:Promise<void>=Promise.resolve();
 const render=()=>{
  if(disposed)return;
  const busy=pending||current.pending;
  get('migration-status').textContent=busy?'Checking legacy settings…':migrationText(current);
  button('choose-legacy').disabled=busy||!!current.status?.native.requires_reopen;
  button('inspect-migration').disabled=busy;
  button('confirm-migration').disabled=busy||current.uncertain||!current.status?.preview;
  button('dismiss-migration').disabled=busy;
  const data=current.status?.preview?.imported??current.status?.imported;
  get('migration-review').hidden=!data;
  get('migration-actions').hidden=!current.status?.preview;
  // Source values are text, never HTML, URLs to fetch, or command inputs.
  get('migration-details').textContent=data?JSON.stringify({folders:data.source,identity:data.identity,configuration:data.config,historical_ledger:data.ledger},null,2):'';
  get('migration-consent').textContent=data?`Game telemetry consent: ${data.config.telemetry.opted_in?'opted in':'off'}. Launcher setup diagnostics: ${current.status!.native.preferences.launcher_summary_consent?'on':'off'} (unchanged). Import sends nothing.`:'';
 };
 const watcher=runtime.runFork(Effect.flatMap(Migration,s=>Stream.runForEach(s.changes,value=>Effect.sync(()=>{current=value;render();}))));
 const run=(action:'choose'|'inspect'|'dismiss'|'confirm')=>{if(disposed||pending)return mutation;pending=true;render();mutation=runtime.runPromise(Effect.flatMap(Migration,s=>Effect.result(s.run(action)))).then(()=>{if(action==='confirm'||action==='inspect')onChange();}).finally(()=>{pending=false;render();});return mutation;};
 const handlers=[['choose-legacy','choose'],['inspect-migration','inspect'],['confirm-migration','confirm'],['dismiss-migration','dismiss']] as const;
 const listeners=handlers.map(([id,action])=>{const listener=()=>{if(!button(id).disabled)void run(action);};button(id).addEventListener('click',listener);return ()=>button(id).removeEventListener('click',listener);});
 const ready=run('inspect');
 return {ready,refresh:()=>run('inspect'),settled:()=>mutation,dispose:async()=>{disposed=true;listeners.forEach(remove=>remove());await Effect.runPromise(Fiber.interrupt(watcher));await mutation;await runtime.dispose();}};
}
