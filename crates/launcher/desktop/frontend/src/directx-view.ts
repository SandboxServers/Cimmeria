import {Effect,Schema} from 'effect';
import {DirectxStatus,type PhysxResult} from './contract';
import type {Invoke} from './view';
import type {PanelContext} from './panel-context';
import {reportUiEvent,uiCode} from './ui-event';

export function directxResultText(result:PhysxResult):string {
 switch(result.state){
  case 'installed':return result.restart_required?'DirectX components installed. Restart Windows before playing.':'Required DirectX components are installed.';
  case 'cancelled_at_uac':return 'DirectX setup cancelled at the Windows permission prompt.';
  case 'cancelled_in_installer':return 'DirectX setup was cancelled. The required components are still missing.';
  case 'failed':return `DirectX setup failed (exit code ${result.exit_code}). Check your internet connection and try again.`;
  case 'not_detected':return 'DirectX setup finished, but the required 32-bit components are still missing. Recheck after restarting Windows.';
  case 'start_failed':return `Windows could not start DirectX setup (error ${result.win32_error}).`;
  case 'other_version_blocks':return 'Another installer blocked DirectX setup. Finish it, then try again.';
 }
}
const failures:Record<string,string>={download_failed:'The Microsoft installer could not be downloaded. Check your internet connection, then try again.',
 package_mismatch:'The downloaded installer failed verification and was not run.',busy:'Another setup, game or update is running. Wait for it to finish.',
 io:'DirectX setup could not access its files. Check permissions and free disk space.',schema:'DirectX status was unreadable. Restart the launcher and recheck.'};

/** Shown in the installation/adoption flow; every successful installer is re-probed. */
export function mountDirectx(document:Document,invoke:Invoke,context:PanelContext) {
 const panel=document.getElementById('directx-panel')!;
 panel.innerHTML='<h3>DirectX game components</h3><p id="directx-state"></p><div class="settings-actions"><button id="install-directx">Install DirectX components</button><button id="check-directx">Recheck components</button></div><p id="directx-status" role="status" aria-live="polite"></p>';
 const get=(id:string)=>document.getElementById(id)!;
 const install=get('install-directx') as HTMLButtonElement;
 const check=get('check-directx') as HTMLButtonElement;
 let current:DirectxStatus|null=null;let busy=false;let disposed=false;let generation=0;let mutation=Promise.resolve();
 const render=()=>{
  if(disposed)return;
  panel.hidden=current?.supported===false;
  const missing=!!current?.missing.length;
  get('directx-state').textContent=!current?'Checking required 32-bit DirectX libraries…':missing
   ?`The game needs legacy DirectX components (${current.missing.join(', ')}). These are separate from the DirectX version included with Windows.`
   :'Required 32-bit DirectX components are installed.';
  install.hidden=!!current&&!missing&&!busy;install.disabled=busy||!!current?.installing||!current;
  check.disabled=busy||!!current?.installing;
  install.textContent=busy?'Setting up DirectX…':'Install DirectX components';install.setAttribute('aria-busy',String(busy));
 };
 const call=(command:'inspect'|'install')=>Promise.resolve().then(()=>invoke('directx_command',{request:{command,schema_version:1}}))
  .then(value=>Effect.runPromise(Schema.decodeUnknownEffect(DirectxStatus,{onExcessProperty:'error'})(value)).catch(()=>{throw 'schema';}));
 const failure=(error:unknown)=>{
  reportUiEvent('install','directx_status_failed',uiCode(error));
  if(!disposed)get('directx-status').textContent=failures[uiCode(error)]??'DirectX status could not be confirmed. Recheck before retrying setup.';
 };
 const refresh=()=>{
  if(disposed||busy)return mutation;
  const token=++generation;
  mutation=call('inspect').then(next=>{if(!disposed&&!busy&&token===generation){current=next;
   if(next.missing.length&&next.last_result?.state==='installed')get('directx-status').textContent='Required DirectX components are missing again. Run setup or recheck after restarting Windows.';
   render();}},error=>{if(token===generation){failure(error);render();}});return mutation;
 };
 const setup=()=>{
  if(disposed||busy||install.disabled)return;
  ++generation;busy=true;get('directx-status').textContent='Downloading the verified Microsoft installer, then waiting for its permission and setup prompts…';render();
  mutation=call('install').then(next=>{if(!disposed){current=next;get('directx-status').textContent=next.last_result?directxResultText(next.last_result):'Setup finished. Checking components…';}},failure)
   .finally(()=>{busy=false;render();}).then(()=>{if(!disposed)return refresh();});
 };
 const recheck=()=>{void refresh();};
 install.addEventListener('click',setup);check.addEventListener('click',recheck);
 const unsubscribe=context.onSnapshot(()=>{void refresh();});render();
 return {ready:context.snapshot()?refresh():Promise.resolve(),refresh,settled:()=>mutation,dispose:async()=>{
  disposed=true;unsubscribe();install.removeEventListener('click',setup);check.removeEventListener('click',recheck);await mutation;panel.textContent='';}};
}
