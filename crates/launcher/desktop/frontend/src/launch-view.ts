import {Context,Effect,Fiber,Layer,ManagedRuntime,Stream} from 'effect';
import {launchBlocked,launchBridgeLayer,LaunchState,makeLaunchWorkflow} from './launch-workflow';
import type {Invoke} from './view';
class Launch extends Context.Service<Launch,Effect.Success<typeof makeLaunchWorkflow>>()('launcher/Play') {}
export function launchText(s:LaunchState):string {
 if(s.pending)return 'Starting game…';
 if(s.error==='launcher_too_old'||s.status?.launcher_update_required)return 'Update the launcher before playing this release. Your game files are preserved.';
 if(s.error)return 'Could not read game status. Resolve any folder-access prompt, then recheck.';
 const status=s.status;if(!status)return 'Checking Play availability…';
 const o=status.observation;
 if(o?.phase==='unknown')return 'Game status is unknown. Play, Repair and removal remain blocked. Preserve files and restart to inspect.';
 if(o?.phase==='process_started')return 'Game process started. Login and world entry are not verified.';
 if(o?.phase==='host_started'||o?.phase==='preparing')return 'Preparing the game and waiting for its process…';
 if(o?.phase==='process_exited')return o.early?`Game exited shortly after starting (code ${o.code}).`:`Game exited (code ${o.code}).`;
 if(o?.phase==='not_started')return 'Game could not start. Files are preserved; recheck before trying again.';
 if(!status.resources_available)return 'Play unavailable: this build is missing verified game launch resources.';
 if(launchBlocked(status))return 'Another operation owns the game. Wait for it to finish.';
 return status.installation_id?'Ready to Play.':'Finish installation and compatibility checks to enable Play.';
}
export function mountLaunch(document:Document,invoke:Invoke,onChange:()=>void=()=>{},uuid:()=>string=()=>crypto.randomUUID()) {
 const runtime=ManagedRuntime.make(Layer.effect(Launch,makeLaunchWorkflow).pipe(Layer.provide(launchBridgeLayer(request=>invoke('launch_command',{request})))));
 const button=document.getElementById('launch') as HTMLButtonElement;
 const recheck=document.getElementById('inspect-launch') as HTMLButtonElement;
 let current:LaunchState={status:null,pending:false,uncertain:true,error:null};let disposed=false;let pending=false;let playing=false;
 let mutation:Promise<void>=Promise.resolve();let revision=-1;
 const render=()=>{if(disposed)return;button.hidden=!!current.status&&!current.status.installation_id&&!current.status.observation&&!current.status.launcher_update_required;button.disabled=pending||current.pending||current.uncertain||!current.status?.installation_id||launchBlocked(current.status);
 button.textContent=playing||current.pending?'Starting…':'Play';button.setAttribute('aria-busy',String(pending||current.pending));
 document.getElementById('launch-status')!.textContent=playing?'Starting game…':launchText(current);
 recheck.disabled=pending||current.pending;};
 const watcher=runtime.runFork(Effect.flatMap(Launch,s=>Stream.runForEach(s.changes,value=>Effect.sync(()=>{current=value;render();
 const next=value.status?.native.operation.revision;if(next!==undefined&&next!==revision){revision=next;onChange();}}))));
 const run=(play:boolean)=>{if(disposed||pending)return mutation;pending=true;playing=play;render();
 mutation=runtime.runPromise(Effect.flatMap(Launch,s=>Effect.result(play?s.play(uuid()):s.inspect))).then(()=>{}).finally(()=>{pending=false;playing=false;render();});return mutation;};
 const click=()=>{if(!button.disabled)void run(true);};const check=()=>{if(!recheck.disabled)void run(false);};
 button.addEventListener('click',click);recheck.addEventListener('click',check);
 // Observation belongs to the mounted application, including Settings navigation.
 const poll=setInterval(()=>{if(!disposed&&!pending&&!current.uncertain&&!current.error)void run(false);},1000);
 const ready=run(false);
 return {ready,refresh:()=>run(false),settled:()=>mutation,dispose:async()=>{disposed=true;clearInterval(poll);button.removeEventListener('click',click);recheck.removeEventListener('click',check);await Effect.runPromise(Fiber.interrupt(watcher));await mutation;await runtime.dispose();}};
}
