import type {Invoke} from './view';
import type {PanelContext} from './panel-context';
import {scaffoldFailure,type DisplayMode,type DisplayOptions} from './contract';
import {reportUiEvent,uiCode} from './ui-event';
import type {FailureCode} from './failure-codes';
import {failureText} from './failure-copy';
import failureCodes from './failure-codes.json';
/**
 * Settings panel (PD-26): how the game window starts, saved through `launcher_command`
 * (`save_display_options`, C-PD6) the way the login-server list saves (`save_client_options`): same
 * `expected_revision`, same reload check, same stale-revision recovery. The modes come from
 * `snapshot.display_modes`; a Mac offers `windowed` only, so it gets a line instead of the mode select.
 */
const modeLabel:Record<DisplayMode,string>={windowed:'Windowed',fullscreen:'Fullscreen (uses the resolution chosen in the game)'};
// PD-34 replaces this fixed list with the sizes that fit the chosen display.
const sizes:readonly (readonly [number,number])[]=[[1024,768],[1280,720],[1600,900],[1920,1080],[2560,1440]];
const sizeValue=(width:number,height:number)=>`${width}x${height}`;
const sizeLabel=(width:number,height:number)=>`${width} × ${height}`;
export const displayAppliesText='The launcher applies this every time you press Play, so a display change made inside the game is replaced at the next start.';
export const displayWindowsText='New: the game now starts in a 1024 × 768 window by default. Choose Fullscreen here to go back.';
export const displayMacText='On a Mac the game always runs in a window, so Cmd+Tab works.';
// The two refusals this panel has its own line for (PD-26 step 4); every other code is the mapped C-PD1 copy.
const saveErrorText:Record<string,string>={
 invalid_display_options:'That size can\'t be used. Pick one from the list.',
 stale_revision:'Settings changed while saving, so nothing was saved. Press Save again to save this setting.',
};
const jobErrors=failureCodes.job_errors as Record<string,FailureCode|undefined>;
export function mountDisplay(document:Document,invoke:Invoke,context:PanelContext) {
 const panel=document.getElementById('display-panel');
 if(!panel)throw new Error('Missing launcher element display-panel');
 const make=<K extends keyof HTMLElementTagNameMap>(tag:K,id:string)=>{const e=document.createElement(tag);e.id=id;return e;};
 const heading=make('h3','display-title');heading.textContent='Display';
 const modeLabelEl=make('label','display-mode-label');modeLabelEl.setAttribute('for','display-mode');modeLabelEl.textContent='Display mode';
 const mode=make('select','display-mode');
 const mac=make('p','display-mac');mac.textContent=displayMacText;mac.hidden=true;
 const sizeLabelEl=make('label','display-size-label');sizeLabelEl.setAttribute('for','display-size');sizeLabelEl.textContent='Window size';
 const size=make('select','display-size');
 const save=make('button','display-save');save.textContent='Save';save.disabled=true;
 const status=make('p','display-status');status.setAttribute('role','status');status.setAttribute('aria-live','polite');
 const applies=make('p','display-applies');applies.textContent=displayAppliesText;
 const windows=make('p','display-windows-note');windows.textContent=displayWindowsText;windows.hidden=true;
 panel.textContent='';panel.append(heading,modeLabelEl,mode,mac,sizeLabelEl,size,save,status,applies,windows);
 let dirty=false;let saving=false;let disposed=false;let mutation:Promise<void>=Promise.resolve();
 // `selected` on the option, not `select.value =`: it is what every DOM (and linkedom in the tests) supports.
 const option=(value:string,label:string,selected:boolean)=>{const o=document.createElement('option');o.value=value;o.textContent=label;o.selected=selected;return o;};
 const macOnly=(modes:readonly DisplayMode[])=>modes.length===1&&modes[0]==='windowed';
 const chosenMode=():DisplayMode=>(macOnly(context.snapshot()?.display_modes??[])?'windowed':mode.value as DisplayMode);
 const showSize=()=>{const shown=chosenMode()==='windowed';size.hidden=!shown;sizeLabelEl.hidden=!shown;};
 const fill=(modes:readonly DisplayMode[],saved:DisplayOptions)=>{
  const onlyWindowed=macOnly(modes);
  mode.hidden=onlyWindowed;modeLabelEl.hidden=onlyWindowed;mac.hidden=!onlyWindowed;
  // The Windows note: the only platform whose game used to start fullscreen offers both modes.
  windows.hidden=onlyWindowed||!modes.includes('fullscreen');
  mode.textContent='';for(const m of modes)mode.append(option(m,modeLabel[m],m===saved.mode));
  const list=sizes.some(([w,h])=>w===saved.width&&h===saved.height)?sizes:[...sizes,[saved.width,saved.height] as const];
  size.textContent='';for(const [w,h] of list)size.append(option(sizeValue(w,h),sizeLabel(w,h),w===saved.width&&h===saved.height));
  showSize();
 };
 // The saved setting fills the controls until the player changes one; their choice is never overwritten.
 const sync=()=>{
  const snapshot=context.snapshot();if(disposed)return;
  if(!snapshot){save.disabled=true;return;}
  if(!saving)save.disabled=false;
  if(dirty||saving)return;
  fill(snapshot.display_modes,snapshot.preferences.display);
 };
 const unsubscribe=context.onSnapshot(()=>sync());
 const onChange=()=>{dirty=true;showSize();};
 const onSave=()=>{
  if(disposed||saving)return;
  const snapshot=context.snapshot();
  if(!snapshot){status.textContent='Settings are still loading. Try again in a moment.';return;}
  const [width,height]=size.value.split('x').map(Number) as [number,number];
  const display:DisplayOptions={mode:chosenMode(),width,height};
  saving=true;save.disabled=true;mode.disabled=true;size.disabled=true;save.setAttribute('aria-busy','true');status.textContent='Saving…';
  const request={command:'save_display_options',schema_version:1,expected_revision:snapshot.preferences.revision,display};
  mutation=invoke('launcher_command',{request}).then(async()=>{
   // Until the reload lands the controls keep the player's choice: the old snapshot must not overwrite what was saved.
   const refreshed=await context.refresh().then(()=>true,()=>false);
   if(disposed)return;
   // A refresh that brought no snapshot newer than the one this save was based on did not reload it.
   const reloaded=refreshed&&(context.snapshot()?.preferences.revision??-1)>request.expected_revision;
   if(!reloaded){dirty=true;status.textContent='Saved, but the launcher could not reload settings. Restart the launcher to see the saved setting.';return;}
   dirty=false;status.textContent='Saved. Applies the next time you press Play.';
  },async failure=>{
   const code=scaffoldFailure(failure);
   if(code==='stale_revision')await context.refresh().catch(e=>reportUiEvent('settings','refresh_failed',uiCode(e)));
   if(!disposed)status.textContent=saveErrorText[code]??failureText(jobErrors[code]??'unknown').message;
  }).finally(()=>{saving=false;if(disposed)return;save.disabled=false;mode.disabled=false;size.disabled=false;save.setAttribute('aria-busy','false');sync();});
 };
 mode.addEventListener('change',onChange);size.addEventListener('change',onChange);save.addEventListener('click',onSave);
 sync();
 const ready=Promise.resolve();
 return {ready,refresh:()=>{sync();return mutation;},settled:()=>mutation,dispose:async()=>{
  disposed=true;unsubscribe();mode.removeEventListener('change',onChange);size.removeEventListener('change',onChange);
  save.removeEventListener('click',onSave);await mutation;panel.textContent='';}};
}
