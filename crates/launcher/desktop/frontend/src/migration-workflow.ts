import {Context,Data,Effect,Layer,PubSub,Ref,Schema,Stream} from 'effect';
import {NativeSnapshot} from './contract';
const Revision=Schema.Int.check(Schema.isBetween({minimum:0,maximum:Number.MAX_SAFE_INTEGER}));
const Legacy=Schema.Struct({
 source:Schema.Struct({launcher_directory:Schema.String,game_directory:Schema.String}),
 identity:Schema.Struct({schema_version:Schema.Literal(1),install_id:Schema.String,machine_id:Schema.String,first_seen_ms:Schema.Int,created_by_launcher_version:Schema.String}),
 config:Schema.Struct({schema_version:Schema.Literals([1,2]),install_path:Schema.String,manifest_url:Schema.String,
  login_servers:Schema.Array(Schema.Struct({name:Schema.String,url:Schema.String})),
  telemetry:Schema.Struct({opted_in:Schema.Boolean,prompt_answered:Schema.Boolean,auth_url:Schema.String}),
  client_patches:Schema.Struct({enabled:Schema.Boolean,dll_override:Schema.NullOr(Schema.String)})}),
 ledger:Schema.Struct({applied_patches:Schema.Array(Schema.String),seed_sha256:Schema.NullOr(Schema.String),seed_adopted:Schema.Boolean}),
 confirmation:Schema.String,
});
export const MigrationStatus=Schema.Struct({schema_version:Schema.Literal(1),native:NativeSnapshot,imported:Schema.NullOr(Legacy),preview:Schema.NullOr(Schema.Struct({imported:Legacy,preferences_revision:Revision}))});
export type MigrationStatus=typeof MigrationStatus.Type;
export type Request={command:'inspect'|'dismiss';schema_version:1}|{command:'confirm';schema_version:1;confirmation:string;preferences_revision:number;confirmed:true};
export class MigrationFailure extends Data.TaggedError('MigrationFailure')<{readonly code:string}> {}
const codes=new Set(['busy','missing_source','unsupported_schema','invalid_source','source_changed','conflict','io','stale_revision','persistence_uncertain','too_large','unsafe_file','in_use']);
const failure=(e:unknown)=>{const v=typeof e==='object'&&e!==null&&'storage' in e?e.storage:e;return new MigrationFailure({code:typeof v==='string'&&codes.has(v)?v:'transport'});};
export class MigrationBridge extends Context.Service<MigrationBridge,{call:(request:Request|'choose')=>Effect.Effect<MigrationStatus,MigrationFailure>}>()('launcher/MigrationBridge') {}
export const migrationBridgeLayer=(call:(request:Request|'choose')=>Promise<unknown>)=>Layer.succeed(MigrationBridge,{
 call:request=>Effect.tryPromise({try:()=>call(request),catch:failure}).pipe(
 // A human may leave the native chooser open; mutation timeouts require reconciliation.
 effect=>request==='choose'?effect:effect.pipe(Effect.timeout('5 seconds'),Effect.catchTag('TimeoutError',()=>Effect.fail(new MigrationFailure({code:'transport'})))),
 Effect.flatMap(value=>Schema.decodeUnknownEffect(MigrationStatus,{onExcessProperty:'error'})(value).pipe(Effect.mapError(()=>new MigrationFailure({code:'schema'})))))
});
export type MigrationState={status:MigrationStatus|null;pending:boolean;uncertain:boolean;error:string|null};
export const makeMigrationWorkflow=Effect.gen(function*(){
 const bridge=yield* MigrationBridge;
 const state=yield* Ref.make<MigrationState>({status:null,pending:false,uncertain:true,error:null});
 const events=yield* Effect.acquireRelease(PubSub.sliding<MigrationState>({capacity:1,replay:1}),PubSub.shutdown);
 const publish=(s:MigrationState)=>Ref.set(state,s).pipe(Effect.andThen(PubSub.publish(events,s)),Effect.asVoid);
 const run=(action:'choose'|'inspect'|'dismiss'|'confirm')=>Effect.gen(function*(){
  const before=yield* Ref.get(state);
  if(before.pending)return yield* Effect.fail(new MigrationFailure({code:'busy'}));
  const preview=before.status?.preview;
  if(action==='confirm'&&(before.uncertain||!preview))return yield* Effect.fail(new MigrationFailure({code:'source_changed'}));
  yield* publish({...before,pending:true,error:null});
  const request:Request|'choose'=action==='choose'?'choose':action==='confirm'?{command:'confirm',schema_version:1,confirmation:preview!.imported.confirmation,preferences_revision:preview!.preferences_revision,confirmed:true}:{command:action,schema_version:1};
  return yield* bridge.call(request).pipe(Effect.tap(status=>publish({status,pending:true,uncertain:status.native.requires_reopen,error:null})),
   Effect.tapError(error=>Ref.get(state).pipe(Effect.flatMap(s=>publish({...s,uncertain:true,error:error.code})))),
   Effect.ensuring(Ref.get(state).pipe(Effect.flatMap(s=>publish({...s,pending:false})))));
 });
 yield* publish(yield* Ref.get(state));
 return {run,snapshot:Ref.get(state),changes:Stream.fromPubSub(events)};
});
