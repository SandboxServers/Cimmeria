import {Context,Effect,Fiber,Layer,ManagedRuntime,Stream} from 'effect';
import {gameSetupComplete,launchBlocked,launchBridgeLayer,LaunchState,makeLaunchWorkflow} from './launch-workflow';
import {operationOpen} from './contract';
import type {Invoke} from './view';
class Launch extends Context.Service<Launch,Effect.Success<typeof makeLaunchWorkflow>>()('launcher/Play') {}
// An adopt operation that is not terminal is owned by the adoption panel; Play says one line about it.
function adoptLine(s:LaunchState):string|null {
 const operation=s.status?.native.operation.operation;
 if(operation?.kind!=='adopt'||!operationOpen(operation))return null;
 return operation.state==='reconciliation_required'?'Finish or abandon the interrupted adoption in the adoption panel before you play.':'An adoption review is open. Finish or cancel it in the adoption panel.';
}
export function launchText(s:LaunchState):string {
 if(s.pending)return 'Starting game…';
 if(s.error==='launcher_too_old'||s.status?.launcher_update_required)return 'Update the launcher before playing this release. Your game files are preserved.';
 if(s.error==='prerequisites_missing')return 'Finish DirectX and PhysX setup before playing. Your game files are installed.';
 if(s.error)return adoptLine(s)??'Resolve any folder-access prompt, then recheck status. Play was not retried.';
 const status=s.status;if(!status)return 'Checking Play availability…';
 const o=status.observation;
 if(status.prerequisites_installing)return 'Game component setup is running. Finish its installer before playing.';
 if(o?.phase==='unknown')return 'Game status is unknown. Play, Repair and removal remain blocked. Preserve files and restart to inspect.';
 if(o?.phase==='process_started')return 'Game process started. Login and world entry are not verified.';
 if(o?.phase==='host_started'||o?.phase==='preparing')return 'Preparing the game and waiting for its process…';
 if(o?.phase==='process_exited')return o.early?`Game exited shortly after starting (code ${o.code}).`:`Game exited (code ${o.code}).`;
 if(status.prerequisites_missing?.length)return `Complete ${status.prerequisites_missing.map(name=>name==='directx'?'DirectX':'PhysX').join(' and ')} setup before playing.`;
 if(o?.phase==='not_started')return 'Game could not start. Files are preserved; recheck before trying again. If you edited the login servers, check them under Login servers.';
 if(!status.resources_available)return 'Play unavailable: this build is missing verified game launch resources.';
 if(launchBlocked(status))return adoptLine(s)??'Another operation owns the game. Wait for it to finish.';
 // Play stays enabled: a skipped patch change is named, and Repair is the way to a full game.
 if(status.installation_id&&status.patches_incomplete>0)return `Ready to Play. ${status.ops_skipped} patch ${status.ops_skipped===1?'change':'changes'} could not be applied. If the game crashes or misbehaves, use Repair (downloads the full game, 4.1 GB).`;
 return status.installation_id?'Ready to Play.':'Finish installation and compatibility checks to enable Play.';
}
export function mountLaunch(document:Document,invoke:Invoke,onChange:()=>void=()=>{},uuid:()=>string=()=>crypto.randomUUID(),updateBlocked=false,onReadiness:(ready:boolean)=>void=()=>{}) {
 const runtime=ManagedRuntime.make(Layer.effect(Launch,makeLaunchWorkflow).pipe(Layer.provide(launchBridgeLayer(request=>invoke('launch_command',{request})))));
 const button=document.getElementById('launch') as HTMLButtonElement;
 const recheck=document.getElementById('inspect-launch') as HTMLButtonElement;
 const settingsCheck=document.getElementById('inspect-launch-settings') as HTMLButtonElement;
 const settingsStatus=document.getElementById('launch-settings-status')!;
 let current:LaunchState={status:null,pending:false,uncertain:true,error:null};let disposed=false;let pending=false;let playing=false;
 let mutation:Promise<void>=Promise.resolve();let revision=-1;
 let inspecting=false;
 const render=()=>{if(disposed)return;button.hidden=!!current.status&&!current.status.installation_id&&!current.status.observation&&!current.status.launcher_update_required;button.disabled=updateBlocked||playing||current.pending||current.uncertain||!current.status?.installation_id||launchBlocked(current.status);
 button.textContent=playing||current.pending?'Starting…':'Play';button.setAttribute('aria-busy',String(playing||current.pending));
 document.getElementById('launch-status')!.textContent=playing?'Starting game…':updateBlocked?'Resolve the launcher update above before playing.':launchText(current);
 const ready=!!current.status&&!current.uncertain&&!current.error&&gameSetupComplete(current.status);
 recheck.disabled=playing||current.pending;recheck.hidden=ready||adoptLine(current)!==null;
 settingsCheck.disabled=playing||current.pending||inspecting;
 settingsCheck.setAttribute('aria-busy',String(inspecting));
 settingsStatus.textContent=inspecting?'Checking Play status…':updateBlocked?'Resolve the launcher update before playing.':launchText(current);
 onReadiness(ready);};
 const watcher=runtime.runFork(Effect.flatMap(Launch,s=>Stream.runForEach(s.changes,value=>Effect.sync(()=>{current=value;render();
 const next=value.status?.native.operation.revision;if(next!==undefined&&next!==revision){revision=next;onChange();}}))));
 const run=(play:boolean)=>{
 if(disposed||(play?playing:pending||playing))return mutation;
 if(play)playing=true;else pending=true;render();
 mutation=runtime.runPromise(Effect.flatMap(Launch,s=>Effect.result(play?s.play(uuid()):s.inspect))).then(()=>{}).finally(()=>{if(play)playing=false;else pending=false;render();});return mutation;};
 const click=()=>{if(!button.disabled)void run(true);};const check=()=>{if(!recheck.disabled)void run(false);};
 const inspectSettings=()=>{if(settingsCheck.disabled)return;inspecting=true;render();void run(false).finally(()=>{inspecting=false;render();});};
 button.addEventListener('click',click);recheck.addEventListener('click',check);
 settingsCheck.addEventListener('click',inspectSettings);
 // Observation belongs to the mounted application, including Settings navigation.
 const poll=setInterval(()=>{if(!disposed&&!pending&&!current.uncertain&&!current.error)void run(false);},1000);
 const ready=run(false);
 return {ready,setUpdateBlocked:(blocked:boolean)=>{updateBlocked=blocked;render();},refresh:()=>run(false),settled:()=>mutation,dispose:async()=>{disposed=true;clearInterval(poll);button.removeEventListener('click',click);recheck.removeEventListener('click',check);settingsCheck.removeEventListener('click',inspectSettings);await Effect.runPromise(Fiber.interrupt(watcher));await mutation;await runtime.dispose();}};
}
