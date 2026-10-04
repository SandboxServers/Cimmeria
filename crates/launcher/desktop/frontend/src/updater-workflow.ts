import {Context,Data,Effect,Layer,PubSub,Ref,Schema,Stream} from 'effect';
const Revision=Schema.Int.check(Schema.isBetween({minimum:0,maximum:Number.MAX_SAFE_INTEGER}));
const ErrorCode=Schema.Union([Schema.String,Schema.Struct({storage:Schema.String})]);
export const UpdaterStatus=Schema.Struct({schema_version:Schema.Literal(1),revision:Revision,operation_revision:Revision,
 phase:Schema.Literals(['disabled','idle','checking','available','up_to_date','downloading','verifying','ready','failed']),
 offer:Schema.NullOr(Schema.Struct({id:Schema.String,version:Schema.String,notes:Schema.String})),
 failure:Schema.NullOr(ErrorCode),requires_reopen:Schema.Boolean});
export type UpdaterStatus=typeof UpdaterStatus.Type;
export type UpdaterRequest={command:'inspect';schema_version:1}|{command:'check';schema_version:1;revision:number;operation_revision:number}|{command:'prepare';schema_version:1;revision:number;operation_revision:number;offer_id:string};
export class UpdaterFailure extends Data.TaggedError('UpdaterFailure')<{readonly code:string}> {}
const codes=new Set(['disabled','busy','stale_revision','stale_offer','policy','feed','platform','not_newer','signature','signed_version','size','transport','timeout','interrupted','io','corrupt','unsupported_schema','unsafe_file','persistence_uncertain','too_large','in_use']);
export const updaterFailureCode=(value:unknown):string=>{const v=typeof value==='object'&&value!==null&&'storage' in value?value.storage:value;return typeof v==='string'&&codes.has(v)?v:'transport';};
export class UpdaterBridge extends Context.Service<UpdaterBridge,{call:(request:UpdaterRequest)=>Effect.Effect<UpdaterStatus,UpdaterFailure>}>()('launcher/UpdaterBridge') {}
export const updaterBridgeLayer=(call:(request:UpdaterRequest)=>Promise<unknown>)=>Layer.succeed(UpdaterBridge,{
 call:request=>Effect.tryPromise({try:()=>call(request),catch:e=>new UpdaterFailure({code:updaterFailureCode(e)})}).pipe(
  Effect.timeout(request.command==='prepare'?'330 seconds':'45 seconds'),Effect.catchTag('TimeoutError',()=>Effect.fail(new UpdaterFailure({code:'timeout'}))),
  Effect.flatMap(value=>Schema.decodeUnknownEffect(UpdaterStatus,{onExcessProperty:'error'})(value).pipe(Effect.mapError(()=>new UpdaterFailure({code:'schema'})))))
});
export type UpdaterState={status:UpdaterStatus|null;pending:'inspect'|'check'|'prepare'|null;uncertain:boolean;error:string|null};
export const makeUpdaterWorkflow=Effect.gen(function*(){
 const bridge=yield* UpdaterBridge;
 const state=yield* Ref.make<UpdaterState>({status:null,pending:null,uncertain:true,error:null});
 const events=yield* Effect.acquireRelease(PubSub.sliding<UpdaterState>({capacity:1,replay:1}),PubSub.shutdown);
 const publish=(s:UpdaterState)=>Ref.set(state,s).pipe(Effect.andThen(PubSub.publish(events,s)),Effect.asVoid);
 const run=(action:'inspect'|'check'|'prepare')=>Effect.gen(function*(){
  const before=yield* Ref.get(state),status=before.status;
  if(before.pending)return yield* Effect.fail(new UpdaterFailure({code:'busy'}));
  if(action!=='inspect'&&(before.uncertain||!status||status.requires_reopen))return yield* Effect.fail(new UpdaterFailure({code:'stale_revision'}));
  if(action==='prepare'&&(status?.phase!=='available'||!status.offer))return yield* Effect.fail(new UpdaterFailure({code:'stale_offer'}));
  yield* publish({...before,pending:action,error:null});
  const request:UpdaterRequest=action==='inspect'?{command:'inspect',schema_version:1}:action==='check'?{command:'check',schema_version:1,revision:status!.revision,operation_revision:status!.operation_revision}:{command:'prepare',schema_version:1,revision:status!.revision,operation_revision:status!.operation_revision,offer_id:status!.offer!.id};
  return yield* bridge.call(request).pipe(Effect.tap(status=>publish({status,pending:action,uncertain:status.requires_reopen,error:null})),
   Effect.tapError(error=>Ref.get(state).pipe(Effect.flatMap(s=>publish({...s,uncertain:true,error:error.code})))),
   Effect.ensuring(Ref.get(state).pipe(Effect.flatMap(s=>publish({...s,pending:null})))));
 });
 yield* publish(yield* Ref.get(state));
 return {run,snapshot:Ref.get(state),changes:Stream.fromPubSub(events)};
});
