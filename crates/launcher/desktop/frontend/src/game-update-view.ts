import {Context,Effect,Fiber,Layer,ManagedRuntime,Stream} from 'effect';
import {Action,GameUpdateState,gameUpdateBridgeLayer,makeGameUpdateWorkflow} from './game-update-workflow';
import type {Invoke} from './view';
class GameUpdate extends Context.Service<GameUpdate,Effect.Success<typeof makeGameUpdateWorkflow>>()('launcher/GameUpdate') {}
const actions:Action[]=['apply','recover','abandon','discard','cleanup','rollback'];
const labels:Record<Action,string>={apply:'Update game',recover:'Recover update',abandon:'Abandon preparation',discard:'Remove partial update',cleanup:'Remove old backup',rollback:'Restore previous release'};
const consequences:Record<Action,string>={
 apply:'Rebuild the game from the signed release and replace the current game folder. Local modifications and game-local saves remain only in the retained old backup; they are not merged into the new game.',
 rollback:'Download and rebuild the previous signed release. This does not restore local modifications from the old backup. The current game folder will become a separate backup.',
 recover:'Resume the recorded file replacement and release publication. This does not start another download. Recovery is refused if the recorded files cannot be verified.',
 abandon:'Stop an interrupted preparation while keeping its files and the current game. This is unavailable once game replacement has begun.',
 discard:'Permanently remove verified partial files from this failed or cancelled update. The current game and retained backups remain.',
 cleanup:'Permanently remove the old game backup, including its local modifications and game-local saves. The current game remains.',
};
export function gameUpdateText(s:GameUpdateState):string {
 if(s.pending)return 'Working on the game update request…';
 if(s.status?.native.requires_reopen)return 'Restart the launcher to inspect the saved update state.';
 if(s.error)return 'The update result could not be confirmed. Recheck status before continuing.';
 const op=s.status?.native.operation.operation;
 if(op?.kind==='update'){
  if(op.state==='reconciliation_required')return 'The update was interrupted. Review recovery or abandon preparation before continuing.';
  if(op.state==='cancel_requested')return 'Cancelling preparation. Waiting for the saved result…';
  if(op.state==='starting'||op.state==='running')return 'Preparing and verifying the signed game release. The current game is retained until replacement.';
  if(op.state==='failed'||op.state==='cancelled')return 'Update preparation did not complete. Review partial-file cleanup or check for a fresh offer.';
 }
 if(s.status?.offer?.launcher_update_required)return 'Update the launcher before applying this game release.';
 if(s.status?.offer)return 'A different signed game release is available. Review the replacement before applying it.';
 if(s.status?.checked)return 'The installed game matches the signed release.';
 return s.status?.can_check?'Check for a signed game release.':'Game updates become available after a verified installation and any active operation finishes.';
}
export function mountGameUpdate(document:Document,invoke:Invoke,onChange:()=>void=()=>{}) {
 const runtime=ManagedRuntime.make(Layer.effect(GameUpdate,makeGameUpdateWorkflow).pipe(Layer.provide(gameUpdateBridgeLayer(request=>invoke('game_update_command',{request})))));
 const get=(id:string)=>document.getElementById(id)!;
 const button=(id:string)=>get(id) as HTMLButtonElement;
 let current:GameUpdateState={status:null,pending:false,uncertain:true,error:null,review:null};
 let disposed=false,mutating=false,reading=false,pollable=true;
 let mutation:Promise<void>=Promise.resolve(),inspection:Promise<void>=Promise.resolve();
 const render=()=>{
  if(disposed)return;
  const s=current.status,m=s?.maintenance,op=s?.native.operation.operation,busy=mutating||current.pending;
  const enabled=!busy&&!current.uncertain;
  get('game-update-status').textContent=gameUpdateText(current);
  button('check-game-update').disabled=!enabled||!s?.can_check;
  button('inspect-game-update').disabled=busy;
  const available:Record<Action,boolean>={apply:!!s?.offer&&!s.offer.launcher_update_required,recover:!!m?.recovery,abandon:!!m?.recovery,discard:!!m?.discard,cleanup:!!m&&['retained','cleanup_pending'].includes(m.backup),rollback:!!m?.rollback};
  for(const action of actions){const b=button(`${action}-game-update`);b.hidden=!available[action];b.disabled=!enabled;}
  button('cancel-game-update').hidden=op?.kind!=='update'||!['starting','running','cancel_requested'].includes(op.state);
  button('cancel-game-update').disabled=!enabled||op?.state==='cancel_requested';
  const progress=get('game-update-progress') as HTMLProgressElement;
  progress.hidden=!s?.progress;
  if(s?.progress){progress.max=Math.max(1,s.progress.total);progress.value=s.progress.current;}
  get('game-update-review').hidden=!current.review;
  if(current.review){
   const r=current.review;
   get('game-update-review-title').textContent=labels[r.action];
   get('game-update-directory').textContent=r.directory;
   get('game-update-identities').hidden=!['apply','recover','rollback'].includes(r.action);
   get('game-update-identities').textContent=`Current signed release: ${r.from}\nTarget signed release: ${r.to}`;
   get('game-update-consequences').textContent=consequences[r.action];
   button('confirm-game-update').textContent=labels[r.action];
  }
  button('confirm-game-update').disabled=!enabled||!current.review;
  button('dismiss-game-update').disabled=busy;
 };
 const watcher=runtime.runFork(Effect.flatMap(GameUpdate,w=>Stream.runForEach(w.changes,s=>Effect.sync(()=>{current=s;render();}))));
 const refresh=()=>{
  if(disposed||reading||mutating)return inspection;
  reading=true;
  inspection=runtime.runPromise(Effect.flatMap(GameUpdate,w=>Effect.result(w.inspect))).then(result=>{pollable=result._tag==='Success';}).finally(()=>{reading=false;render();});
  return inspection;
 };
 const run=(effect:Effect.Effect<unknown,unknown,GameUpdate>,changed=false)=>{
  if(disposed||mutating)return mutation;
  mutating=true;render();
  mutation=runtime.runPromise(Effect.result(effect)).then(()=>{if(changed)onChange();}).finally(()=>{mutating=false;render();});
  return mutation;
 };
 const listeners:(()=>void)[]=[];
 const on=(id:string,fn:()=>void)=>{const b=button(id),listener=()=>{if(!b.disabled)fn();};b.addEventListener('click',listener);listeners.push(()=>b.removeEventListener('click',listener));};
 on('inspect-game-update',()=>{void refresh();});
 on('check-game-update',()=>{void run(Effect.flatMap(GameUpdate,w=>w.check));});
 for(const action of actions)on(`${action}-game-update`,()=>{void run(Effect.flatMap(GameUpdate,w=>w.review(action)));});
 on('dismiss-game-update',()=>{void run(Effect.flatMap(GameUpdate,w=>w.dismiss));});
 on('confirm-game-update',()=>{void run(Effect.flatMap(GameUpdate,w=>w.confirm(crypto.randomUUID())),true);});
 on('cancel-game-update',()=>{void run(Effect.flatMap(GameUpdate,w=>w.cancel),true);});
 const ready=refresh();
 const timer=setInterval(()=>{const op=current.status?.native.operation.operation;if(pollable&&op?.kind==='update'&&['starting','running','cancel_requested'].includes(op.state))void refresh().then(onChange);},1000);
 return {ready,refresh,settled:()=>Promise.all([inspection,mutation]),dispose:async()=>{disposed=true;clearInterval(timer);listeners.forEach(remove=>remove());await Effect.runPromise(Fiber.interrupt(watcher));await Promise.all([inspection,mutation]);await runtime.dispose();}};
}
