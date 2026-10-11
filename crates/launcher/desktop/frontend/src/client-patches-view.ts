import type {Invoke} from './view';
/** Settings panel, scaffold (LX-02): LX-04 replaces this body and talks to `launcher_command` (`save_client_options`). */
export const clientPatchesUnavailable='Client patches are not available in this build yet.';
export function mountClientPatches(document:Document,_invoke:Invoke) {
 const panel=document.getElementById('client-patches-panel');
 if(!panel)throw new Error('Missing launcher element client-patches-panel');
 const heading=document.createElement('h3');heading.id='client-patches-title';heading.textContent='Client patches';
 const status=document.createElement('p');status.id='client-patches-status';status.setAttribute('role','status');status.setAttribute('aria-live','polite');status.textContent=clientPatchesUnavailable;
 panel.textContent='';panel.append(heading,status);
 const settled=Promise.resolve();
 return {ready:settled,refresh:()=>settled,settled:()=>settled,dispose:async()=>{panel.textContent='';}};
}
