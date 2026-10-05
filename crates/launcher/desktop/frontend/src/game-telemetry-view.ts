import {Context,Effect,Fiber,Layer,ManagedRuntime,Stream} from 'effect';
import {gameTelemetryBridgeLayer,GameTelemetryState,makeGameTelemetryWorkflow} from './game-telemetry-workflow';
import type {Invoke} from './view';
class GameTelemetry extends Context.Service<GameTelemetry,Effect.Success<typeof makeGameTelemetryWorkflow>>()('launcher/GameTelemetry') {}
export function gameTelemetryText(s:GameTelemetryState,saving:boolean|null=null):string {
 if(saving!==null)return saving?'Turning game diagnostics on…':'Turning game diagnostics off…';
 if(s.pending&&!s.status)return 'Checking game diagnostics…';
 if(s.error==='platform_unavailable')return 'This build does not include the game diagnostics module, so nothing was turned on.';
 if(s.error)return 'Your choice could not be confirmed. Recheck status before changing it; nothing was retried.';
 const status=s.status;if(!status)return 'Checking game diagnostics…';
 if(!status.available)return status.opted_in?'Game diagnostics are on, but this build does not include the module. Nothing is sent.':'Not available in this build. Nothing is sent.';
 if(!status.opted_in)return 'Off. Play loads no diagnostics module and sends nothing from the game.';
 const last=status.last_outcome;
 if(last==='attached')return 'On. The last Play loaded the diagnostics module with a server session. Takes effect each time you press Play.';
 if(last==='session_unavailable')return 'On, but the server gave no session for the last Play, so the game started without diagnostics.';
 if(last==='endpoint_refused')return 'On, but the last Play refused the server address offered for diagnostics, so nothing was sent.';
 if(last==='session_not_written')return 'On, but the session could not be saved beside the game for the last Play, so it started without diagnostics.';
 return 'On. The next Play loads the diagnostics module into the game and sends game events and logs to your login server.';
}
export function mountGameTelemetry(document:Document,invoke:Invoke) {
 const runtime=ManagedRuntime.make(Layer.effect(GameTelemetry,makeGameTelemetryWorkflow).pipe(Layer.provide(gameTelemetryBridgeLayer(request=>invoke('game_telemetry_command',{request})))));
 const checkbox=document.getElementById('game-telemetry') as HTMLInputElement;
 const recheck=document.getElementById('inspect-game-telemetry') as HTMLButtonElement;
 let current:GameTelemetryState={status:null,pending:false,uncertain:true,error:null};let disposed=false;let pending=false;let saving:boolean|null=null;let mutation:Promise<void>=Promise.resolve();
 const render=()=>{if(disposed)return;const busy=pending||current.pending;
  // The box shows the pressed choice at once, then the confirmed one.
  checkbox.checked=saving??current.status?.opted_in??false;
  // Opting out stays possible in a build without the module.
  checkbox.disabled=busy||current.uncertain||!current.status||(!current.status.available&&!current.status.opted_in);
  checkbox.setAttribute('aria-busy',String(saving!==null));
  document.getElementById('game-telemetry-status')!.textContent=gameTelemetryText(current,saving);
  recheck.disabled=busy;recheck.hidden=!current.error&&!current.uncertain;};
 const watcher=runtime.runFork(Effect.flatMap(GameTelemetry,s=>Stream.runForEach(s.changes,value=>Effect.sync(()=>{current=value;render();}))));
 const run=(choice:boolean|null)=>{if(disposed||pending)return mutation;pending=true;saving=choice;render();
  mutation=runtime.runPromise(Effect.flatMap(GameTelemetry,s=>Effect.result(choice===null?s.inspect:s.set(choice)))).then(()=>{}).finally(()=>{pending=false;saving=null;render();});return mutation;};
 const change=()=>{if(checkbox.disabled){render();return;}void run(checkbox.checked);};const check=()=>{if(!recheck.disabled)void run(null);};
 checkbox.addEventListener('change',change);recheck.addEventListener('click',check);
 const ready=run(null);
 return {ready,refresh:()=>run(null),settled:()=>mutation,dispose:async()=>{disposed=true;checkbox.removeEventListener('change',change);recheck.removeEventListener('click',check);await Effect.runPromise(Fiber.interrupt(watcher));await mutation;await runtime.dispose();}};
}
