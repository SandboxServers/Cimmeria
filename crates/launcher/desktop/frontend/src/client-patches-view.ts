import type {Invoke} from './view';
import type {PanelContext} from './panel-context';
import {scaffoldFailure} from './contract';
/**
 * Settings panel (LX-04): "Load client patches", saved through `launcher_command`
 * (`save_client_options`) with the saved login servers carried over unchanged. Off means Play
 * injects no client-patches DLL into a copy this launcher installed; an adopted copy keeps the
 * setting it was adopted with, so a save there is allowed but does not change that copy.
 */
const saveErrorText:Record<string,string>={
 stale_revision:'Settings changed while saving, so nothing was saved. The box shows the saved setting; change it again if you still want to.',
 busy:'The launcher is installing, repairing or updating. Change the setting again when it finishes.',
 persistence_uncertain:'The save could not be confirmed. Restart the launcher to check the saved setting.',
 in_use:'Another launcher is using these settings. Close it, then try again.',
 io:'The setting could not be saved. Check permissions and free disk space.',
};
export function mountClientPatches(document:Document,invoke:Invoke,context:PanelContext) {
 const panel=document.getElementById('client-patches-panel');
 if(!panel)throw new Error('Missing launcher element client-patches-panel');
 const make=<K extends keyof HTMLElementTagNameMap>(tag:K,id:string)=>{const e=document.createElement(tag);e.id=id;return e;};
 const heading=make('h3','client-patches-title');heading.textContent='Client patches';
 const box=make('input','client-patches-enabled');box.type='checkbox';box.disabled=true;
 const label=make('label','client-patches-label');label.setAttribute('for','client-patches-enabled');label.append(box,' Load client patches');
 const hint=make('p','client-patches-hint');
 hint.textContent='Client patches are fixes the launcher loads into the game when it starts. Turn this off to play the unpatched client.';
 // Same rule as the server list (LX-03): an adopted copy plays with what its adoption reviewed.
 const kept=make('p','client-patches-adopted');kept.hidden=true;
 const status=make('p','client-patches-status');status.setAttribute('role','status');status.setAttribute('aria-live','polite');
 panel.textContent='';panel.append(heading,kept,label,hint,status);
 let saving=false;let disposed=false;let mutation:Promise<void>=Promise.resolve();
 const adopted=()=>(context.snapshot()?.adopted_login_servers??null)!==null;
 // The box shows the saved value whenever no save is in flight, so a refused save puts it back.
 const sync=()=>{
  const snapshot=context.snapshot();if(disposed)return;
  if(!snapshot){box.disabled=true;return;}
  kept.hidden=!adopted();
  kept.textContent=adopted()?'This copy was adopted, so Play keeps the client-patches setting it was adopted with. Changing the box below does not change this copy.':'';
  if(saving)return;
  box.disabled=false;box.checked=snapshot.preferences.client_patches_enabled;
 };
 const unsubscribe=context.onSnapshot(()=>sync());
 const onChange=()=>{
  if(disposed||saving)return;
  const snapshot=context.snapshot();
  if(!snapshot){box.checked=!box.checked;status.textContent='Settings are still loading. Try again in a moment.';return;}
  const enabled=box.checked;
  saving=true;box.disabled=true;box.setAttribute('aria-busy','true');status.textContent='Saving client patches setting…';
  const request={command:'save_client_options',schema_version:1,expected_revision:snapshot.preferences.revision,
   login_servers:snapshot.preferences.login_servers,client_patches_enabled:enabled};
  let unconfirmed=false;
  mutation=invoke('launcher_command',{request}).then(async()=>{
   const reloaded=await context.refresh().then(()=>true,()=>false);
   if(disposed)return;
   // The old snapshot must not put the previous value back over what was saved.
   if(!reloaded){unconfirmed=true;status.textContent='Saved, but the launcher could not reload settings. Restart the launcher to see the saved setting.';return;}
   status.textContent=adopted()?'Saved, but this copy keeps the client-patches setting it was adopted with.'
    :enabled?'Saved. The next Play loads client patches.':'Saved. The next Play starts the game without client patches.';
  },async failure=>{
   const code=scaffoldFailure(failure);
   if(code==='stale_revision')await context.refresh().catch(()=>{});
   if(!disposed)status.textContent=saveErrorText[code]??'The setting could not be saved. Recheck status, then try again.';
  }).finally(()=>{saving=false;if(disposed)return;box.setAttribute('aria-busy','false');
   if(unconfirmed){box.disabled=false;return;}
   sync();});
 };
 box.addEventListener('change',onChange);
 sync();
 const ready=Promise.resolve();
 return {ready,refresh:()=>{sync();return mutation;},settled:()=>mutation,dispose:async()=>{
  disposed=true;unsubscribe();box.removeEventListener('change',onChange);await mutation;panel.textContent='';}};
}
