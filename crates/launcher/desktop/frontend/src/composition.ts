import { mountGameUpdate } from './game-update-view';
import { mountUpdater } from './updater-view';
import { mountMigration } from './migration-view';
import { mountAdoption } from './adoption-view';
import { mountLaunch } from './launch-view';
import { mountGameTelemetry } from './game-telemetry-view';
import { mountTelemetryStatus } from './telemetry-status-view';
import { mountInstall } from './install-view';
import { mountLauncher, Invoke } from './view';

/** Mount the launcher once; operation revisions refresh dependent capabilities. */
export function mountDesktop(document:Document,invoke:Invoke) {
// A finished install, repair or uninstall, and a game-telemetry save, change what
// Settings panels describe (LX-07): re-inspect so each gets a new snapshot.
let refreshLauncher = () => {};
const installation = mountInstall(document,invoke,undefined,()=>refreshLauncher());
const app = mountLauncher(document,invoke,()=>{void installation.refresh();},()=>installation.installFromArchive());
refreshLauncher = () => { void app.refresh(); };
let updater: ReturnType<typeof mountUpdater> | undefined;
// Its own record: it never reads or writes the launcher-summary choice.
const gameTelemetry = mountGameTelemetry(document,invoke,()=>refreshLauncher());
// The status line under the same checkbox (LT-11): on mount, every 15 s, and on change.
const telemetryStatus = mountTelemetryStatus(document,invoke);
// The game-telemetry save reads the checkbox after its first inspect: refresh the line once that lands.
void gameTelemetry.ready.then(()=>telemetryStatus.refresh());
// A finished Play changes what the last launch did about game diagnostics.
const play = mountLaunch(document,invoke,()=>{void installation.refresh(); void app.refresh(); void updater?.refresh(); void gameTelemetry.refresh(); void telemetryStatus.refresh();},undefined,true,ready=>installation.setPlayable(ready));
let adoption: ReturnType<typeof mountAdoption> | undefined;
const migration = mountMigration(document,invoke,()=>{void app.refresh(); void installation.refresh(); void play.refresh(); void adoption?.refresh();});
// Adoption notifies only when a saved revision changed, so this cannot loop.
adoption = mountAdoption(document,invoke,()=>{void app.refresh(); void installation.refresh(); void play.refresh();});
let adoptionRevision: string | undefined;
const stopAdoptionRefresh = app.context.onSnapshot(snapshot=>{
 const key=`${snapshot.operation.revision}:${snapshot.preferences.revision}`;
 if(key!==adoptionRevision){adoptionRevision=key;void adoption?.refresh();}
});
updater = mountUpdater(document,invoke,()=>{void app.refresh(); void installation.refresh(); void play.refresh();},{startup:true,onPlayGate:blocked=>play.setUpdateBlocked(blocked)});
const gameUpdate = mountGameUpdate(document,invoke,()=>{void app.refresh();void installation.refresh();void play.refresh();void updater?.refresh();});
return {installation, app, play, migration, adoption, updater, gameUpdate, gameTelemetry, telemetryStatus,
 dispose:async()=>{stopAdoptionRefresh();telemetryStatus.dispose();await Promise.all([gameTelemetry.dispose(),gameUpdate.dispose(),adoption!.dispose(),migration.dispose(),play.dispose(),app.dispose(),installation.dispose(),updater!.dispose()]);}};
}
