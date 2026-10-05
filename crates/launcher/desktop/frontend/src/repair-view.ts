import type { InstallViewState, makeInstallWorkflow } from './install-workflow';
import type { Effect } from 'effect';
type Service = Effect.Success<typeof makeInstallWorkflow>;
type Action = 'repair'|'recover_repair'|'abandon_repair'|'cleanup_repair';
/** Settings confirmation is transient; native state alone authorizes mutation. */
export function repairControls(document:Document, uuid:()=>string, run:(action:(service:Service)=>ReturnType<Service['repair']>)=>unknown) {
  const get=<T extends HTMLElement=HTMLElement>(id:string)=>document.getElementById(id) as T|null;
  let current:InstallViewState;
  let pending:{action:Action,id:string,installationId:string|null,directory:string}|null=null;
  const listeners:(()=>void)[]=[];
  const on=(id:string,action:()=>void)=>{const button=get<HTMLButtonElement>(id);if(!button)return;
    const listener=()=>{if(!button.disabled)action();};button.addEventListener('click',listener);listeners.push(()=>button.removeEventListener('click',listener));};
  const show=()=>{
    const confirmation=get('repair-confirmation');if(!confirmation)return;
    confirmation.hidden=!pending;
    get('repair-directory')!.textContent=pending?.directory??'';
    get('repair-consequences')!.textContent=pending?.action==='repair'?'Rebuild the saved signed release. Game modifications will be replaced. The old game remains until replacement; settings and diagnostics consent stay unchanged. Cancel is available only before replacement begins.':pending?.action==='recover_repair'?'Finish a checkpointed replacement only after native ownership and helper checks pass. This cannot be cancelled. Unknown helper outcomes remain blocked.':pending?.action==='abandon_repair'?'Stop only interrupted preparation before a commit checkpoint. Keep the old game and all staged files. This does not repair content or free staging space. Unknown helper outcomes may prevent abandonment.':'Permanently delete the current successful repair’s old backup, including modifications there. Keep the active game and retained stages. This cannot be undone.';
  };
  for(const [id,action] of [['repair','repair'],['recover-repair','recover_repair'],['abandon-repair','abandon_repair'],['cleanup-repair','cleanup_repair']] as const)on(id,()=>{
    const target=current.status?.repair?.target;const operation=current.status?.native.operation.operation;
    pending={action,id:action==='repair'?uuid():operation!.id,installationId:target?.installation_id??null,directory:target?.directory??current.status?.repair?.directory??'the saved installation'};show();get<HTMLButtonElement>('confirm-repair')?.focus();
  });
  on('dismiss-repair',()=>{const action=pending?.action;pending=null;show();get<HTMLButtonElement>(action==='repair'?'repair':action==='recover_repair'?'recover-repair':action==='abandon_repair'?'abandon-repair':'cleanup-repair')?.focus();});
  on('confirm-repair',()=>{const intent=pending;pending=null;show();if(!intent)return;
    run(service=>intent.action==='repair'?service.repair(intent.id,intent.installationId!):service.repairAction(intent.action,intent.id));});
  return {render(state:InstallViewState,ready:boolean){current=state;const repair=state.status?.repair;
    for(const [id,enabled] of [['repair',!!repair?.target],['recover-repair',repair?.recovery],['abandon-repair',repair?.recovery],['cleanup-repair',repair?.cleanup]] as const){
      const button=get<HTMLButtonElement>(id);if(button){button.disabled=!ready||!enabled;if(id!=='repair')button.hidden=!enabled;}}
    if(pending && (pending.action==='repair'?pending.installationId!==repair?.target?.installation_id:pending.id!==state.status?.native.operation.operation?.id))pending=null;
    const confirm=get<HTMLButtonElement>('confirm-repair');if(confirm)confirm.disabled=!ready;
    const dismiss=get<HTMLButtonElement>('dismiss-repair');if(dismiss)dismiss.disabled=state.busy;
    show();
  },dispose(){listeners.forEach(remove=>remove());}};
}
