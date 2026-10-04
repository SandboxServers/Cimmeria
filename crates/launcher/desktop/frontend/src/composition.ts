import { mountUpdater } from './updater-view';
import { mountMigration } from './migration-view';
import { mountLaunch } from './launch-view';
import { mountInstall } from './install-view';
import { mountLauncher, Invoke } from './view';

/** Mount the launcher once; operation revisions refresh dependent capabilities. */
export function mountDesktop(document:Document,invoke:Invoke) {
const installation = mountInstall(document,invoke);
const app = mountLauncher(document,invoke,()=>{void installation.refresh();});
let updater: ReturnType<typeof mountUpdater> | undefined;
const play = mountLaunch(document,invoke,()=>{void installation.refresh(); void app.refresh(); void updater?.refresh();});
const migration = mountMigration(document,invoke,()=>{void app.refresh(); void installation.refresh(); void play.refresh();});
updater = mountUpdater(document,invoke,()=>{void app.refresh(); void installation.refresh(); void play.refresh();});
return {installation, app, play, migration, updater,
 dispose:async()=>{await Promise.all([migration.dispose(),play.dispose(),app.dispose(),installation.dispose(),updater!.dispose()]);}};
}
