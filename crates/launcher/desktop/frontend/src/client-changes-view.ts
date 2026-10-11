import type {Invoke} from './view';
/** Settings panel, scaffold (LX-02): LX-07 replaces this body and talks to `client_changes_command`. */
export const clientChangesUnavailable='Changes to your client are not available in this build yet.';
export function mountClientChanges(document:Document,_invoke:Invoke) {
 const panel=document.getElementById('client-changes-panel');
 if(!panel)throw new Error('Missing launcher element client-changes-panel');
 const heading=document.createElement('h3');heading.id='client-changes-title';heading.textContent='Changes to your client';
 const status=document.createElement('p');status.id='client-changes-status';status.setAttribute('role','status');status.setAttribute('aria-live','polite');status.textContent=clientChangesUnavailable;
 panel.textContent='';panel.append(heading,status);
 const settled=Promise.resolve();
 return {ready:settled,refresh:()=>settled,settled:()=>settled,dispose:async()=>{panel.textContent='';}};
}
