import {Context,Data,Effect,Layer,PubSub,Ref,Schema,Semaphore,Stream} from 'effect';
import {NativeSnapshot} from './contract';
const Offer=Schema.Struct({id:Schema.String,installation_id:Schema.String,directory:Schema.String,
 current_digest:Schema.String,target_digest:Schema.String,current_patches:Schema.Array(Schema.String),target_patches:Schema.Array(Schema.String),launcher_update_required:Schema.Boolean});
export const GameUpdateStatus=Schema.Struct({schema_version:Schema.Literal(1),native:NativeSnapshot,
 can_check:Schema.Boolean,checked:Schema.Boolean,offer:Schema.NullOr(Offer),progress:Schema.NullOr(Schema.Struct({phase:Schema.Literals(['download','extraction']),current:Schema.Number,total:Schema.Number})),maintenance:Schema.NullOr(Schema.Struct({operation_id:Schema.String,directory:Schema.String,previous_digest:Schema.String,target_digest:Schema.String,recovery:Schema.Boolean,discard:Schema.Boolean,rollback:Schema.Boolean,backup:Schema.Literals(['unavailable','not_retained','retained','cleanup_pending','removed'])}))});
export type GameUpdateStatus=typeof GameUpdateStatus.Type;
export type Action='apply'|'recover'|'abandon'|'discard'|'cleanup'|'rollback';
export type Review={action:Action;revision:number;identity:string;directory:string;from:string;to:string};
export type Request={command:'inspect';schema_version:1}|{command:'check';schema_version:1;operation_revision:number}|{command:'apply';schema_version:1;offer_id:string;operation_id:string;operation_revision:number;confirmed:true}|{command:'cancel';schema_version:1;operation_id:string}|{command:'maintain';schema_version:1;action:'recover'|'abandon'|'discard'|'cleanup';operation_id:string;operation_revision:number;confirmed:true}|{command:'rollback';schema_version:1;completed_update:string;operation_id:string;operation_revision:number;confirmed:true};
export class GameUpdateFailure extends Data.TaggedError('GameUpdateFailure')<{readonly code:string}> {}
export class GameUpdateBridge extends Context.Service<GameUpdateBridge,{invoke:(request:Request)=>Effect.Effect<GameUpdateStatus,GameUpdateFailure>}>()('launcher/GameUpdateBridge') {}
export const gameUpdateBridgeLayer=(invoke:(request:Request)=>Promise<unknown>)=>Layer.succeed(GameUpdateBridge,{
 invoke:(request)=>Effect.tryPromise({try:()=>invoke(request),catch:error=>new GameUpdateFailure({code:typeof error==='string'?error:'transport'})}).pipe(
  Effect.timeout('35 seconds'),Effect.catchTag('TimeoutError',()=>Effect.fail(new GameUpdateFailure({code:'transport'}))),
  Effect.flatMap(value=>Schema.decodeUnknownEffect(GameUpdateStatus,{onExcessProperty:'error'})(value).pipe(Effect.mapError(()=>new GameUpdateFailure({code:'schema'}))))
 )});
export type GameUpdateState={status:GameUpdateStatus|null;pending:boolean;uncertain:boolean;error:string|null;review:Review|null};
export const makeGameUpdateWorkflow=Effect.gen(function*(){
 const bridge=yield* GameUpdateBridge,gate=yield* Semaphore.make(1);
 const state=yield* Ref.make<GameUpdateState>({status:null,pending:false,uncertain:true,error:null,review:null});
 const events=yield* Effect.acquireRelease(PubSub.sliding<GameUpdateState>({capacity:1,replay:1}),PubSub.shutdown);
 const publish=(next:GameUpdateState)=>Ref.set(state,next).pipe(Effect.andThen(PubSub.publish(events,next)),Effect.asVoid);
 const accept=(status:GameUpdateStatus)=>Effect.gen(function*(){
  const before=yield* Ref.get(state);
  if(before.status&&status.native.operation.revision<before.status.native.operation.revision)return yield* Effect.fail(new GameUpdateFailure({code:'stale_revision'}));
  yield* publish({...before,status,uncertain:status.native.requires_reopen,error:null,review:before.review&&sameReview(before.review,status)?before.review:null});return status;
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
 const review=(action:Action)=>Effect.gen(function*(){
  const before=yield* Ref.get(state);
  if(before.pending||before.uncertain||!before.status)return yield* Effect.fail(new GameUpdateFailure({code:'busy'}));
  const reviewed=makeReview(action,before.status);
  if(!reviewed)return yield* Effect.fail(new GameUpdateFailure({code:'unavailable'}));
  yield* publish({...before,review:reviewed,error:null});
 });
 const dismiss=Ref.get(state).pipe(Effect.flatMap(s=>publish({...s,review:null})));
 const confirm=(id:string)=>Effect.gen(function*(){
  const before=yield* Ref.get(state),reviewed=before.review;
  if(before.pending||before.uncertain||!reviewed)return yield* Effect.fail(new GameUpdateFailure({code:'busy'}));
  yield* publish({...before,pending:true,error:null});
  return yield* Effect.gen(function*(){
   const status=yield* inspectUnlocked;
   if(!sameReview(reviewed,status))return yield* Effect.fail(new GameUpdateFailure({code:'stale_revision'}));
   const base={schema_version:1 as const,operation_revision:reviewed.revision,confirmed:true as const};
   const request:Request=reviewed.action==='apply'?{...base,command:'apply',offer_id:reviewed.identity,operation_id:id}:
    reviewed.action==='rollback'?{...base,command:'rollback',completed_update:reviewed.identity,operation_id:id}:
    {...base,command:'maintain',action:reviewed.action,operation_id:reviewed.identity};
   return yield* bridge.invoke(request).pipe(Effect.flatMap(accept));
  }).pipe(Semaphore.withPermits(gate,1),Effect.tapError(failure),Effect.ensuring(Ref.get(state).pipe(Effect.flatMap(s=>publish({...s,pending:false,review:null})))));
 });
 const cancel=Effect.gen(function*(){
  const before=yield* Ref.get(state),op=before.status?.native.operation.operation;
  if(before.pending||before.uncertain||op?.kind!=='update'||!['starting','running'].includes(op.state))return yield* Effect.fail(new GameUpdateFailure({code:'busy'}));
  yield* publish({...before,pending:true,error:null});
  return yield* bridge.invoke({command:'cancel',schema_version:1,operation_id:op.id}).pipe(Effect.flatMap(accept),Semaphore.withPermits(gate,1),Effect.tapError(failure),Effect.ensuring(Ref.get(state).pipe(Effect.flatMap(s=>publish({...s,pending:false})))));
 });
 return {inspect,check,review,dismiss,confirm,cancel,snapshot:Ref.get(state),changes:Stream.fromPubSub(events)};
});

function makeReview(action:Action,status:GameUpdateStatus):Review|null {
 if(status.native.requires_reopen)return null;
 const revision=status.native.operation.revision,offer=status.offer,m=status.maintenance;
 if(action==='apply')return offer&&!offer.launcher_update_required?{action,revision,identity:offer.id,directory:offer.directory,from:offer.current_digest,to:offer.target_digest}:null;
 if(!m)return null;
 const allowed=(action==='recover'||action==='abandon')?m.recovery:action==='discard'?m.discard:action==='rollback'?m.rollback:['retained','cleanup_pending'].includes(m.backup);
 return allowed?{action,revision,identity:m.operation_id,directory:m.directory,from:action==='rollback'?m.target_digest:m.previous_digest,to:action==='rollback'?m.previous_digest:m.target_digest}:null;
}
function sameReview(review:Review,status:GameUpdateStatus):boolean {
 return JSON.stringify(review)===JSON.stringify(makeReview(review.action,status));
}
