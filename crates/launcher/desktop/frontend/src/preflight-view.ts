import {Effect,Schema} from 'effect';
import {FailureCode} from './failure-codes';
import {failureText} from './failure-copy';
import type {Check,CheckId,PreflightFailure,PreflightOperation,PreflightWorkflow,Report} from './preflight-workflow';
/**
 * The preflight panel (PD-11, requirement 2): for each check that is not Ok, what is missing, what the
 * launcher will do about it, and the button that does it. `show` renders a report; it never starts the
 * guarded operation itself (PD-12 wires `#preflight-continue` through `onContinue`).
 */
const verbs:Record<PreflightOperation,string>={play:'play',install:'install',repair:'repair'};
const isFailureCode=Schema.is(FailureCode);
// Transport and schema failures have no C-PD1 code of their own; they read as `unknown`.
const failureLines=(failure:PreflightFailure)=>{const {message,fix}=failureText(isFailureCode(failure.code)?failure.code:'unknown');return `${message}\n${fix}`;};
type Next={text:string;button?:{id:string;label:string;install?:CheckId;chooseFolder?:true}};
/** What happens for one check that is not Ok: the next step, and the button that takes it (if any). */
export function nextStep(check:Check,operation:PreflightOperation):Next {
 const fix=failureText(check.failure??'unknown',{needed:check.bytes_needed??undefined,free:check.bytes_free??undefined}).fix;
 if(check.action==='auto_install'&&check.id==='rosetta')return {text:'The launcher will install Rosetta 2 from Apple. macOS will ask for your administrator password.',
  button:{id:'preflight-install-rosetta',label:'Install Rosetta 2',install:'rosetta'}};
 if(check.action==='download'&&check.id==='wine_runtime')return operation==='install'
  ?{text:'The launcher will download the compatibility runtime (64 MB) from the WoWSilicon release page during the install.'}
  :{text:'The launcher will download the compatibility runtime (64 MB) from the WoWSilicon release page.',
   button:{id:'preflight-install-wine_runtime',label:'Download now',install:'wine_runtime'}};
 if(check.action==='ask_user'&&check.id==='game_folder')return {text:fix,button:{id:'preflight-choose-folder',label:'Choose game folder',chooseFolder:true}};
 // AskUser + Rosetta (`Unknown`) points at the panel's own `#preflight-recheck`; Manual and the rest say the fix.
 return {text:fix};
}
export function mountPreflight(document:Document,workflow:PreflightWorkflow,onContinue:(operation:PreflightOperation)=>void=()=>{}) {
 const panel=document.getElementById('preflight-panel');
 if(!panel)throw new Error('Missing launcher element preflight-panel');
 const make=<K extends keyof HTMLElementTagNameMap>(tag:K,id?:string)=>{const e=document.createElement(tag);if(id)e.id=id;return e;};
 const heading=make('h3','preflight-title');const summary=make('p','preflight-summary');const list=make('ul','preflight-checks');
 const recheck=make('button','preflight-recheck');const proceed=make('button','preflight-continue');proceed.textContent='Continue';
 const status=make('p','preflight-status');status.setAttribute('role','status');status.setAttribute('aria-live','polite');status.style.whiteSpace='pre-line';
 const actions=make('div');actions.className='settings-actions';actions.append(recheck,proceed);
 panel.textContent='';panel.append(heading,summary,list,actions,status);panel.hidden=true;
 let current:Report|null=null;let busy:'recheck'|CheckId|null=null;let disposed=false;let mutation:Promise<void>=Promise.resolve();
 const render=()=>{
  if(disposed||!current)return;
  const verb=verbs[current.operation];const open=current.checks.filter(check=>check.status!=='ok');
  const warnings=open.some(check=>check.status==='warning');
  heading.textContent=`Before you ${verb}`;
  summary.textContent=!open.length?`Everything needed to ${verb} is ready.`:current.ready?`You can ${verb}. Some things may cause problems:`:`Fix these before you ${verb}:`;
  list.textContent='';
  for(const check of open){
   const row=make('li');row.dataset.check=check.id;
   const message=make('p');message.textContent=failureText(check.failure??'unknown',{needed:check.bytes_needed??undefined,free:check.bytes_free??undefined}).message;
   const next=nextStep(check,current.operation);
   const then=make('p');then.textContent=check.status==='warning'?`${next.text} You can continue.`:next.text;
   row.append(message,then);
   if(next.button){
    const spec=next.button;const button=make('button',spec.id);const installing=spec.install!==undefined&&busy===spec.install;
    button.textContent=installing?'Installing…':spec.label;button.disabled=busy!==null;button.setAttribute('aria-busy',String(installing));
    button.addEventListener('click',()=>spec.install?install(spec.install):chooseFolder());
    row.append(button);
   }
   list.append(row);
  }
  recheck.textContent=busy==='recheck'?'Checking…':'Check again';recheck.disabled=busy!==null;recheck.setAttribute('aria-busy',String(busy==='recheck'));
  proceed.hidden=!(current.ready&&warnings);proceed.disabled=busy!==null;
 };
 const show=(report:Report)=>{if(disposed)return;current=report;status.textContent='';panel.hidden=false;render();};
 const hide=()=>{panel.hidden=true;};
 // One call at a time; the pressed button shows it is working before the native reply.
 const call=(what:'recheck'|CheckId,effect:(operation:PreflightOperation)=>Effect.Effect<Report,PreflightFailure>)=>{
  if(disposed||busy!==null||!current)return mutation;
  const operation=current.operation;busy=what;status.textContent='';render();
  mutation=Effect.runPromise(Effect.result(effect(operation))).then(result=>{
   if(disposed)return;
   if(result._tag==='Success'){current=result.success;status.textContent='';}else status.textContent=failureLines(result.failure);
  }).finally(()=>{busy=null;render();});
  return mutation;
 };
 const install=(check:CheckId)=>{void call(check,operation=>workflow.installDependency(operation,check));};
 const onRecheck=()=>{void call('recheck',operation=>workflow.inspectPreflight(operation));};
 // Opens Settings the way `#settings` does, with the folder chooser focused.
 const chooseFolder=()=>{
  const get=(id:string)=>document.getElementById(id);
  const gear=get('gear'),play=get('play');if(gear)gear.hidden=false;if(play)play.hidden=true;
  get('settings')?.setAttribute('aria-expanded','true');get('home')?.setAttribute('aria-pressed','false');
  (get('choose') as HTMLButtonElement|null)?.focus();
 };
 const onContinueClick=()=>{if(disposed||busy!==null||!current||proceed.hidden)return;const operation=current.operation;hide();onContinue(operation);};
 recheck.addEventListener('click',onRecheck);proceed.addEventListener('click',onContinueClick);
 return {show,hide,settled:()=>mutation,dispose:async()=>{
  disposed=true;recheck.removeEventListener('click',onRecheck);proceed.removeEventListener('click',onContinueClick);await mutation;panel.textContent='';panel.hidden=true;}};
}
