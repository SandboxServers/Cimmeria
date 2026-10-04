import { mountLauncher, Invoke } from './view';

declare global {
  interface Window { __TAURI__: { core: { invoke: Invoke } } }
}
const app = mountLauncher(document, (command, args) => window.__TAURI__.core.invoke(command, args));
window.addEventListener('pagehide', () => { void app.dispose(); }, {once:true});
