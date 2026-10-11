import {Effect} from 'effect';
import type {Invoke} from './view';
import type {PanelContext} from './panel-context';
import {decodePhysxStatus,physxFailure,type PhysxResult,type PhysxStatus} from './contract';
/**
 * Settings panel (LX-12, D-LX13): PhysX System Software on native Windows. Shows whether it is
 * installed and offers "Install PhysX", which runs the pinned vendor MSI elevated through UAC.
 * The panel also owns the non-blocking warning beside Play. Off Windows `supported` is false and
 * both stay hidden: the macOS path sets PhysX up inside Wine during installation.
 */
export const physxInstalled=(s:PhysxStatus)=>!!s.detection?.core_present;
const manual='install "NVIDIA PhysX Legacy System Software" yourself';
export function physxResultText(r:PhysxResult):string {
 switch(r.state){
  case 'installed':return r.restart_required?'PhysX System Software installed. Restart Windows before you play.':'PhysX System Software installed.';
  case 'cancelled_at_uac':return 'Install cancelled at the Windows permission prompt. Nothing was changed.';
  case 'cancelled_in_installer':return 'The PhysX installer was cancelled. Nothing was installed.';
  case 'failed':return `The PhysX installer failed (exit code ${r.exit_code}). Try again, or ${manual}.`;
  case 'not_detected':return `The PhysX installer finished (exit code ${r.exit_code}), but PhysX System Software is still not detected.`;
  case 'other_version_blocks':return `Another PhysX version is installed and blocks this installer, but it lacks the 2.6.3 core the game loads. To add that core, ${manual}.`;
  case 'start_failed':return `Windows could not start the PhysX installer (error ${r.win32_error}).`;
 }
}
const errorText:Record<string,string>={
 // Adopted copies, older installs and some seeds never had the retained installer.
 installer_unavailable:`The PhysX installer is not in this game folder. If the launcher installed this game, run Repair, then try again. Otherwise ${manual}.`,
 package_mismatch:'The PhysX installer in this game folder is not the expected file, so it was not run. Repair the game, then try again.',
 busy:'Another install, repair, update or game is running. Try again when it finishes.',
 persistence_uncertain:'The launcher could not confirm its saved state. Restart the launcher, then try again.',
 in_use:'Another launcher is using these settings. Close it, then try again.',
 io:'The launcher could not prepare the PhysX installer. Check permissions and free disk space.',
 unavailable:'PhysX setup is only needed on Windows.',
};
export const physxWarning='PhysX System Software is missing, so the game may not start. Install it under Settings.';
export function mountPhysx(document:Document,invoke:Invoke,context:PanelContext) {
 const panel=document.getElementById('physx-panel');const warning=document.getElementById('physx-play-warning');
 if(!panel||!warning)throw new Error('Missing launcher element physx-panel or physx-play-warning');
 const make=<K extends keyof HTMLElementTagNameMap>(tag:K,id:string)=>{const e=document.createElement(tag);e.id=id;return e;};
 const heading=make('h3','physx-title');heading.textContent='PhysX System Software';
 const state=make('p','physx-state');
 const install=make('button','install-physx');install.textContent='Install PhysX';
 const status=make('p','physx-status');status.setAttribute('role','status');status.setAttribute('aria-live','polite');
 panel.textContent='';panel.append(heading,state,install,status);panel.hidden=true;warning.hidden=true;
 let current:PhysxStatus|null=null;let installing=false;let disposed=false;let mutation:Promise<void>=Promise.resolve();
 const render=()=>{
  if(disposed)return;
  const s=current;const supported=!!s?.supported;panel.hidden=!supported;
  const missing=supported&&!physxInstalled(s!);
  warning.hidden=!missing;warning.textContent=missing?physxWarning:'';
  if(!s||!supported)return;
  const lacking=s.detection?.core_registered?'A PhysX version is installed, but not the 2.6.3 core the game needs to start its physics engine.'
   :'Not installed. The game needs it to start its physics engine.';
  state.textContent=!missing?'Installed.':s.installer_available?lacking:lacking+' '+errorText.installer_unavailable!;
  const busy=installing||s.installing;
  install.hidden=!missing&&!busy;install.disabled=busy||!s.installer_available;
  install.textContent=busy?'Installing…':'Install PhysX';install.setAttribute('aria-busy',String(busy));
 };
 // Never throws synchronously into the snapshot feed, whatever the bridge does.
 const call=(command:'inspect'|'install')=>Promise.resolve().then(()=>invoke('physx_command',{request:{command,schema_version:1}}));
 const decode=(value:unknown)=>Effect.runPromise(decodePhysxStatus(value)).catch(()=>{throw 'schema';});
 const inspect=()=>{
  if(disposed||installing)return mutation;
  mutation=call('inspect').then(decode).then(next=>{
   if(disposed||installing)return;current=next;
   // A press whose reply was lost still has its result line.
   if(next.last_result&&!status.textContent)status.textContent=physxResultText(next.last_result);
   render();
  },failure=>{if(!disposed&&current?.supported){status.textContent=failure==='schema'?'The launcher answered in an unexpected format. Restart it.':errorText[physxFailure(failure)]??'PhysX status could not be read. Try again.';render();}});
  return mutation;
 };
 const onInstall=()=>{
  if(disposed||installing||install.disabled)return;
  installing=true;status.textContent='Waiting for the Windows permission prompt, then the PhysX installer…';render();
  mutation=call('install').then(decode).then(next=>{
   if(disposed)return;current=next;
   status.textContent=next.last_result?physxResultText(next.last_result):'The PhysX installer finished.';
  },failure=>{
   if(disposed)return;
   const code=failure==='schema'?'schema':physxFailure(failure);
   status.textContent=code==='schema'?'The launcher answered in an unexpected format. Restart it, then check PhysX again.'
    :errorText[code]??'The launcher could not confirm the result. Check PhysX again before retrying.';
  }).finally(()=>{installing=false;render();}).then(()=>{if(!disposed)return inspect();});
 };
 install.addEventListener('click',onInstall);
 // Each native snapshot (the first, and after an install, repair or uninstall, which change whether the
 // retained installer is there) re-inspects, so the panel asks nothing before the launcher has opened.
 const unsubscribe=context.onSnapshot(()=>{void inspect();});
 const ready=context.snapshot()?inspect():Promise.resolve();
 return {ready,refresh:inspect,settled:()=>mutation,dispose:async()=>{
  disposed=true;unsubscribe();install.removeEventListener('click',onInstall);await mutation;panel.textContent='';warning.hidden=true;}};
}
