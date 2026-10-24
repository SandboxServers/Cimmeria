import type {Invoke} from './view';
import type {PanelContext} from './panel-context';
import {scaffoldFailure,type DisplayInfo,type DisplayMode,type DisplayOptions,type FrameCap,type MonitorRect,type NativeSnapshot} from './contract';
import {reportUiEvent,uiCode} from './ui-event';
import type {FailureCode} from './failure-codes';
import {failureText} from './failure-copy';
import failureCodes from './failure-codes.json';
/**
 * Settings panel (PD-26): how the game window starts, saved through `launcher_command`
 * (`save_display_options`, C-PD6) the way the login-server list saves (`save_client_options`): same
 * `expected_revision`, same reload check, same stale-revision recovery. The modes come from
 * `snapshot.display_modes`; a Mac offers `windowed` only, so it gets a line instead of the mode select.
 * PD-34: the sizes are the ones that fit the chosen monitor (`display_sizes`), the monitor select shows only
 * with two or more displays, and the frame cap only where the platform offers one (`frame_caps`, macOS).
 */
const modeLabel:Record<DisplayMode,string>={windowed:'Windowed',fullscreen:'Fullscreen (uses the resolution chosen in the game)'};
const sizeValue=(width:number,height:number)=>`${width}x${height}`;
const sizeLabel=(width:number,height:number)=>`${width} × ${height}`;
export const displayNoFitText='(doesn\'t fit this display)';
export const displayMonitorNoteText='Remembered for this set of displays.';
export const displayWindowedOnlyText='Applies to windowed mode.';
// `Main display`, then `Display 2`, `Display 3` ... in list order, each with its size in the units the game sees.
const monitorLabels=(displays:readonly DisplayInfo[])=>{let n=1;return displays.map(d=>`${d.main?'Main display':`Display ${++n}`} (${sizeLabel(d.width,d.height)})`);};
const frameCapLabel=(cap:FrameCap,refreshHz:number)=>cap==='fps30'?'30 fps (cooler, quieter)':cap==='fps60'?'60 fps':`Match display (${refreshHz} Hz)`;
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
 const monitorLabelEl=make('label','display-monitor-label');monitorLabelEl.setAttribute('for','display-monitor');monitorLabelEl.textContent='Monitor';
 const monitor=make('select','display-monitor');
 const monitorNote=make('p','display-monitor-note');monitorNote.textContent=displayMonitorNoteText;
 const monitorWindowed=make('p','display-monitor-windowed');monitorWindowed.textContent=displayWindowedOnlyText;monitorWindowed.hidden=true;
 const capLabelEl=make('label','display-frame-cap-label');capLabelEl.setAttribute('for','display-frame-cap');capLabelEl.textContent='Frame rate limit';
 const cap=make('select','display-frame-cap');
 const save=make('button','display-save');save.textContent='Save';save.disabled=true;
 const status=make('p','display-status');status.setAttribute('role','status');status.setAttribute('aria-live','polite');
 const applies=make('p','display-applies');applies.textContent=displayAppliesText;
 const windows=make('p','display-windows-note');windows.textContent=displayWindowsText;windows.hidden=true;
 panel.textContent='';panel.append(heading,modeLabelEl,mode,mac,monitorLabelEl,monitor,monitorNote,monitorWindowed,sizeLabelEl,size,capLabelEl,cap,save,status,applies,windows);
 let dirty=false;let saving=false;let disposed=false;let mutation:Promise<void>=Promise.resolve();
 // `selected` on the option, not `select.value =`: it is what every DOM (and linkedom in the tests) supports.
 const option=(value:string,label:string,selected:boolean)=>{const o=document.createElement('option');o.value=value;o.textContent=label;o.selected=selected;return o;};
 const macOnly=(modes:readonly DisplayMode[])=>modes.length===1&&modes[0]==='windowed';
 const chosenMode=():DisplayMode=>(macOnly(context.snapshot()?.display_modes??[])?'windowed':mode.value as DisplayMode);
 const displaysOf=()=>context.snapshot()?.displays??[];
 // The position in `displays` of the chosen monitor: the select when it is shown, else the main display.
 const chosenMonitor=()=>{const displays=displaysOf();const main=Math.max(0,displays.findIndex(d=>d.main));
  if(monitor.hidden)return main;const i=Number(monitor.value);return Number.isInteger(i)&&i>=0&&i<displays.length?i:main;};
 // The size and the monitor apply to a window only. A Windows Fullscreen keeps the monitor visible but disabled,
 // with a note, and keeps the saved monitor (PD-33 ruling 1, A3).
 const showMode=()=>{const windowed=chosenMode()==='windowed';size.hidden=!windowed;sizeLabelEl.hidden=!windowed;
  monitor.disabled=saving||!windowed;monitorWindowed.hidden=monitor.hidden||windowed;};
 // The sizes that fit the chosen monitor, plus `current` when it is not one of them. Without display data
 // (an older engine) there is nothing to measure against, so the default and `current` are offered unlabelled.
 const fillSizes=(current:readonly [number,number])=>{
  const fits=context.snapshot()?.display_sizes?.[chosenMonitor()];
  const list:(readonly [number,number])[]=[...(fits??[[1024,768]])];
  const [cw,ch]=current;const listed=list.some(([w,h])=>w===cw&&h===ch);
  size.textContent='';for(const [w,h] of list)size.append(option(sizeValue(w,h),sizeLabel(w,h),w===cw&&h===ch));
  if(!listed)size.append(option(sizeValue(cw,ch),fits?`${sizeLabel(cw,ch)} ${displayNoFitText}`:sizeLabel(cw,ch),true));
 };
 const fillCaps=(caps:readonly FrameCap[],chosen:FrameCap)=>{
  const hz=displaysOf()[chosenMonitor()]?.refresh_hz??60;
  cap.textContent='';for(const c of caps)cap.append(option(c,frameCapLabel(c,hz),c===chosen));
 };
 const fill=(snapshot:NativeSnapshot)=>{
  const modes=snapshot.display_modes;const saved=snapshot.preferences.display;const displays=snapshot.displays??[];
  const onlyWindowed=macOnly(modes);
  mode.hidden=onlyWindowed;modeLabelEl.hidden=onlyWindowed;mac.hidden=!onlyWindowed;
  // The Windows note: the only platform whose game used to start fullscreen offers both modes.
  windows.hidden=onlyWindowed||!modes.includes('fullscreen');
  mode.textContent='';for(const m of modes)mode.append(option(m,modeLabel[m],m===saved.mode));
  const many=displays.length>1;monitor.hidden=!many;monitorLabelEl.hidden=!many;monitorNote.hidden=!many;
  const remembered=displays.findIndex(d=>d.index===snapshot.display_monitor);
  const selected=remembered>=0?remembered:displays.findIndex(d=>d.main);
  monitor.textContent='';monitorLabels(displays).forEach((label,i)=>monitor.append(option(String(i),label,i===selected)));
  fillSizes([saved.width,saved.height]);
  const caps=snapshot.frame_caps??[];cap.hidden=caps.length===0;capLabelEl.hidden=caps.length===0;
  fillCaps(caps,saved.frame_cap??'fps30');
  showMode();
 };
 // The saved setting fills the controls until the player changes one; their choice is never overwritten.
 const sync=()=>{
  const snapshot=context.snapshot();if(disposed)return;
  if(!snapshot){save.disabled=true;return;}
  if(!saving)save.disabled=false;
  if(dirty||saving)return;
  fill(snapshot);
 };
 const unsubscribe=context.onSnapshot(()=>sync());
 const onChange=()=>{dirty=true;showMode();};
 // A new monitor has its own sizes and refresh rate: keep the chosen size and cap, relabelled for it.
 const onMonitor=()=>{
  dirty=true;const [w,h]=size.value.split('x').map(Number) as [number,number];
  fillSizes([w,h]);fillCaps(context.snapshot()?.frame_caps??[],(cap.value||'fps30') as FrameCap);showMode();
 };
 const onSave=()=>{
  if(disposed||saving)return;
  const snapshot=context.snapshot();
  if(!snapshot){status.textContent='Settings are still loading. Try again in a moment.';return;}
  const [width,height]=size.value.split('x').map(Number) as [number,number];
  const offered=(snapshot.frame_caps??[]).length>0;
  const display:DisplayOptions=offered?{mode:chosenMode(),width,height,frame_cap:(cap.value||'fps30') as FrameCap}:{mode:chosenMode(),width,height};
  // The chosen monitor's rectangle, or null for the main display (and for a single display).
  const target=snapshot.displays?.[chosenMonitor()];
  const rect:MonitorRect|null=target&&!target.main?{x:target.x,y:target.y,width:target.width,height:target.height}:null;
  saving=true;save.disabled=true;mode.disabled=true;size.disabled=true;monitor.disabled=true;cap.disabled=true;
  save.setAttribute('aria-busy','true');status.textContent='Saving…';
  const request={command:'save_display_options',schema_version:1,expected_revision:snapshot.preferences.revision,display,monitor:rect};
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
  }).finally(()=>{saving=false;if(disposed)return;save.disabled=false;mode.disabled=false;size.disabled=false;cap.disabled=false;showMode();save.setAttribute('aria-busy','false');sync();});
 };
 mode.addEventListener('change',onChange);size.addEventListener('change',onChange);cap.addEventListener('change',onChange);monitor.addEventListener('change',onMonitor);
 save.addEventListener('click',onSave);
 sync();
 const ready=Promise.resolve();
 return {ready,refresh:()=>{sync();return mutation;},settled:()=>mutation,dispose:async()=>{
  disposed=true;unsubscribe();mode.removeEventListener('change',onChange);size.removeEventListener('change',onChange);
  cap.removeEventListener('change',onChange);monitor.removeEventListener('change',onMonitor);save.removeEventListener('click',onSave);await mutation;panel.textContent='';}};
}
