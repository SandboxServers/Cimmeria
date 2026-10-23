import {Context,Data,Effect,Layer,PubSub,Ref,Schema,Semaphore,Stream} from 'effect';
import {NativeSnapshot} from './contract';
const Count=Schema.Int.check(Schema.isBetween({minimum:0,maximum:Number.MAX_SAFE_INTEGER}));
const FileClass=Schema.Literals(['stock','patched','prepared','changed','missing','extra']);
const Light=Schema.Struct({patches_total:Count,patches_current:Count,patches_pending:Count,download_bytes:Count,
 skipped:Schema.Array(Schema.Struct({path:Schema.String,patch:Schema.String,reason:Schema.String})),skipped_total:Count,
 index_state:Schema.Literals(['compared','index_not_published','index_unreachable','index_invalid']),
 index_counts:Schema.Record(Schema.String,Count),index_files:Schema.Array(Schema.Tuple([Schema.String,FileClass])),index_files_total:Count});
const codes=['unsupported_schema','platform_unavailable','import_required','unsupported_catalog','unsupported_configuration','review_unavailable','source_changed','in_use','consent_required','no_reusable_files','busy','stale_revision','identity_conflict','recovery_required','persistence_uncertain','invalid_directory','unsafe_file','corrupt_state','io','cancelled','launcher_too_old','invalid_artifact','unsupported_archive','network','release_unavailable','verification_failed','unsupported_layout','game_running','duplicate_release_file','leftover_launcher_files','folder_not_writable','not_a_game_folder','backup_conflict','unsupported_mode'] as const;
const Code=Schema.Literals(codes);
const Review=Schema.Struct({preview_handle:Schema.String,operation_revision:Count,preferences_revision:Count,source:Schema.String,destination:Schema.String,
 source_kind:Schema.Literals(['legacy','folder']),
 release:Schema.Struct({manifest_sha256:Schema.String,seed_sha256:Schema.String,patches:Schema.Array(Schema.String)}),
 counts:Schema.Struct({matched:Count,known_transform:Count,modified:Count,needs_patching:Count,missing:Count,extra:Count}),
 differences:Schema.Array(Schema.Struct({path:Schema.String,source_path:Schema.NullOr(Schema.String),classification:Schema.Literals(['matched','known_transform','modified','needs_patching','missing','extra'])})),differences_omitted:Count,
 requires_replacement:Schema.Boolean,backup_directory:Schema.optionalKey(Schema.String),
 download:Schema.optionalKey(Schema.Struct({seed:Schema.Boolean,patches:Schema.Array(Schema.String),bytes:Count,local_seed:Schema.optionalKey(Schema.Boolean)})),
 login_servers:Schema.optionalKey(Schema.Array(Schema.Struct({name:Schema.String,url:Schema.String}))),client_patches_enabled:Schema.optionalKey(Schema.Boolean),
 game_telemetry_opted_in:Schema.optionalKey(Schema.Boolean),game_telemetry_available:Schema.Boolean,requires_normalization:Schema.Boolean,requires_telemetry_acceptance:Schema.Boolean,user_data_remains_in_source:Schema.Boolean,
 in_place:Schema.Boolean,blocked_files:Schema.Array(Schema.String),
 index_fallback:Schema.optionalKey(Schema.Struct({reason:Schema.String,check:Schema.NullOr(Schema.String),http_status:Schema.NullOr(Count),seed_bytes:Count})),light:Schema.optionalKey(Light)});
const Unsafe=Schema.Struct({entries:Schema.Array(Schema.Struct({reason:Schema.Literals(['link','cloud_file','hard_link','special_file','name','case_collision']),path:Schema.NullOr(Schema.String)})),total:Count});
export const AdoptionStatus=Schema.Struct({schema_version:Schema.Literal(1),native:NativeSnapshot,
 backend:Schema.Literals(['available','in_place','unsupported_platform','helper_unavailable']),
 imported:Schema.NullOr(Schema.Struct({launcher_directory:Schema.String,game_directory:Schema.String,blocker:Schema.NullOr(Code)})),
 modes:Schema.Array(Schema.Literals(['light','in_place','copy'])),
 activity:Schema.Literals(['idle','preparing','review','copying']),review:Schema.NullOr(Review),
 progress:Schema.NullOr(Schema.Struct({phase:Schema.Literals(['download','extraction','copy']),current:Count,total:Count,item:Schema.optionalKey(Schema.Literals(['seed','patch','file_index','index_build','reference','scan']))})),cancellable:Schema.Boolean,
 reconciliation:Schema.NullOr(Schema.Union([Schema.Struct({kind:Schema.Literal('preparation'),preparation_id:Schema.String}),
  Schema.Struct({kind:Schema.Literal('copy'),operation_id:Schema.String,directory:Schema.String,can_recover:Schema.Boolean,can_abandon:Schema.Boolean})])),
 preparations:Schema.Array(Schema.String),has_valid_install:Schema.Boolean,owned:Schema.Boolean,completed:Schema.NullOr(Schema.Struct({directory:Schema.String,backup_directory:Schema.optionalKey(Schema.String),light:Schema.optionalKey(Schema.Boolean)})),last_error:Schema.NullOr(Code),last_rejection:Schema.NullOr(Schema.Struct({check:Schema.String,entry:Schema.NullOr(Schema.String),expected:Schema.NullOr(Schema.String),found:Schema.NullOr(Schema.String),http_status:Schema.NullOr(Count)})),unsafe_entries:Schema.NullOr(Unsafe)});
export type AdoptionStatus=typeof AdoptionStatus.Type;
export type Request={command:'inspect'|'dismiss'|'cancel';schema_version:1}|{command:'confirm';schema_version:1;work_id:string;preview_handle:string;operation_revision:number;preferences_revision:number;normalize_managed_files:boolean;accept_unavailable_game_telemetry:boolean;old_game_closed:boolean;replace_modified_files?:boolean;confirmed:true}|{command:'recover'|'abandon';schema_version:1;operation_id:string;operation_revision:number;confirmed:true}|{command:'abandon_preparation';schema_version:1;preparation_id:string;operation_revision:number;confirmed:true};
export type Maintenance={action:'recover'|'abandon'|'abandon_preparation';id:string};
export type Choice='normalize'|'telemetry'|'closed'|'replace';
export class AdoptionFailure extends Data.TaggedError('AdoptionFailure')<{readonly code:string}> {}
export class AdoptionBridge extends Context.Service<AdoptionBridge,{call:(request:Request|'choose')=>Effect.Effect<AdoptionStatus,AdoptionFailure>}>()('launcher/AdoptionBridge') {}
const known=new Set<string>(codes);
const failure=(e:unknown)=>new AdoptionFailure({code:typeof e==='string'&&known.has(e)?e:'transport'});
/** The native side may have acted; only a fresh inspection can say. Never retried. */
export const unknownOutcome=(code:string)=>['transport','schema','io','persistence_uncertain','corrupt_state'].includes(code);
export const adoptionBridgeLayer=(call:(request:Request|'choose')=>Promise<unknown>)=>Layer.succeed(AdoptionBridge,{
 call:request=>Effect.tryPromise({try:()=>call(request),catch:failure}).pipe(
  // A person may leave the native folder dialog open; every other reply is bounded.
  effect=>request==='choose'?effect:effect.pipe(Effect.timeout('35 seconds'),Effect.catchTag('TimeoutError',()=>Effect.fail(new AdoptionFailure({code:'transport'})))),
  Effect.flatMap(value=>Schema.decodeUnknownEffect(AdoptionStatus,{onExcessProperty:'error'})(value).pipe(Effect.mapError(()=>new AdoptionFailure({code:'schema'}))))
 )});
export type AdoptionState={status:AdoptionStatus|null;pending:boolean;uncertain:boolean;error:string|null;stale:boolean;maintenance:Maintenance|null}&Record<Choice,boolean>;
const initial:AdoptionState={status:null,pending:false,uncertain:true,error:null,stale:false,maintenance:null,normalize:false,telemetry:false,closed:false,replace:false};
/** Consent rule comes from the native review; this only checks the boxes it asks for.
 * A folder whose release files do not verify cannot be adopted in place at all. */
export const consented=(s:AdoptionState)=>{const r=s.status?.review;return !!r&&!r.blocked_files.length&&s.closed&&(!r.requires_normalization||s.normalize)&&(!r.requires_telemetry_acceptance||s.telemetry)&&(!r.requires_replacement||s.replace);};
const offered=(status:AdoptionStatus,m:Maintenance)=>{const r=status.reconciliation;
 if(m.action==='abandon_preparation')return (r?.kind==='preparation'&&r.preparation_id===m.id)||status.preparations.includes(m.id);
 return r?.kind==='copy'&&r.operation_id===m.id&&(m.action==='recover'?r.can_recover:r.can_abandon);};
export const makeAdoptionWorkflow=Effect.gen(function*(){
 const bridge=yield* AdoptionBridge,gate=yield* Semaphore.make(1);
 const state=yield* Ref.make<AdoptionState>(initial);
 const events=yield* Effect.acquireRelease(PubSub.sliding<AdoptionState>({capacity:1,replay:1}),PubSub.shutdown);
 const publish=(s:AdoptionState)=>Ref.set(state,s).pipe(Effect.andThen(PubSub.publish(events,s)),Effect.asVoid);
 const accept=(status:AdoptionStatus)=>Effect.gen(function*(){
  const before=yield* Ref.get(state);
  if(before.status&&status.native.operation.revision<before.status.native.operation.revision)return yield* Effect.fail(new AdoptionFailure({code:'stale_revision'}));
  // A different review starts with every confirmation unchecked.
  const same=!!status.review&&status.review.preview_handle===before.status?.review?.preview_handle;
  yield* publish({...before,status,uncertain:status.native.requires_reopen,error:null,stale:same&&before.stale,
   normalize:same&&before.normalize,telemetry:same&&before.telemetry,closed:same&&before.closed,replace:same&&before.replace,
   maintenance:before.maintenance&&offered(status,before.maintenance)?before.maintenance:null});
  return status;
 });
 const failed=(error:AdoptionFailure)=>Ref.get(state).pipe(Effect.flatMap(s=>publish({...s,uncertain:s.uncertain||unknownOutcome(error.code),error:error.code,stale:s.stale||error.code==='stale_revision'})));
 const settle=Ref.get(state).pipe(Effect.flatMap(s=>publish({...s,pending:false})));
 const inspectUnlocked=bridge.call({command:'inspect',schema_version:1}).pipe(Effect.flatMap(accept));
 const inspect=inspectUnlocked.pipe(Effect.tapError(error=>Ref.get(state).pipe(Effect.flatMap(s=>publish({...s,uncertain:true,error:error.code})))),Semaphore.withPermits(gate,1));
 /** One mutation at a time; a lost reply is reported, never redispatched. */
 const mutate=(request:(s:AdoptionState)=>Effect.Effect<Request|'choose',AdoptionFailure>)=>Effect.gen(function*(){
  const before=yield* Ref.get(state);
  if(before.pending)return yield* Effect.fail(new AdoptionFailure({code:'busy'}));
  yield* publish({...before,pending:true,error:null});
  return yield* request(before).pipe(Effect.flatMap(bridge.call),Effect.flatMap(accept),Semaphore.withPermits(gate,1),Effect.tapError(failed),Effect.ensuring(settle));
 });
 const simple=(command:'choose'|'dismiss'|'cancel')=>mutate(()=>Effect.succeed(command==='choose'?'choose' as const:{command,schema_version:1 as const}));
 const confirm=mutate(before=>Effect.gen(function*(){
  const review=before.status?.review;
  if(!review||before.uncertain||before.stale||!consented(before))return yield* Effect.fail(new AdoptionFailure({code:'consent_required'}));
  // Native state may have moved while the review was read. A stale review is
  // kept on screen and reported; the confirmation is not sent.
  const status=yield* inspectUnlocked;
  if(status.review?.preview_handle!==review.preview_handle)return yield* Effect.fail(new AdoptionFailure({code:'review_unavailable'}));
  if(status.native.operation.revision!==review.operation_revision||status.native.preferences.revision!==review.preferences_revision)return yield* Effect.fail(new AdoptionFailure({code:'stale_revision'}));
  return {command:'confirm',schema_version:1,work_id:crypto.randomUUID(),preview_handle:review.preview_handle,operation_revision:review.operation_revision,preferences_revision:review.preferences_revision,
   normalize_managed_files:before.normalize,accept_unavailable_game_telemetry:before.telemetry,old_game_closed:before.closed,replace_modified_files:before.replace,confirmed:true} as const;
 }));
 const propose=(maintenance:Maintenance)=>Ref.get(state).pipe(Effect.flatMap(s=>s.pending||s.uncertain||!s.status||!offered(s.status,maintenance)?Effect.fail(new AdoptionFailure({code:'busy'})):publish({...s,maintenance,error:null})));
 const maintain=mutate(before=>{
  const m=before.maintenance,revision=before.status?.native.operation.revision;
  if(!m||before.uncertain||revision===undefined)return Effect.fail(new AdoptionFailure({code:'busy'}));
  const base={schema_version:1 as const,operation_revision:revision,confirmed:true as const};
  return Effect.succeed(m.action==='abandon_preparation'?{...base,command:m.action,preparation_id:m.id}:{...base,command:m.action,operation_id:m.id});
 });
 yield* publish(yield* Ref.get(state));
 return {inspect,choose:simple('choose'),dismiss:simple('dismiss'),cancel:simple('cancel'),confirm,propose,maintain,
  withdraw:Ref.get(state).pipe(Effect.flatMap(s=>publish({...s,maintenance:null}))),
  set:(choice:Choice,value:boolean)=>Ref.get(state).pipe(Effect.flatMap(s=>publish({...s,[choice]:value}))),
  snapshot:Ref.get(state),changes:Stream.fromPubSub(events)};
});
