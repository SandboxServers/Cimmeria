import { mountDesktop } from './composition';
import type { Invoke } from './view';
declare global { interface Window { __TAURI__: { core: { invoke: Invoke } } } }
const desktop = mountDesktop(document,(command,args)=>window.__TAURI__.core.invoke(command,args));
window.addEventListener('pagehide',()=>{void desktop.dispose();},{once:true});
