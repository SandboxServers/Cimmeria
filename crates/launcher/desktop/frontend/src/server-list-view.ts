import type {Invoke} from './view';
/** Settings panel, scaffold (LX-02): LX-03 replaces this body and talks to `launcher_command` (`save_client_options`). */
export const serverListUnavailable='Login servers are not available in this build yet.';
export function mountServerList(document:Document,_invoke:Invoke) {
 const panel=document.getElementById('server-list-panel');
 if(!panel)throw new Error('Missing launcher element server-list-panel');
 const heading=document.createElement('h3');heading.id='server-list-title';heading.textContent='Login servers';
 const status=document.createElement('p');status.id='server-list-status';status.setAttribute('role','status');status.setAttribute('aria-live','polite');status.textContent=serverListUnavailable;
 panel.textContent='';panel.append(heading,status);
 const settled=Promise.resolve();
 return {ready:settled,refresh:()=>settled,settled:()=>settled,dispose:async()=>{panel.textContent='';}};
}
