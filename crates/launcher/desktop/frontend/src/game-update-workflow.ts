import {Context,Data,Effect,Layer,PubSub,Ref,Schema,Semaphore,Stream} from 'effect';
import {NativeSnapshot} from './contract';
const Offer=Schema.Struct({id:Schema.String,installation_id:Schema.String,directory:Schema.String,
 current_digest:Schema.String,target_digest:Schema.String,current_patches:Schema.Array(Schema.String),target_patches:Schema.Array(Schema.String),launcher_update_required:Schema.Boolean});
export const GameUpdateStatus=Schema.Struct({schema_version:Schema.Literal(1),native:NativeSnapshot,
 can_check:Schema.Boolean,checked:Schema.Boolean,offer:Schema.NullOr(Offer)});
export type GameUpdateStatus=typeof GameUpdateStatus.Type;
export type Request={command:'inspect';schema_version:1}|{command:'check';schema_version:1;operation_revision:number};
export class GameUpdateFailure extends Data.TaggedError('GameUpdateFailure')<{readonly code:string}> {}
export class GameUpdateBridge extends Context.Service<GameUpdateBridge,{invoke:(request:Request)=>Effect.Effect<GameUpdateStatus,GameUpdateFailure>}>()('launcher/GameUpdateBridge') {}
export const gameUpdateBridgeLayer=(invoke:(request:Request)=>Promise<unknown>)=>Layer.succeed(GameUpdateBridge,{
 invoke:(request)=>Effect.tryPromise({try:()=>invoke(request),catch:error=>new GameUpdateFailure({code:typeof error==='string'?error:'transport'})}).pipe(
  Effect.timeout('35 seconds'),Effect.catchTag('TimeoutError',()=>Effect.fail(new GameUpdateFailure({code:'transport'}))),
  Effect.flatMap(value=>Schema.decodeUnknownEffect(GameUpdateStatus,{onExcessProperty:'error'})(value).pipe(Effect.mapError(()=>new GameUpdateFailure({code:'schema'}))))
 )});
export type GameUpdateState={status:GameUpdateStatus|null;pending:boolean;uncertain:boolean;error:string|null};
export const makeGameUpdateWorkflow=Effect.gen(function*(){
 const bridge=yield* GameUpdateBridge,gate=yield* Semaphore.make(1);
 const state=yield* Ref.make<GameUpdateState>({status:null,pending:false,uncertain:true,error:null});
 const events=yield* Effect.acquireRelease(PubSub.sliding<GameUpdateState>({capacity:1,replay:1}),PubSub.shutdown);
 const publish=(next:GameUpdateState)=>Ref.set(state,next).pipe(Effect.andThen(PubSub.publish(events,next)),Effect.asVoid);
 const accept=(status:GameUpdateStatus)=>Effect.gen(function*(){
  const before=yield* Ref.get(state);
  if(before.status&&status.native.operation.revision<before.status.native.operation.revision)return yield* Effect.fail(new GameUpdateFailure({code:'stale_revision'}));
  yield* publish({...before,status,uncertain:status.native.requires_reopen,error:null});return status;
 });
 const failure=(error:GameUpdateFailure)=>Ref.get(state).pipe(Effect.flatMap(s=>publish({...s,uncertain:true,error:error.code})));
 const inspectUnlocked=bridge.invoke({command:'inspect',schema_version:1}).pipe(Effect.flatMap(accept),Effect.tapError(failure));
 const inspect=inspectUnlocked.pipe(Semaphore.withPermits(gate,1));
 const check=Effect.gen(function*(){
  const before=yield* Ref.get(state);
  if(before.pending||before.uncertain||!before.status?.can_check)return yield* Effect.fail(new GameUpdateFailure({code:'busy'}));
  yield* publish({...before,pending:true,error:null});
  return yield* Effect.gen(function*(){
   const status=yield* inspectUnlocked;
   if(!status.can_check||status.native.requires_reopen)return yield* Effect.fail(new GameUpdateFailure({code:'busy'}));
   return yield* bridge.invoke({command:'check',schema_version:1,operation_revision:status.native.operation.revision}).pipe(Effect.flatMap(accept));
  }).pipe(Semaphore.withPermits(gate,1),Effect.tapError(failure),Effect.ensuring(Ref.get(state).pipe(Effect.flatMap(s=>publish({...s,pending:false})))));
 });
 yield* publish(yield* Ref.get(state));
 return {inspect,check,snapshot:Ref.get(state),changes:Stream.fromPubSub(events)};
});
