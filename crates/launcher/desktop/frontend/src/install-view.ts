import { repairControls } from './repair-view';
import { displayPath } from './display-path';
import { Context, Effect, Fiber, Layer, ManagedRuntime, Result, Stream } from 'effect';
import { installBridgeLayer, InstallFailure, InstallStatus, InstallViewState, makeInstallWorkflow, operationActive } from './install-workflow';
import type { SeedArchiveState } from './seed-archive-workflow';
import { acquireSeedArchive, seedArchiveOfferText, seedArchiveShowsLine, seedArchiveText, SeedArchive } from './seed-archive-view';
import type { Invoke } from './view';
import { formatBytes, formatDuration, makeRateTracker, type TransferSample } from './transfer-rate';
import { operationOpen } from './contract';
class Installation extends Context.Service<Installation,Effect.Success<typeof makeInstallWorkflow>>()('launcher/Installation') {}
const errors:Record<InstallFailure['code'],string>={
  launcher_too_old:'Update the launcher before installing or playing this release. Your game files are preserved.',
  transport:'Could not confirm the operation. Recheck status before continuing.',
  schema:'The interface and installer versions do not match.',unsupported_schema:'This installer state requires a different launcher version.',
  platform_unavailable:'The compatibility helper is unavailable. Check that the launcher files are intact.',io:'Cannot read or write installation state. Check disk space and permissions.',
  corrupt_state:'Saved installation state cannot be read. Files have been preserved.',invalid_directory:'Choose an empty game folder in Settings.',
  stale_revision:'Installation state changed. Recheck status.',busy:'An operation already owns this installation.',
  unknown_operation:'This operation is no longer the current installation. Recheck status.',identity_conflict:'This request does not match the saved installation.',
  recovery_required:'The interrupted installation needs inspection before continuing.',persistence_uncertain:'Restart the launcher to inspect uncertain saved state.',
  manifest_unavailable:'The release could not be downloaded. Check your connection and try again.',invalid_manifest:'The release could not be verified.',
  signing_key_unavailable:'This build is missing its release verification key.',
};
/** Application lifetime owns observation. Navigating tabs never disposes it. */
/** `onFinished` runs each time an install, repair, uninstall or cleanup stops running. */
export function mountInstall(document:Document,invoke:Invoke,uuid:()=>string=()=>crypto.randomUUID(),onFinished:()=>void=()=>{},now:()=>number=()=>performance.now()) {
  const runtime=ManagedRuntime.make(Layer.effect(Installation,makeInstallWorkflow).pipe(
    Layer.provide(installBridgeLayer(request=>invoke('install_command',{request}))),
  ));
  const get=<T extends HTMLElement=HTMLElement>(id:string)=>document.getElementById(id)! as T;
  const primary=get<HTMLButtonElement>('install');
  const cancel=get<HTMLButtonElement>('cancel-install');
  const resume=get<HTMLButtonElement>('resume-install');
  const recheck=get<HTMLButtonElement>('inspect-install');
  const cleanup=get<HTMLButtonElement>('clean-failed-install');
  const confirmation=get('cleanup-confirmation');
  let confirming:string|null=null;
  const uninstall=get<HTMLButtonElement>('uninstall');
  const uninstallConfirmation=get('uninstall-confirmation');
  let uninstalling:{installationId:string;operationId:string;directory:string}|null=null;
  const progress=get<HTMLProgressElement>('install-progress');
  // Speed and ETA come from readings of the active phase only; reset on phase change.
  const rates=makeRateTracker(now);let progressPhase:string|null=null;
  const abort=new AbortController();
  let current:InstallViewState={status:null,busy:false,needsInspection:true,error:null};
  let disposed=false; let pending=false; let watching=false;
  let journeyInstallId:string|null=null;
  let playable=false;
  let mutation:Promise<void>=Promise.resolve(); let observation:Promise<void>=Promise.resolve();
  const listeners:(()=>void)[]=[];
  const render=()=>{
    if(disposed)return;
    const status=current.status;
    const operation=status?.native.operation.operation;
    const active=!!status&&operationActive(status);
    const recovery=operation?.state==='reconciliation_required';
    // The adoption panel owns an adopt operation until it is terminal, so Install stays out of it.
    const adopting=operation?.kind==='adopt'&&operationOpen(operation);
    const ready=!!status&&!status.native.requires_reopen&&!current.needsInspection&&!current.busy&&!pending;
    repairs.render(current,ready);
    const canPrepare=!!status?.runtime_setup&&!active&&!recovery;
    primary.hidden=playable||operation?.kind==='launch'||adopting||(!!status?.uninstall&&!canPrepare&&!active);
    primary.disabled=!ready||active||(!canPrepare&&(!status?.install_supported||(!!operation&&!status?.can_retry)||!status.native.preferences.install_directory));
    const removed=operation?.kind==='uninstall'&&operation.state==='succeeded';
    const runtimeSetup=operation?.kind==='prepare_runtime';
    primary.textContent=operation?.kind==='repair'?(active?'Repairing…':operation.state==='succeeded'?'Game reconstructed':'Repair stopped'):canPrepare?'Continue installation':runtimeSetup?(active?'Checking compatibility…':operation?.state==='succeeded'?'Compatibility checked':'Check compatibility status'):removed?'Install Cimmeria':status?.can_retry?'Retry installation':operation?.state==='succeeded'?'Content prepared':active?(operation?.kind==='uninstall'?'Removing…':'Installing…'):'Install Cimmeria';
    const failed=operation?.kind==='install'&&(operation.state==='failed'||operation.state==='cancelled');
    cleanup.hidden=!failed||status?.can_retry===true;
    cleanup.disabled=!ready;
    if(!failed||confirming!==operation?.id)confirming=null;
    confirmation.hidden=confirming===null;
    get<HTMLButtonElement>('confirm-cleanup').disabled=!ready;
    get<HTMLButtonElement>('dismiss-cleanup').disabled=pending||current.busy;
    uninstall.disabled=!ready||active||!status?.uninstall;
    uninstall.textContent=status?.uninstall?.recovery?'Finish uninstall…':'Uninstall…';
    if(uninstalling?.installationId!==status?.uninstall?.installation_id)uninstalling=null;
    uninstallConfirmation.hidden=uninstalling===null;
    get('uninstall-directory').textContent=displayPath(uninstalling?.directory??'');
    get<HTMLButtonElement>('confirm-uninstall').disabled=!ready||active;
    get<HTMLButtonElement>('dismiss-uninstall').disabled=pending||current.busy;
    cancel.hidden=!active||(operation?.kind!=='install'&&operation?.kind!=='prepare_runtime'&&operation?.kind!=='repair');
    cancel.disabled=!ready||operation?.state==='cancel_requested';
    resume.hidden=!recovery||!status?.can_resume; resume.disabled=!ready||!status?.can_resume;
    recheck.textContent=runtimeSetup&&recovery&&status?.can_reconcile?'Recover compatibility setup':'Recheck status';
    recheck.hidden=(!current.error&&!recovery)||adopting;
    recheck.disabled=pending||current.busy||status?.native.requires_reopen===true;
    progress.hidden=!active||!status?.progress;
    if(status?.progress){progress.max=Math.max(1,status.progress.total);progress.value=Math.min(status.progress.current,progress.max);
      if(!status.progress.total)progress.removeAttribute('value');}
    const phase=status?.progress?.phase??null;
    if(!active||!status?.progress||phase!==progressPhase){rates.reset();progressPhase=phase;}
    const transfer=active&&status?.progress?rates.sample(status.progress.current,status.progress.total):null;
    let text='Checking installation…';
    if(current.error)text=operation?.kind==='repair'&&recovery&&['corrupt_state','recovery_required','io'].includes(current.error)?'Repair recovery remains blocked. Preserve the game, backup and stage. Recheck status; checkpoint recovery requires verified replacement evidence, and abandonment requires no commit checkpoint. An unknown helper outcome needs manual inspection.':errors[current.error];
    else if(pending)text='Checking installation status…';
    else if(status){
      if(operation?.kind==='launch')text='';
      else if(operation?.kind==='repair')text=recovery?'Repair needs recovery. In Settings, finish checkpointed replacement or abandon preparation. Unknown helper outcomes stay blocked; preserve files and restart to recheck.':active?(operation.state==='cancel_requested'?'Cancellation requested. Waiting for repair to stop safely.':status.progress?.phase==='download'?progressLine('Downloading repair content…',status.progress,transfer):status.progress?.phase==='extraction'?progressLine('Reconstructing game content…',status.progress,transfer):'Preparing or committing repair…'):operation.state==='succeeded'?repairSuccess(status.repair?.backup):operation.state==='cancelled'?'Repair cancelled. The old game and retained staging files are preserved.':'Repair stopped. Recheck status; retained files are preserved.';
      else if(removed)text='Game uninstalled. You can install it again when ready.';
      else if(canPrepare)text='Game content is ready. Continue installation to check compatibility.';
      else if(runtimeSetup)text=recovery?(status.can_reconcile?'A saved compatibility result is available. Recover setup to finish checking its state.':'Compatibility setup was interrupted. Files are preserved; recovery requires inspection.'):active?(operation?.state==='cancel_requested'?'Cancellation requested. Waiting for compatibility setup to stop safely.':'Checking game compatibility…'):operation?.state==='succeeded'?'Prerequisites checked. Graphics and Play still need validation.':'Compatibility setup stopped. Game files are preserved.';
      else if(operation?.kind==='uninstall')text=recovery?'Uninstall was interrupted. Use Finish uninstall in Settings to confirm removal again.':active?'Removing game files…':'Inspect uninstall status before continuing.';
      else if(operation?.kind==='adopt'&&operation.state==='succeeded')text='';
      else if(operation?.state==='succeeded')text='Game content prepared. Recheck Play status to check readiness.';
      else if(adopting&&recovery)text='';
      else if(recovery)text=status.can_resume||status.can_reconcile?'An interrupted installation was found. Inspect files or explicitly resume the saved attempt.':'An interrupted compatibility operation was found. Files are preserved; recovery is not available in this build. You can recheck status.';
      else if(active)text=operation?.state==='cancel_requested'?'Cancellation requested. Waiting for the installer to stop safely.':
        status.progress?.phase==='download'?progressLine('Downloading verified game content…',status.progress,transfer):status.progress?.phase==='extraction'?progressLine('Extracting game content…',status.progress,transfer):'Preparing installation…';
      else if(status.outcome==='rosetta_required')text='Rosetta is required to prepare Windows game compatibility on this Mac. No game was launched.';
      else if(status.outcome==='runtime_unavailable')text='Windows compatibility could not be prepared. Check your connection and available disk space. Existing files have been preserved.';
      else if(status.can_retry)text='Ready to retry installation.';
      else if(operation)text='Installation stopped. Partial files are preserved. Remove partial files to retry, or choose another empty folder in Settings.';
      else if(!status.install_supported)text='This build is missing a verified compatibility helper. Settings are available.';
      else text=status.native.preferences.install_directory?'Ready to install game content.':'Choose an empty game folder in Settings to begin.';
    }
    const idleSetup=!operation||['install','adopt','prepare_runtime','launch'].includes(operation.kind);
    get('install-status').textContent=playable&&idleSetup&&!current.error&&!active&&!recovery?'':text;
    primary.setAttribute('aria-busy',String(pending||active));
  };
  const repairs=repairControls(document,uuid,action=>run(action));
  // The operation that last stopped running; undefined until the first status,
  // so one already finished at startup is not reported as just finished.
  let stopped:string|null|undefined;
  const noticeStopped=(status:InstallStatus|null)=>{
    if(!status)return;const op=status.native.operation.operation;
    const key=op&&!operationActive(status)?`${op.id}:${op.state}`:null;
    if(stopped!==undefined&&key!==null&&key!==stopped&&!disposed)onFinished();
    stopped=key;
  };
  const watcher=runtime.runFork(Effect.flatMap(Installation,service=>Stream.runForEach(service.changes,
    value=>Effect.sync(()=>{current=value;noticeStopped(value.status);render();}))));
  const advance=(status:InstallStatus)=>{
    const op=status.native.operation.operation;
    if(disposed||pending||!journeyInstallId||op?.id!==journeyInstallId||op.kind!=='install'||op.state!=='succeeded'||!status.runtime_setup)return false;
    const previous=journeyInstallId;journeyInstallId=null;
    void run(service=>service.prepareRuntime(uuid(),status.runtime_setup!,previous));
    return true;
  };
  const observe=()=>{
    const status=current.status; const id=status?.native.operation.operation?.id;
    if(disposed||watching||!status||!id||!operationActive(status)||status.native.requires_reopen)return;
    watching=true;
    let completed:InstallStatus|null=null;
    observation=runtime.runPromise(Effect.flatMap(Installation,service=>Effect.result(service.observe(id))),{signal:abort.signal})
      .then(result=>{if(Result.isSuccess(result))completed=result.success;else journeyInstallId=null;})
      .catch(()=>{journeyInstallId=null;}).finally(()=>{watching=false;if(completed)advance(completed);});
  };
  const run=(action:(service:Effect.Success<typeof makeInstallWorkflow>)=>Effect.Effect<InstallStatus,InstallFailure>)=>{
    if(disposed||pending)return mutation;
    pending=true;render();
    let completed:InstallStatus|null=null;
    mutation=runtime.runPromise(Effect.flatMap(Installation,service=>Effect.result(action(service))),{signal:abort.signal})
      .then(result=>{if(Result.isSuccess(result))completed=result.success;else {journeyInstallId=null;if(!disposed)current={...current,error:result.failure.code,needsInspection:true};}})
      .catch(()=>{journeyInstallId=null;}).finally(()=>{pending=false;render();if(completed&&advance(completed))return;if(!current.error)observe();});
    return mutation;
  };
  // The install-area archive button runs the same native check as Settings (AI-07), on the runtime Settings shares.
  const archiveHold=acquireSeedArchive(invoke);const archiveRuntime=archiveHold.runtime;
  const archiveButton=get<HTMLButtonElement>('install-seed-archive');
  const archiveLine=get<HTMLElement>('install-seed-archive-status');
  const archiveOffer=get<HTMLElement>('install-seed-archive-offer');
  const archiveAccept=get<HTMLButtonElement>('install-seed-archive-accept');
  let archive:SeedArchiveState={status:null,phase:null,cancelling:false,error:null};let archivePressed=false;let acceptPressed=false;
  const renderArchive=()=>{const busy=archivePressed||acceptPressed||archive.phase!==null;
    archiveButton.hidden=playable&&!busy;
    archiveButton.textContent=archivePressed?'Switching to your archive…':'Use a downloaded game archive…';
    archiveButton.disabled=busy;archiveButton.setAttribute('aria-busy',String(busy));
    const question=seedArchiveOfferText(archive);
    archiveOffer.hidden=question===null;archiveOffer.textContent=question??'';
    archiveAccept.hidden=question===null;archiveAccept.disabled=busy;archiveAccept.textContent=acceptPressed?'Checking…':'Yes';
    archiveAccept.setAttribute('aria-busy',String(busy));
    archiveLine.hidden=!seedArchiveShowsLine(archive);archiveLine.textContent=seedArchiveText(archive);};
  const archiveWatcher=archiveRuntime.runFork(Effect.flatMap(SeedArchive,s=>Stream.runForEach(s.changes,
    value=>Effect.sync(()=>{archive=value;renderArchive();}))));
  const chooseArchive=()=>{if(archiveButton.disabled)return;archivePressed=true;renderArchive();
    void archiveRuntime.runPromise(Effect.flatMap(SeedArchive,s=>Effect.result(s.choose))).finally(()=>{archivePressed=false;renderArchive();});};
  archiveButton.addEventListener('click',chooseArchive);listeners.push(()=>archiveButton.removeEventListener('click',chooseArchive));
  const acceptArchive=()=>{if(archiveAccept.disabled)return;acceptPressed=true;renderArchive();
    void archiveRuntime.runPromise(Effect.flatMap(SeedArchive,s=>Effect.result(s.accept))).finally(()=>{acceptPressed=false;renderArchive();});};
  archiveAccept.addEventListener('click',acceptArchive);listeners.push(()=>archiveAccept.removeEventListener('click',acceptArchive));
  renderArchive();
  const on=(element:HTMLButtonElement,action:()=>void)=>{
    const listener=()=>{if(!element.disabled)action();};
    element.addEventListener('click',listener);listeners.push(()=>element.removeEventListener('click',listener));
  };
  on(uninstall,()=>{const target=current.status?.uninstall;if(!target)return;
    uninstalling={installationId:target.installation_id,directory:target.directory,
      operationId:target.recovery?current.status!.native.operation.operation!.id:uuid()};render();});
  on(get<HTMLButtonElement>('dismiss-uninstall'),()=>{uninstalling=null;render();});
  on(get<HTMLButtonElement>('confirm-uninstall'),()=>{const target=uninstalling;uninstalling=null;
    if(target)void run(service=>service.uninstall(target.operationId,target.installationId));});
  on(cleanup,()=>{confirming=current.status?.native.operation.operation?.id??null;render();});
  on(get<HTMLButtonElement>('dismiss-cleanup'),()=>{confirming=null;render();});
  on(get<HTMLButtonElement>('confirm-cleanup'),()=>{const id=confirming;confirming=null;if(id)void run(service=>service.cleanFailed(id));});
  on(primary,()=>{const id=uuid();const status=current.status;
    if(status?.runtime_setup&&status.native.operation.operation){journeyInstallId=null;void run(service=>service.prepareRuntime(id,status.runtime_setup!,status.native.operation.operation!.id));}
    else {journeyInstallId=id;void run(service=>service.install(id));}});
  on(cancel,()=>{journeyInstallId=null;const id=current.status?.native.operation.operation?.id;if(id)void run(service=>service.cancel(id));});
  on(resume,()=>{const id=current.status?.native.operation.operation?.id;if(id)void run(service=>service.resume(id));});
  on(recheck,()=>{const operation=current.status?.native.operation.operation;
    void run(service=>operation?.state==='reconciliation_required'&&current.status?.can_reconcile?service.reconcile(operation.id):service.inspect);});
  const refresh=()=>run(service=>service.inspect);
  const ready=refresh();
  return {ready,refresh,setPlayable:(ready:boolean)=>{playable=ready;render();renderArchive();},settled:()=>mutation,dispose:async()=>{
    if(disposed)return;disposed=true;abort.abort();listeners.forEach(remove=>remove());repairs.dispose();
    await Effect.runPromise(Fiber.interrupt(watcher));await Promise.all([mutation,observation]);await runtime.dispose();
    await Effect.runPromise(Fiber.interrupt(archiveWatcher));await archiveHold.release();
  }};
}

/** Downloads smaller than this (100 MiB) show percent and size only, without speed or ETA. */
const SPEED_MIN_TOTAL_BYTES = 100 * 1024 * 1024;

/**
 * Appends percent, size, speed and ETA to an active progress label. Download shows
 * bytes; extraction counts are unitless, so it shows only the percent.
 */
function progressLine(label: string, progress: NonNullable<InstallStatus['progress']>, transfer: TransferSample | null): string {
  if (!transfer || progress.total <= 0) return label;
  if (progress.phase === 'extraction') return `${label} ${transfer.percent}%`;
  const parts = [`${label} ${transfer.percent}%`, `${formatBytes(progress.current)} of ${formatBytes(progress.total)}`];
  // Small blobs (patches) finish in seconds, so a speed and a jumping ETA would be noise.
  if (progress.total >= SPEED_MIN_TOTAL_BYTES) {
    if (transfer.rateBytesPerSec !== null) parts.push(`${formatBytes(transfer.rateBytesPerSec)}/s`);
    if (transfer.etaSeconds !== null) parts.push(`about ${formatDuration(transfer.etaSeconds)} left`);
  }
  return parts.join(' · ');
}

function repairSuccess(backup: NonNullable<InstallStatus['repair']>['backup']): string {
  const evidence = backup === 'retained' ? 'The old backup is retained; remove it explicitly in Settings.'
    : backup === 'cleanup_pending' ? 'Backup cleanup is incomplete; finish it explicitly in Settings.'
    : backup === 'removed' ? 'The old backup has been removed.'
    : backup === 'not_retained' ? 'No old backup was retained.'
    : 'Recheck status for current backup details.';
  return `Game reconstructed. ${evidence} Play readiness is unchanged.`;
}
