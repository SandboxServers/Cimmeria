import { mountMigration } from './migration-view';
import { mountLaunch } from './launch-view';
import { mountInstall } from './install-view';
import { mountLauncher, Invoke } from './view';

declare global {
  interface Window { __TAURI__: { core: { invoke: Invoke } } }
}
const invoke: Invoke = (command,args) => window.__TAURI__.core.invoke(command,args);
const installation = mountInstall(document,invoke);
const app = mountLauncher(document,invoke,()=>{void installation.refresh();});
const play = mountLaunch(document,invoke,()=>{void installation.refresh(); void app.refresh();});
const migration = mountMigration(document,invoke,()=>{void app.refresh(); void installation.refresh(); void play.refresh();});
window.addEventListener('pagehide', () => { void migration.dispose(); void play.dispose(); void app.dispose(); void installation.dispose(); }, {once:true});
