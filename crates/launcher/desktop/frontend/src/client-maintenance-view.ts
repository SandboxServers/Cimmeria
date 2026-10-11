import {Effect} from 'effect';
import type {Invoke} from './view';
import type {PanelContext} from './panel-context';
import {clientMaintenanceFailure,decodeClientMaintenanceStatus,type ClientMaintenanceCommand,type ClientMaintenanceError,type ClientMaintenanceStatus} from './contract';

/** Settings panel for `client_maintenance_command` (LX-06). The second reset deletes every client setting, so it sends nothing until the confirm step is accepted. */
export const clientMaintenanceText={
 cacheBusy:'Clearing the client cache…',allBusy:'Deleting all client state…',
 cacheDone:'Cache cleared.',allDone:'Client state deleted. The game starts fresh next time.',nothing:'Nothing to clear.',
 confirm:'This deletes all of the game client\'s settings, key bindings and cache on this computer. Your characters on the server are not affected.',
 confirmOpen:'Confirm below to delete all client state, or cancel.',cancelled:'Reset cancelled. Nothing was deleted.',
};
const failures:Record<ClientMaintenanceError|'transport'|'schema',string>={
 confirmation_required:'Confirm the reset first. Nothing was deleted.',
 profile_unavailable:'Your user profile folder could not be found, so nothing was deleted.',
 unsafe_path:'The cache folder is reached through a link, so nothing was deleted. Use Reset all client state, or remove the link yourself.',
 busy:'Another launcher operation is running. Wait for it to finish, then try again.',
 io:'Some files could not be deleted. Close the game and try again.',
 in_use:'The launcher\'s state is in use by another launcher window. Close it and try again.',
 corrupt:'The launcher\'s state could not be read. Nothing was deleted.',
 unsupported_schema:'This launcher build does not understand the request. Update the launcher.',
 too_large:'The launcher could not finish the reset (too_large).',unsafe_file:'The launcher could not finish the reset (unsafe_file).',
 invalid_directory:'The launcher could not finish the reset (invalid_directory).',stale_revision:'The launcher could not finish the reset (stale_revision).',
 persistence_uncertain:'The launcher could not finish the reset (persistence_uncertain).',
 transport:'The launcher could not reach its native side. Try again.',schema:'The launcher got an answer it does not understand.',
};
export const clientMaintenanceFailureText=(code:keyof typeof failures)=>failures[code];

export function mountClientMaintenance(document:Document,invoke:Invoke,_context:PanelContext) {
 const panel=document.getElementById('client-maintenance-panel');
 if(!panel)throw new Error('Missing launcher element client-maintenance-panel');
 const make=<K extends keyof HTMLElementTagNameMap>(tag:K,id:string,text:string)=>{const el=document.createElement(tag);el.id=id;el.textContent=text;return el;};
 const heading=make('h3','client-maintenance-title','Client cache and settings');
 const about=make('p','client-maintenance-about','Clear the cache if the game shows old content after a server change. Reset all client state if the game will not start or its settings are broken.');
 const resetCache=make('button','client-maintenance-reset-cache','Reset client cache');resetCache.type='button';
 const resetAll=make('button','client-maintenance-reset-all','Reset all client state…');resetAll.type='button';
 const confirm=make('div','client-maintenance-confirm','');confirm.setAttribute('role','group');confirm.setAttribute('aria-labelledby','client-maintenance-confirm-text');confirm.hidden=true;
 const confirmText=make('p','client-maintenance-confirm-text',clientMaintenanceText.confirm);
 const confirmYes=make('button','client-maintenance-confirm-yes','Delete all client state');confirmYes.type='button';
 const confirmNo=make('button','client-maintenance-confirm-no','Cancel');confirmNo.type='button';
 confirm.append(confirmText,confirmYes,confirmNo);
 const status=make('p','client-maintenance-status','');status.setAttribute('role','status');status.setAttribute('aria-live','polite');
 panel.textContent='';panel.append(heading,about,resetCache,resetAll,confirm,status);

 let latest:ClientMaintenanceStatus|undefined;
 let pending:Promise<void>=Promise.resolve();
 let running=false,disposed=false;
 const render=()=>{
  for(const button of [resetCache,resetAll,confirmYes,confirmNo]){button.disabled=running;button.setAttribute('aria-busy',String(running));}
  if(latest)confirmText.textContent=`${clientMaintenanceText.confirm} Folder: ${latest.client_state.path}`;
 };
 type Answer={ok:ClientMaintenanceStatus}|{error:keyof typeof failures};
 const send=async(command:ClientMaintenanceCommand):Promise<Answer>=>{
  let value:unknown;
  try{value=await invoke('client_maintenance_command',{request:command});}
  catch(error){return {error:clientMaintenanceFailure(error)};}
  const ok=await Effect.runPromise(decodeClientMaintenanceStatus(value).pipe(Effect.match({onFailure:()=>undefined,onSuccess:decoded=>decoded})));
  return ok?{ok}:{error:'schema'};
 };
 const run=(command:ClientMaintenanceCommand,busyText:string,doneText:string)=>{
  if(running)return pending;
  running=true;confirm.hidden=true;status.textContent=busyText;render();
  pending=send(command).then(answer=>{
   if(disposed)return;
   if('ok' in answer){latest=answer.ok;status.textContent=answer.ok.removed?doneText:clientMaintenanceText.nothing;}
   else status.textContent=clientMaintenanceFailureText(answer.error);
  }).finally(()=>{running=false;if(!disposed)render();});
  return pending;
 };
 const inspect=()=>send({command:'inspect',schema_version:1}).then(answer=>{if(!disposed&&'ok' in answer){latest=answer.ok;render();}});

 const onCache=()=>{void run({command:'reset_cache',schema_version:1},clientMaintenanceText.cacheBusy,clientMaintenanceText.cacheDone);};
 // The first press only opens the confirm step, and inspects to name the folder in it;
 // no reset is sent until "Delete all client state".
 const onAll=()=>{if(running)return;confirm.hidden=false;status.textContent=clientMaintenanceText.confirmOpen;confirmYes.focus?.();pending=inspect();};
 const onYes=()=>{void run({command:'reset_all_client_state',schema_version:1,confirmed:true},clientMaintenanceText.allBusy,clientMaintenanceText.allDone);};
 const onNo=()=>{if(running)return;confirm.hidden=true;status.textContent=clientMaintenanceText.cancelled;resetAll.focus?.();};
 resetCache.addEventListener('click',onCache);resetAll.addEventListener('click',onAll);
 confirmYes.addEventListener('click',onYes);confirmNo.addEventListener('click',onNo);
 render();
 // Nothing is asked of native code until a button is pressed. A reset changes no preference
 // or operation, so the shared snapshot needs no refresh either.
 const ready=Promise.resolve();
 return {
  ready,
  refresh:()=>pending,
  settled:()=>pending,
  dispose:async()=>{
   disposed=true;
   resetCache.removeEventListener('click',onCache);resetAll.removeEventListener('click',onAll);
   confirmYes.removeEventListener('click',onYes);confirmNo.removeEventListener('click',onNo);
   await pending.catch(()=>{});panel.textContent='';
  },
 };
}
