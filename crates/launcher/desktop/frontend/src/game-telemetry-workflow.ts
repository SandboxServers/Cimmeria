import {Context,Data,Effect,Layer,PubSub,Ref,Schema,Stream} from 'effect';
// Its own command and record: launcher-summary consent (`save_preferences`) never reaches it.
export const GameTelemetryStatus=Schema.Struct({schema_version:Schema.Literal(1),available:Schema.Boolean,opted_in:Schema.Boolean,
 last_outcome:Schema.NullOr(Schema.Literals(['attached','session_unavailable','endpoint_refused','session_not_written']))});
export type GameTelemetryStatus=typeof GameTelemetryStatus.Type;
export type Request={command:'inspect';schema_version:1}|{command:'set';schema_version:1;opted_in:boolean};
export class GameTelemetryFailure extends Data.TaggedError('GameTelemetryFailure')<{readonly code:string}> {}
const codes=new Set(['unsupported_schema','platform_unavailable','io','corrupt_state','busy','persistence_uncertain']);
const failure=(e:unknown)=>new GameTelemetryFailure({code:typeof e==='string'&&codes.has(e)?e:'transport'});
export class GameTelemetryBridge extends Context.Service<GameTelemetryBridge,{call:(request:Request)=>Effect.Effect<GameTelemetryStatus,GameTelemetryFailure>}>()('launcher/GameTelemetryBridge') {}
export const gameTelemetryBridgeLayer=(call:(request:Request)=>Promise<unknown>)=>Layer.succeed(GameTelemetryBridge,{
 call:request=>Effect.tryPromise({try:()=>call(request),catch:failure}).pipe(
 Effect.timeout('5 seconds'),Effect.catchTag('TimeoutError',()=>Effect.fail(new GameTelemetryFailure({code:'transport'}))),
 Effect.flatMap(value=>Schema.decodeUnknownEffect(GameTelemetryStatus,{onExcessProperty:'error'})(value).pipe(Effect.mapError(()=>new GameTelemetryFailure({code:'schema'})))))
});
export type GameTelemetryState={status:GameTelemetryStatus|null;pending:boolean;uncertain:boolean;error:string|null};
export const makeGameTelemetryWorkflow=Effect.gen(function*(){
 const bridge=yield* GameTelemetryBridge;
 const state=yield* Ref.make<GameTelemetryState>({status:null,pending:false,uncertain:true,error:null});
 const events=yield* Effect.acquireRelease(PubSub.sliding<GameTelemetryState>({capacity:1,replay:1}),PubSub.shutdown);
 const publish=(s:GameTelemetryState)=>Ref.set(state,s).pipe(Effect.andThen(PubSub.publish(events,s)),Effect.asVoid);
 const run=(request:Request)=>Effect.gen(function*(){
  const before=yield* Ref.get(state);
  if(before.pending)return yield* Effect.fail(new GameTelemetryFailure({code:'busy'}));
  // A choice may not be sent on top of an unconfirmed one; inspect first.
  if(request.command==='set'&&before.uncertain)return yield* Effect.fail(new GameTelemetryFailure({code:'busy'}));
  yield* publish({...before,pending:true,error:null});
  // Sent once. A lost reply leaves the choice uncertain until the next inspect; it is never replayed.
  return yield* bridge.call(request).pipe(Effect.tap(status=>publish({status,pending:true,uncertain:false,error:null})),
   Effect.tapError(error=>Ref.get(state).pipe(Effect.flatMap(s=>publish({...s,uncertain:true,error:error.code})))),
   Effect.ensuring(Ref.get(state).pipe(Effect.flatMap(s=>publish({...s,pending:false})))));
 });
 yield* publish(yield* Ref.get(state));
 return {inspect:run({command:'inspect',schema_version:1}),set:(opted_in:boolean)=>run({command:'set',schema_version:1,opted_in}),snapshot:Ref.get(state),changes:Stream.fromPubSub(events)};
});
