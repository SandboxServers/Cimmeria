import { Context, Data, Effect, Layer, PubSub, Ref, Schema, Semaphore, Stream } from 'effect';
import { NativeSnapshot } from './contract';
const Pid=Schema.Int.check(Schema.isBetween({minimum:1,maximum:4294967295}));
export const LaunchStatus=Schema.Struct({schema_version:Schema.Literal(1),native:NativeSnapshot,
  installation_id:Schema.NullOr(Schema.String),resources_available:Schema.Boolean,launcher_update_required:Schema.Boolean,
  observation:Schema.NullOr(Schema.Union([
    Schema.Struct({phase:Schema.Literals(['preparing','not_started','cancelled','unknown'])}),
    Schema.Struct({phase:Schema.Literal('host_started'),host_pid:Pid}),
    Schema.Struct({phase:Schema.Literal('process_started'),host_pid:Pid,guest_pid:Pid}),
    Schema.Struct({phase:Schema.Literal('process_exited'),host_pid:Pid,guest_pid:Pid,code:Schema.Int,early:Schema.Boolean}),
  ]))});
export type LaunchStatus=typeof LaunchStatus.Type;
export type Request={command:'inspect';schema_version:1}|{command:'play';schema_version:1;operation_id:string;operation_revision:number;installation_id:string};
export class LaunchFailure extends Data.TaggedError('LaunchFailure')<{readonly code:string}> {}
export class LaunchBridge extends Context.Service<LaunchBridge,{invoke:(request:Request)=>Effect.Effect<LaunchStatus,LaunchFailure>}>()('launcher/LaunchBridge') {}
export const launchBridgeLayer=(invoke:(request:Request)=>Promise<unknown>)=>Layer.succeed(LaunchBridge,{
  invoke:(request)=>Effect.tryPromise({try:()=>invoke(request),catch:error=>new LaunchFailure({code:error==='launcher_too_old'?'launcher_too_old':'transport'})}).pipe(
    Effect.timeout('5 seconds'),Effect.catchTag('TimeoutError',()=>Effect.fail(new LaunchFailure({code:'transport'}))),
    Effect.flatMap(value=>Schema.decodeUnknownEffect(LaunchStatus,{onExcessProperty:'error'})(value).pipe(Effect.mapError(()=>new LaunchFailure({code:'schema'})))),
  )});
export type LaunchState={status:LaunchStatus|null;pending:boolean;uncertain:boolean;error:string|null};
export const launchBlocked=(status:LaunchStatus)=>status.launcher_update_required || status.native.requires_reopen || !!status.native.operation.operation &&
  !['succeeded','failed','cancelled'].includes(status.native.operation.operation.state);
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
