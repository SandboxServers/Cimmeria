import {Effect,Schema} from 'effect';
import {FailureCode} from './failure-codes';
import {failureText} from './failure-copy';
import type {Check,CheckId,PreflightFailure,PreflightOperation,PreflightWorkflow,Report} from './preflight-workflow';
const verbs:Record<PreflightOperation,string>={play:'play',install:'install',repair:'repair'};
const labels:Record<PreflightOperation,string>={play:'Play',install:'Install',repair:'Repair'};
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
/** What a gated press led to: the operation ran, the panel asks for attention, or another check was running. */
export type GateOutcome='proceeded'|'attention'|'busy';
/** Runs preflight for `operation` before `proceed` (PD-12); `proceed` runs now, on Continue, or never. */
export type PreflightGate=(operation:PreflightOperation,proceed:()=>void)=>Promise<GateOutcome>;
/** No preflight: the operation runs at once (views mounted on their own, and their tests). */
export const noPreflight:PreflightGate=async(_operation,proceed)=>{proceed();return 'proceeded';};
/**
 * The preflight panel (PD-11, requirement 2): for each check that is not Ok, what is missing, what the
 * launcher will do about it, and the button that does it. `show` renders a report; it never starts the
 * guarded operation itself. `gate` (PD-12) inspects first and runs the operation only when nothing is
 * open, or when the player presses `#preflight-continue`.
 */
export function mountPreflight(document:Document,workflow:PreflightWorkflow,onContinue:(operation:PreflightOperation)=>void=()=>{}) {
 const panel=document.getElementById('preflight-panel');
 if(!panel)throw new Error('Missing launcher element preflight-panel');
 const make=<K extends keyof HTMLElementTagNameMap>(tag:K,id?:string)=>{const e=document.createElement(tag);if(id)e.id=id;return e;};
 const heading=make('h3','preflight-title');const summary=make('p','preflight-summary');const list=make('ul','preflight-checks');
 const recheck=make('button','preflight-recheck');const proceed=make('button','preflight-continue');proceed.textContent='Continue';
 const status=make('p','preflight-status');status.setAttribute('role','status');status.setAttribute('aria-live','polite');status.style.whiteSpace='pre-line';
 summary.style.whiteSpace='pre-line';
 const actions=make('div');actions.className='settings-actions';actions.append(recheck,proceed);
 panel.textContent='';panel.append(heading,summary,list,actions,status);panel.hidden=true;
 let current:Report|null=null;let busy:'recheck'|CheckId|null=null;let disposed=false;let mutation:Promise<void>=Promise.resolve();
 // A refused inspect (PD-12 Do 4), the gate's own inspect in flight, and the press waiting on Continue.
 let refusal:{operation:PreflightOperation;failure:PreflightFailure}|null=null;let checking:PreflightOperation|null=null;
 let waiting:{operation:PreflightOperation;proceed:()=>void}|null=null;let generation=0;
 const shown=()=>checking??refusal?.operation??current?.operation??null;
 const render=()=>{
  const operation=shown();
  if(disposed||!operation)return;
  const verb=verbs[operation];
  heading.textContent=`Before you ${verb}`;
  list.textContent='';
  if(checking||refusal){
   // Never a silent block: a refused check says why and still lets the player go ahead.
   summary.textContent=checking?`Checking what the launcher needs to ${verb}…`:failureLines(refusal!.failure);
   recheck.textContent=checking||busy==='recheck'?'Checking…':'Check again';recheck.disabled=checking!==null||busy!==null;recheck.setAttribute('aria-busy',String(checking!==null||busy==='recheck'));
   proceed.textContent='Continue anyway';proceed.hidden=checking!==null;proceed.disabled=busy!==null;
   return;
  }
  const report=current!;const open=report.checks.filter(check=>check.status!=='ok');
  summary.textContent=!open.length?`Everything needed to ${verb} is ready.`:report.ready?`You can ${verb}. Some things may cause problems:`:`Fix these before you ${verb}:`;
  for(const check of open){
   const row=make('li');row.dataset.check=check.id;
   const message=make('p');message.textContent=failureText(check.failure??'unknown',{needed:check.bytes_needed??undefined,free:check.bytes_free??undefined}).message;
   const next=nextStep(check,report.operation);
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
  proceed.textContent='Continue';proceed.hidden=!(report.ready&&open.length>0);proceed.disabled=busy!==null;
 };
 const show=(report:Report)=>{if(disposed)return;generation++;current=report;refusal=null;status.textContent='';panel.hidden=false;render();};
 const refuse=(operation:PreflightOperation,failure:PreflightFailure)=>{if(disposed)return;generation++;current=null;refusal={operation,failure};status.textContent='';panel.hidden=false;render();};
 const hide=()=>{panel.hidden=true;};
 // One call at a time; the pressed button shows it is working before the native reply.
 const call=(what:'recheck'|CheckId,effect:(operation:PreflightOperation)=>Effect.Effect<Report,PreflightFailure>)=>{
  const operation=refusal?.operation??current?.operation;
  if(disposed||busy!==null||checking||!operation)return mutation;
  const started=generation;busy=what;status.textContent='';render();
  mutation=Effect.runPromise(Effect.result(effect(operation))).then(result=>{
   if(disposed||generation!==started)return;
   if(result._tag==='Failure'){status.textContent=failureLines(result.failure);return;}
   current=result.success;refusal=null;
   // After an install the player starts the operation again; the panel never starts it (PD-12 Do 3).
   status.textContent=what!=='recheck'&&current.ready?`Ready. Press ${labels[operation]} again.`:'';
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
 // The panel lives on the home view; a Repair confirmed in Settings returns there the way `#home` does.
 const reveal=()=>{
  const get=(id:string)=>document.getElementById(id);
  const gear=get('gear'),play=get('play');if(!play?.hidden)return;
  if(gear)gear.hidden=true;play.hidden=false;
  get('settings')?.setAttribute('aria-expanded','false');get('home')?.setAttribute('aria-pressed','true');
 };
 const gate:PreflightGate=(operation,next)=>{
  if(disposed||checking||busy!==null)return Promise.resolve('busy');
  // The panel shows the check at once, so every gated press has visible feedback before the reply.
  generation++;checking=operation;current=null;refusal=null;waiting=null;status.textContent='';reveal();panel.hidden=false;render();
  const outcome=Effect.runPromise(Effect.result(workflow.inspectPreflight(operation))).then((result):GateOutcome=>{
   checking=null;
   if(disposed)return 'busy';
   if(result._tag==='Success'&&result.success.ready&&result.success.checks.every(check=>check.status==='ok')){hide();next();return 'proceeded';}
   waiting={operation,proceed:next};
   if(result._tag==='Success')show(result.success);else refuse(operation,result.failure);
   return 'attention';
  });
  mutation=outcome.then(()=>{});
  return outcome;
 };
 const onContinueClick=()=>{
  const operation=shown();
  if(disposed||busy!==null||checking||!operation||proceed.hidden)return;
  const next=waiting?.operation===operation?waiting.proceed:null;waiting=null;hide();onContinue(operation);next?.();
 };
 recheck.addEventListener('click',onRecheck);proceed.addEventListener('click',onContinueClick);
 return {show,hide,gate,settled:()=>mutation,dispose:async()=>{
  disposed=true;waiting=null;recheck.removeEventListener('click',onRecheck);proceed.removeEventListener('click',onContinueClick);await mutation;panel.textContent='';panel.hidden=true;}};
}
