import { mountInstall } from './install-view';
import { mountLauncher, Invoke } from './view';

declare global {
  interface Window { __TAURI__: { core: { invoke: Invoke } } }
}
const invoke: Invoke = (command,args) => window.__TAURI__.core.invoke(command,args);
const installation = mountInstall(document,invoke);
const app = mountLauncher(document,invoke,()=>{void installation.refresh();});
window.addEventListener('pagehide', () => { void app.dispose(); void installation.dispose(); }, {once:true});
