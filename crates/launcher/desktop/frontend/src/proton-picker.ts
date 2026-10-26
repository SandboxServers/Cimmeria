import {Schema} from 'effect';
import type {Invoke} from './view';
import {reportUiEvent,uiCode} from './ui-event';
/**
 * The Linux Proton picker (PP-12), a section of the Settings display panel. It asks `proton_command` for the
 * Protons found (PP-10) and saves one choice: the frontend sends an op and an id, never a path (R9). With no
 * Proton found the section stays hidden, so Windows and macOS never show it.
 */
const ProtonChoice=Schema.Struct({id:Schema.String,name:Schema.String,kind:Schema.String});
const ProtonView=Schema.Struct({available:Schema.Array(ProtonChoice),selected:Schema.NullOr(Schema.String),preferred:Schema.NullOr(Schema.String)});
export type ProtonViewState=typeof ProtonView.Type;
export const protonAutomaticLabel='Automatic';
export const protonSavedText='Saved.';
// The refusals this picker has a line for; any other error reads as the generic line below.
const refusalText:Record<string,string>={
 unknown_proton:'That Proton isn\'t installed any more, so it wasn\'t chosen. Pick another, or choose Automatic.',
 busy:'The launcher is busy, so nothing was saved. Try again when it finishes.',
 preference_unsaved:'The choice couldn\'t be saved. Try again.',
};
const genericRefusalText='Proton couldn\'t be chosen. Try again.';
export function mountProton(document:Document,invoke:Invoke,host:HTMLElement) {
 const make=<K extends keyof HTMLElementTagNameMap>(tag:K,id:string)=>{const e=document.createElement(tag);e.id=id;return e;};
 const section=make('section','proton-picker');section.setAttribute('aria-labelledby','proton-title');section.hidden=true;
 const heading=make('h3','proton-title');heading.textContent='Proton';
 const label=make('label','proton-label');label.setAttribute('for','proton-select');label.textContent='Proton for Play';
 const select=make('select','proton-select');
 const status=make('p','proton-status');status.setAttribute('role','status');status.setAttribute('aria-live','polite');
 section.append(heading,label,select,status);host.append(section);
 let view:ProtonViewState={available:[],selected:null,preferred:null};let saving=false;let disposed=false;let mutation:Promise<void>=Promise.resolve();
 const option=(value:string,text:string,selected:boolean)=>{const o=document.createElement('option');o.value=value;o.textContent=text;o.selected=selected;return o;};
 // The saved choice is selected when it is still found; otherwise Automatic, which is what Play then uses.
 const fill=(next:ProtonViewState)=>{
  view=next;
  const chosen=next.preferred!==null&&next.available.some(p=>p.id===next.preferred)?next.preferred:'';
  select.textContent='';select.append(option('',protonAutomaticLabel,chosen===''));
  for(const p of next.available)select.append(option(p.id,p.name,p.id===chosen));
  section.hidden=next.available.length===0;
 };
 const send=(request:Record<string,unknown>)=>invoke('proton_command',{request}).then(value=>{
  if(!Schema.is(ProtonView)(value))throw 'schema';
  return value;
 });
 const refresh=async()=>{
  if(disposed||saving)return;
  try{fill(await send({op:'inspect'}));}
  catch(error){reportUiEvent('settings','proton_inspect_failed',uiCode(error));section.hidden=true;}
 };
 // The first change saves at once; a refusal puts the selection back and says why.
 const onChange=async()=>{
  if(disposed||saving)return;
  const id=select.value===''?null:select.value;
  saving=true;select.disabled=true;status.textContent='Saving…';
  try{fill(await send({op:'select',id}));status.textContent=protonSavedText;}
  catch(error){
   const code=typeof error==='string'?error:'transport';
   if(!disposed){fill(view);status.textContent=refusalText[code]??genericRefusalText;}
   if(code!=='busy'&&code!=='unknown_proton'&&code!=='preference_unsaved')reportUiEvent('settings','proton_save_failed',uiCode(error));
  }finally{saving=false;if(!disposed)select.disabled=false;}
 };
 const onSelect=()=>{mutation=onChange();};
 select.addEventListener('change',onSelect);
 const ready=refresh();mutation=ready;
 return {ready,refresh:()=>{mutation=refresh();return mutation;},settled:()=>mutation,dispose:async()=>{
  disposed=true;select.removeEventListener('change',onSelect);await mutation;section.remove();}};
}
