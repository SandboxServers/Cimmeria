import { Context, Effect, Fiber, Layer, ManagedRuntime, Result, Stream } from 'effect';
import { installBridgeLayer, InstallFailure, InstallStatus, InstallViewState, makeInstallWorkflow, operationActive } from './install-workflow';
import type { Invoke } from './view';
class Installation extends Context.Service<Installation,Effect.Success<typeof makeInstallWorkflow>>()('launcher/Installation') {}
const errors:Record<InstallFailure['code'],string>={
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
export function mountInstall(document:Document,invoke:Invoke,uuid:()=>string=()=>crypto.randomUUID()) {
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
  const abort=new AbortController();
  let current:InstallViewState={status:null,busy:false,needsInspection:true,error:null};
  let disposed=false; let pending=false; let watching=false;
  let journeyInstallId:string|null=null;
  let mutation:Promise<void>=Promise.resolve(); let observation:Promise<void>=Promise.resolve();
  const listeners:(()=>void)[]=[];
  const render=()=>{
    if(disposed)return;
    const status=current.status;
    const operation=status?.native.operation.operation;
    const active=!!status&&operationActive(status);
    const recovery=operation?.state==='reconciliation_required';
    const ready=!!status&&!status.native.requires_reopen&&!current.needsInspection&&!current.busy&&!pending;
    const canPrepare=!!status?.runtime_setup&&!active&&!recovery;
    primary.disabled=!ready||active||(!canPrepare&&(!status?.install_supported||(!!operation&&!status?.can_retry)||!status.native.preferences.install_directory));
    const removed=operation?.kind==='uninstall'&&operation.state==='succeeded';
    const runtimeSetup=operation?.kind==='prepare_runtime';
    primary.textContent=canPrepare?'Continue installation':runtimeSetup?(active?'Checking compatibility…':operation?.state==='succeeded'?'Compatibility checked':'Check compatibility status'):removed?'Install Stargate Worlds':status?.can_retry?'Retry installation':operation?.state==='succeeded'?'Content prepared':active?(operation?.kind==='uninstall'?'Removing…':'Installing…'):'Install Stargate Worlds';
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
    get('uninstall-directory').textContent=uninstalling?.directory??'';
    get<HTMLButtonElement>('confirm-uninstall').disabled=!ready||active;
    get<HTMLButtonElement>('dismiss-uninstall').disabled=pending||current.busy;
    cancel.hidden=!active||(operation?.kind!=='install'&&operation?.kind!=='prepare_runtime');
    cancel.disabled=!ready||operation?.state==='cancel_requested';
    resume.hidden=!recovery||!status?.can_resume; resume.disabled=!ready||!status?.can_resume;
    recheck.textContent=runtimeSetup&&recovery&&status?.can_reconcile?'Recover compatibility setup':'Recheck status';
    recheck.hidden=!current.error&&!recovery;
    recheck.disabled=pending||current.busy||status?.native.requires_reopen===true;
    progress.hidden=!active||!status?.progress;
    if(status?.progress){progress.max=Math.max(1,status.progress.total);progress.value=Math.min(status.progress.current,progress.max);
      if(!status.progress.total)progress.removeAttribute('value');}
    let text='Checking installation…';
    if(current.error)text=errors[current.error];
    else if(pending)text='Confirming operation…';
    else if(status){
      if(removed)text='Game uninstalled. You can install it again when ready.';
      else if(canPrepare)text='Game content is ready. Continue installation to check compatibility.';
      else if(runtimeSetup)text=recovery?(status.can_reconcile?'A saved compatibility result is available. Recover setup to finish checking its state.':'Compatibility setup was interrupted. Files are preserved; recovery requires inspection.'):active?(operation?.state==='cancel_requested'?'Cancellation requested. Waiting for compatibility setup to stop safely.':'Checking game compatibility…'):operation?.state==='succeeded'?'Prerequisites checked. Graphics and Play still need validation.':'Compatibility setup stopped. Game files are preserved.';
      else if(operation?.kind==='uninstall')text=recovery?'Uninstall was interrupted. Use Finish uninstall in Settings to confirm removal again.':active?'Removing game files…':'Inspect uninstall status before continuing.';
      else if(operation?.state==='succeeded')text='Game content prepared. This build cannot continue compatibility setup for this installation.';
      else if(recovery)text=status.can_resume||status.can_reconcile?'An interrupted installation was found. Inspect files or explicitly resume the saved attempt.':'An interrupted compatibility operation was found. Files are preserved; recovery is not available in this build. You can recheck status.';
      else if(active)text=operation?.state==='cancel_requested'?'Cancellation requested. Waiting for the installer to stop safely.':
        status.progress?.phase==='download'?'Downloading verified game content…':status.progress?.phase==='extraction'?'Extracting game content…':'Preparing installation…';
      else if(status.outcome==='rosetta_required')text='Rosetta is required to prepare Windows game compatibility on this Mac. No game was launched.';
      else if(status.outcome==='runtime_unavailable')text='Windows compatibility could not be prepared. Check your connection and available disk space. Existing files have been preserved.';
      else if(status.can_retry)text='Ready to retry installation.';
      else if(operation)text='Installation stopped. Partial files are preserved. Remove partial files to retry, or choose another empty folder in Settings.';
      else if(!status.install_supported)text='This build is missing a verified compatibility helper. Settings and Patch Notes are available.';
      else text=status.native.preferences.install_directory?'Ready to install game content.':'Choose an empty game folder in Settings to begin.';
    }
    get('install-status').textContent=text;
    primary.setAttribute('aria-busy',String(pending||active));
  };
  const watcher=runtime.runFork(Effect.flatMap(Installation,service=>Stream.runForEach(service.changes,
    value=>Effect.sync(()=>{current=value;render();}))));
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
  return {ready,refresh,settled:()=>mutation,dispose:async()=>{
    if(disposed)return;disposed=true;abort.abort();listeners.forEach(remove=>remove());
    await Effect.runPromise(Fiber.interrupt(watcher));await Promise.all([mutation,observation]);await runtime.dispose();
  }};
}
