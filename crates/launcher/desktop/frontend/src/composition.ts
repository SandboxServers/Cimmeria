import { mountGameUpdate } from './game-update-view';
import { mountUpdater } from './updater-view';
import { mountMigration } from './migration-view';
import { mountAdoption } from './adoption-view';
import { mountLaunch } from './launch-view';
import { mountGameTelemetry } from './game-telemetry-view';
import { mountInstall } from './install-view';
import { mountLauncher, Invoke } from './view';

/** Mount the launcher once; operation revisions refresh dependent capabilities. */
export function mountDesktop(document:Document,invoke:Invoke) {
const installation = mountInstall(document,invoke);
const app = mountLauncher(document,invoke,()=>{void installation.refresh();});
let updater: ReturnType<typeof mountUpdater> | undefined;
// Its own record: it never reads or writes the launcher-summary choice.
const gameTelemetry = mountGameTelemetry(document,invoke);
// A finished Play changes what the last launch did about game diagnostics.
const play = mountLaunch(document,invoke,()=>{void installation.refresh(); void app.refresh(); void updater?.refresh(); void gameTelemetry.refresh();});
let adoption: ReturnType<typeof mountAdoption> | undefined;
const migration = mountMigration(document,invoke,()=>{void app.refresh(); void installation.refresh(); void play.refresh(); void adoption?.refresh();});
// Adoption notifies only when a saved revision changed, so this cannot loop.
adoption = mountAdoption(document,invoke,()=>{void app.refresh(); void installation.refresh(); void play.refresh();});
updater = mountUpdater(document,invoke,()=>{void app.refresh(); void installation.refresh(); void play.refresh();});
const gameUpdate = mountGameUpdate(document,invoke,()=>{void app.refresh();void installation.refresh();void play.refresh();void updater?.refresh();});
return {installation, app, play, migration, adoption, updater, gameUpdate, gameTelemetry,
 dispose:async()=>{await Promise.all([gameTelemetry.dispose(),gameUpdate.dispose(),adoption!.dispose(),migration.dispose(),play.dispose(),app.dispose(),installation.dispose(),updater!.dispose()]);}};
}
