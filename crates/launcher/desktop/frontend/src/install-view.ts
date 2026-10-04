import { Context, Effect, Fiber, Layer, ManagedRuntime, Result, Stream } from 'effect';
import { installBridgeLayer, InstallFailure, InstallViewState, makeInstallWorkflow, operationActive } from './install-workflow';
import type { Invoke } from './view';
class Installation extends Context.Service<Installation,Effect.Success<typeof makeInstallWorkflow>>()('launcher/Installation') {}
const errors:Record<InstallFailure['code'],string>={
  transport:'Could not confirm the operation. Recheck status before continuing.',
  schema:'The interface and installer versions do not match.',unsupported_schema:'This installer state requires a different launcher version.',
  platform_unavailable:'Mac installation is not available in this build yet.',io:'Cannot read or write installation state. Check disk space and permissions.',
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
  const progress=get<HTMLProgressElement>('install-progress');
  const abort=new AbortController();
  let current:InstallViewState={status:null,busy:false,needsInspection:true,error:null};
  let disposed=false; let pending=false; let watching=false;
  let mutation:Promise<void>=Promise.resolve(); let observation:Promise<void>=Promise.resolve();
  const listeners:(()=>void)[]=[];
  const render=()=>{
    if(disposed)return;
    const status=current.status;
    const operation=status?.native.operation.operation;
    const active=!!status&&operationActive(status);
    const recovery=operation?.state==='reconciliation_required';
    const ready=!!status&&!status.native.requires_reopen&&!current.needsInspection&&!current.busy&&!pending;
    primary.disabled=!ready||!status?.install_supported||!!operation||!status.native.preferences.install_directory;
    primary.textContent=operation?.state==='succeeded'?'Content prepared':active?'Installing…':'Install Stargate Worlds';
    cancel.hidden=!active;
    cancel.disabled=!ready||operation?.state==='cancel_requested';
    resume.hidden=!recovery; resume.disabled=!ready||!status?.install_supported;
    recheck.hidden=!current.error&&!recovery;
    recheck.disabled=pending||current.busy||status?.native.requires_reopen===true;
    progress.hidden=!active||!status?.progress;
    if(status?.progress){progress.max=Math.max(1,status.progress.total);progress.value=Math.min(status.progress.current,progress.max);
      if(!status.progress.total)progress.removeAttribute('value');}
    let text='Checking installation…';
    if(current.error)text=errors[current.error];
    else if(pending)text='Confirming installation request…';
    else if(status){
      if(operation?.state==='succeeded')text='Game content prepared. Runtime checks and Play are not connected in this build.';
      else if(recovery)text='An interrupted installation was found. Inspect files or explicitly resume the saved attempt.';
      else if(active)text=operation?.state==='cancel_requested'?'Cancellation requested. Waiting for the installer to stop safely.':
        status.progress?.phase==='download'?'Downloading verified game content…':status.progress?.phase==='extraction'?'Extracting game content…':'Preparing installation…';
      else if(status.outcome==='rosetta_required')text='Rosetta is required to prepare Windows game compatibility on this Mac. No game was launched.';
      else if(status.outcome==='runtime_unavailable')text='Windows compatibility could not be prepared. Check your connection and available disk space. Existing files have been preserved.';
      else if(operation)text='Installation stopped. Partial files are preserved. Retry and cleanup are not connected in this build.';
      else if(!status.install_supported)text='Mac compatibility setup is still in development. Settings and Patch Notes are available.';
      else text=status.native.preferences.install_directory?'Ready to install game content.':'Choose an empty game folder in Settings to begin.';
    }
    get('install-status').textContent=text;
    primary.setAttribute('aria-busy',String(pending||active));
  };
  const watcher=runtime.runFork(Effect.flatMap(Installation,service=>Stream.runForEach(service.changes,
    value=>Effect.sync(()=>{current=value;render();}))));
  const observe=()=>{
    const status=current.status; const id=status?.native.operation.operation?.id;
    if(disposed||watching||!status||!id||!operationActive(status)||status.native.requires_reopen)return;
    watching=true;
    observation=runtime.runPromise(Effect.flatMap(Installation,service=>Effect.result(service.observe(id))),{signal:abort.signal})
      .then(()=>{}).catch(()=>{}).finally(()=>{watching=false;});
  };
  const run=(action:(service:Effect.Success<typeof makeInstallWorkflow>)=>Effect.Effect<unknown,InstallFailure>)=>{
    if(disposed||pending)return mutation;
    pending=true;render();
    mutation=runtime.runPromise(Effect.flatMap(Installation,service=>Effect.result(action(service))),{signal:abort.signal})
      .then(result=>{if(!disposed&&Result.isFailure(result))current={...current,error:result.failure.code,needsInspection:true};})
      .catch(()=>{}).finally(()=>{pending=false;render();if(!current.error)observe();});
    return mutation;
  };
  const on=(element:HTMLButtonElement,action:()=>void)=>{
    const listener=()=>{if(!element.disabled)action();};
    element.addEventListener('click',listener);listeners.push(()=>element.removeEventListener('click',listener));
  };
  on(primary,()=>{const id=uuid();void run(service=>service.install(id));});
  on(cancel,()=>{const id=current.status?.native.operation.operation?.id;if(id)void run(service=>service.cancel(id));});
  on(resume,()=>{const id=current.status?.native.operation.operation?.id;if(id)void run(service=>service.resume(id));});
  on(recheck,()=>{const operation=current.status?.native.operation.operation;
    void run(service=>operation?.state==='reconciliation_required'?service.reconcile(operation.id):service.inspect);});
  const refresh=()=>run(service=>service.inspect);
  const ready=refresh();
  return {ready,refresh,settled:()=>mutation,dispose:async()=>{
    if(disposed)return;disposed=true;abort.abort();listeners.forEach(remove=>remove());
    await Effect.runPromise(Fiber.interrupt(watcher));await Promise.all([mutation,observation]);await runtime.dispose();
  }};
}
