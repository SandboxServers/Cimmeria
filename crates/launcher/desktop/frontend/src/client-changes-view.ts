import {Effect,Schema} from 'effect';
import type {Invoke} from './view';
import type {PanelContext} from './panel-context';
import {ClientChangesStatus,scaffoldFailure,type ClientChangesCommand} from './contract';
/**
 * Settings panel (LX-07): the read-only "Changes to your client" list. Native
 * code builds every row from the installed copy (`client_changes_command`);
 * this view only renders them, and re-reads on each snapshot because an
 * install, Update or settings change can change the list.
 */
export const clientChangesUnavailable='Changes to your client are not available in this build yet.';
export const clientChangesNothingInstalled='Nothing is installed yet. Once the game is installed, this lists every way the launcher changes it from the stock 2009 client.';
export const clientChangesLoading='Reading the changes to your client...';
const failureLine=(code:string)=>code==='unavailable'?clientChangesUnavailable
 :code==='busy'?'The installation is busy with another operation. This list updates when it finishes.'
 :code==='persistence_uncertain'?'The launcher could not confirm its saved state. Reopen the launcher to see the changes to your client.'
 :`Could not read the changes to your client (${code}).`;
const inspect:ClientChangesCommand={command:'inspect',schema_version:1};
const decode=Schema.decodeUnknownEffect(ClientChangesStatus,{onExcessProperty:'error'});
export function mountClientChanges(document:Document,invoke:Invoke,context:PanelContext) {
 const panel=document.getElementById('client-changes-panel');
 if(!panel)throw new Error('Missing launcher element client-changes-panel');
 const heading=document.createElement('h3');heading.id='client-changes-title';heading.textContent='Changes to your client';
 const status=document.createElement('p');status.id='client-changes-status';status.setAttribute('role','status');status.setAttribute('aria-live','polite');status.textContent=clientChangesLoading;
 const list=document.createElement('div');list.id='client-changes-list';
 panel.textContent='';panel.append(heading,status,list);
 let disposed=false;
 const render=(value:ClientChangesStatus)=>{
  list.textContent='';
  if(!value.installed){status.textContent=clientChangesNothingInstalled;return;}
  status.textContent=`${value.changes.length} changes to this copy of the game, from the launcher, its patches and the server.`;
  let group:HTMLUListElement|undefined;let current='';
  for(const row of value.changes){
   if(!group||row.group!==current){
    current=row.group;
    const title=document.createElement('h4');title.textContent=row.heading;
    group=document.createElement('ul');group.dataset.group=row.group;
    list.append(title,group);
   }
   const item=document.createElement('li');item.dataset.status=row.status;
   if(row.patch_id!==null)item.dataset.patchId=row.patch_id;
   const name=document.createElement('strong');name.textContent=row.title;
   const state=document.createElement('span');state.className='client-change-status';state.textContent=` (${row.status_label})`;
   const description=document.createElement('p');description.textContent=row.description;
   item.append(name,state,description);group.append(item);
  }
 };
 const load=async()=>{
  let value:ClientChangesStatus;
  try{value=await Effect.runPromise(decode(await invoke('client_changes_command',{request:inspect})));}
  catch(error){
   if(disposed)return;
   // A schema mismatch is not a native code string, so it reads as transport.
   list.textContent='';status.textContent=failureLine(scaffoldFailure(error));return;
  }
  if(!disposed)render(value);
 };
 // One read at a time; a snapshot that lands mid-read queues exactly one more.
 let chain:Promise<void>=Promise.resolve();let queued=false;
 const refresh=()=>{
  if(queued)return chain;
  queued=true;
  chain=chain.then(()=>{queued=false;return disposed?undefined:load();});
  return chain;
 };
 // The launcher's own inspect publishes the first snapshot, so the list is read
 // then rather than racing it at mount; `ready` is that first read.
 let first!:()=>void;const ready=new Promise<void>(resolve=>{first=resolve;});
 const follow=()=>{void refresh().then(first);};
 const unsubscribe=context.onSnapshot(follow);
 if(context.snapshot())follow();
 // Never waits on a pending read: native may not answer before the window closes.
 return {ready,refresh,settled:()=>chain,dispose:async()=>{disposed=true;unsubscribe();first();panel.textContent='';}};
}
