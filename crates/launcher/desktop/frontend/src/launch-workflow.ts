import { Context, Data, Effect, Layer, PubSub, Ref, Schema, Semaphore, Stream } from 'effect';
import { NativeSnapshot, operationOpen } from './contract';
import { reportUiEvent, uiCode } from './ui-event';
import { FailureCode } from './failure-codes';
const Pid=Schema.Int.check(Schema.isBetween({minimum:1,maximum:4294967295}));
const Count=Schema.Int.check(Schema.isBetween({minimum:0,maximum:4294967295}));
export const LaunchStatus=Schema.Struct({schema_version:Schema.Literal(1),native:NativeSnapshot,
  installation_id:Schema.NullOr(Schema.String),setup_complete:Schema.Boolean,resources_available:Schema.Boolean,launcher_update_required:Schema.Boolean,patches_incomplete:Count,ops_skipped:Count,
  prerequisites_missing:Schema.optionalKey(Schema.Array(Schema.Literals(['directx','physx']))),
  prerequisites_installing:Schema.optionalKey(Schema.Boolean),
  observation:Schema.NullOr(Schema.Union([
    Schema.Struct({phase:Schema.Literals(['preparing','not_started','cancelled','unknown'])}),
    Schema.Struct({phase:Schema.Literal('host_started'),host_pid:Pid}),
    Schema.Struct({phase:Schema.Literal('process_started'),host_pid:Pid,guest_pid:Pid}),
    Schema.Struct({phase:Schema.Literal('process_exited'),host_pid:Pid,guest_pid:Pid,code:Schema.Int,early:Schema.Boolean}),
  ])),
  // Why the current launch did not start (C-PD3); null when none is known.
  failure:Schema.NullOr(FailureCode)});
export type LaunchStatus=typeof LaunchStatus.Type;
export type Request={command:'inspect';schema_version:1}|{command:'play';schema_version:1;operation_id:string;operation_revision:number;installation_id:string};
export class LaunchFailure extends Data.TaggedError('LaunchFailure')<{readonly code:string}> {}
export class LaunchBridge extends Context.Service<LaunchBridge,{invoke:(request:Request)=>Effect.Effect<LaunchStatus,LaunchFailure>}>()('launcher/LaunchBridge') {}
/** Every native `JobError` code, by its serde name, and the failure code Play shows for it (C-PD1 `job_errors`). */
export const jobErrorFailure:Readonly<Record<string,FailureCode>>={unsupported_schema:'launcher_too_old',platform_unavailable:'platform_unsupported',
  launcher_too_old:'launcher_too_old',io:'local_io',corrupt_state:'state_corrupt',invalid_directory:'game_folder_invalid',stale_revision:'status_stale',
  busy:'operation_busy',unknown_operation:'status_stale',identity_conflict:'identity_conflict',recovery_required:'recovery_required',
  persistence_uncertain:'persistence_uncertain',manifest_unavailable:'manifest_unreachable',invalid_manifest:'manifest_invalid',signing_key_unavailable:'signing_key_unavailable',prerequisites_missing:'prerequisites_missing'};
// A known native refusal keeps its failure code; anything else (a thrown value, an unknown string) is transport.
const refusalCode=(error:unknown):string=>typeof error==='string'&&Object.hasOwn(jobErrorFailure,error)?jobErrorFailure[error]!:'transport';
export const launchBridgeLayer=(invoke:(request:Request)=>Promise<unknown>)=>Layer.succeed(LaunchBridge,{
  invoke:(request)=>Effect.tryPromise({try:()=>invoke(request),catch:error=>{
    if(error!=='launcher_too_old')reportUiEvent('launch','invoke_failed',uiCode(error));
    return new LaunchFailure({code:refusalCode(error)});
  }}).pipe(
    Effect.timeout('5 seconds'),Effect.catchTag('TimeoutError',()=>Effect.fail(new LaunchFailure({code:'transport'}))),
    Effect.flatMap(value=>Schema.decodeUnknownEffect(LaunchStatus,{onExcessProperty:'error'})(value).pipe(Effect.mapError(()=>new LaunchFailure({code:'schema'})))),
  )});
export type LaunchState={status:LaunchStatus|null;pending:boolean;uncertain:boolean;error:string|null};
/** Content readiness is independent of the launcher-update gate. */
export const gameSetupComplete=(status:LaunchStatus)=>status.setup_complete&&!status.native.requires_reopen&&status.observation?.phase!=='unknown'&&(!operationOpen(status.native.operation.operation)||(status.native.operation.operation?.kind==='launch'&&status.native.operation.operation.state!=='reconciliation_required'));
export const launchBlocked=(status:LaunchStatus)=>!!status.prerequisites_installing || !!status.prerequisites_missing?.length || status.launcher_update_required || status.native.requires_reopen || operationOpen(status.native.operation.operation);
export const makeLaunchWorkflow=Effect.gen(function*(){
 const bridge=yield* LaunchBridge;const gate=yield* Semaphore.make(1);
 const state=yield* Ref.make<LaunchState>({status:null,pending:false,uncertain:true,error:null});
 const events=yield* Effect.acquireRelease(PubSub.sliding<LaunchState>({capacity:1,replay:1}),PubSub.shutdown);
 const publish=(next:LaunchState)=>Ref.set(state,next).pipe(Effect.andThen(PubSub.publish(events,next)),Effect.asVoid);
 const accept=(status:LaunchStatus)=>Effect.gen(function*(){const current=yield* Ref.get(state);
   if(current.status && status.native.operation.revision<current.status.native.operation.revision) return yield* Effect.fail(new LaunchFailure({code:'stale_revision'}));
   yield* publish({...current,status,uncertain:status.native.requires_reopen,error:null});return status;});
 const failure=(error:LaunchFailure)=>Ref.get(state).pipe(Effect.flatMap(s=>publish({...s,uncertain:true,error:error.code})));
 const inspectUnlocked=bridge.invoke({command:'inspect',schema_version:1}).pipe(Effect.flatMap(accept),Effect.tapError(failure));
 const inspect=inspectUnlocked.pipe(Semaphore.withPermits(gate,1));
 const play=(id:string)=>Effect.gen(function*(){
   const before=yield* Ref.get(state);
   // Never queue a second click behind the first, or retry an uncertain mutation.
   if(before.pending||before.uncertain||!before.status||launchBlocked(before.status))return yield* Effect.fail(new LaunchFailure({code:'busy'}));
   yield* publish({...before,pending:true,error:null});
   return yield* Effect.gen(function*(){const status=yield* inspectUnlocked;
     if(launchBlocked(status)||!status.installation_id)return yield* Effect.fail(new LaunchFailure({code:'unavailable'}));
     return yield* bridge.invoke({command:'play',schema_version:1,operation_id:id,operation_revision:status.native.operation.revision,installation_id:status.installation_id}).pipe(Effect.flatMap(accept));
   }).pipe(Semaphore.withPermits(gate,1),Effect.tapError(failure),Effect.ensuring(Ref.get(state).pipe(Effect.flatMap(s=>publish({...s,pending:false})))));
 });
 yield* publish(yield* Ref.get(state));
 return {inspect,play,snapshot:Ref.get(state),changes:Stream.fromPubSub(events)};
});
