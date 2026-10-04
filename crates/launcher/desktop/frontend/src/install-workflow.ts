import { Context, Data, Effect, Layer, PubSub, Ref, Schedule, Schema, Semaphore, Stream } from 'effect';
import { NativeSnapshot } from './contract';

const Count = Schema.Int.check(Schema.isBetween({minimum:0, maximum:Number.MAX_SAFE_INTEGER}));
export const InstallStatus = Schema.Struct({
  schema_version:Schema.Literal(1), native:NativeSnapshot, install_supported:Schema.Boolean, can_resume:Schema.Boolean, can_reconcile:Schema.Boolean, can_retry:Schema.Boolean,
  runtime_setup:Schema.NullOr(Schema.String),
  repair:Schema.optionalKey(Schema.Struct({directory:Schema.optionalKey(Schema.NullOr(Schema.String)),target:Schema.NullOr(Schema.Struct({installation_id:Schema.String,directory:Schema.String})),recovery:Schema.Boolean,cleanup:Schema.Boolean,backup:Schema.optionalKey(Schema.Literals(['unavailable','not_retained','retained','cleanup_pending','removed']))})),
  uninstall:Schema.NullOr(Schema.Struct({installation_id:Schema.String,directory:Schema.String,recovery:Schema.Boolean})),
  progress:Schema.NullOr(Schema.Struct({phase:Schema.Literals(['download','extraction']), current:Count, total:Count})),
  outcome:Schema.NullOr(Schema.Literals(['content_prepared','cancelled','destination_unavailable','install_failed','content_invalid','reconciliation_required','rosetta_required','runtime_unavailable'])),
});
export type InstallStatus = typeof InstallStatus.Type;
export type InstallRequest = {command:'inspect';schema_version:1} |
  {command:'install';schema_version:1;operation_id:string;operation_revision:number;preferences_revision:number} |
  {command:'prepare_runtime';schema_version:1;operation_id:string;operation_revision:number;installation_id:string} |
  {command:'uninstall';schema_version:1;operation_id:string;operation_revision:number;installation_id:string;confirmed:true} |
  {command:'clean_failed';schema_version:1;operation_id:string;operation_revision:number;confirmed:true} |
  {command:'repair';schema_version:1;operation_id:string;operation_revision:number;installation_id:string;confirmed:true} |
  {command:'recover_repair'|'abandon_repair'|'cleanup_repair';schema_version:1;operation_id:string;operation_revision:number;confirmed:true} |
  {command:'cancel';schema_version:1;operation_id:string} |
  {command:'resume'|'reconcile';schema_version:1;operation_id:string;operation_revision:number};
const codes = ['launcher_too_old','unsupported_schema','platform_unavailable','io','corrupt_state','invalid_directory','stale_revision',
  'busy','unknown_operation','identity_conflict','recovery_required','persistence_uncertain','manifest_unavailable',
  'invalid_manifest','signing_key_unavailable','transport','schema'] as const;
export class InstallFailure extends Data.TaggedError('InstallFailure')<{readonly code:typeof codes[number]}> {}
export class InstallBridge extends Context.Service<InstallBridge, {
  readonly invoke:(request:InstallRequest)=>Effect.Effect<InstallStatus,InstallFailure>;
}>()('launcher/InstallBridge') {}
export const installBridgeLayer = (invoke:(request:InstallRequest)=>Promise<unknown>) => Layer.succeed(InstallBridge, {
  invoke:(request)=>Effect.tryPromise({try:()=>invoke(request), catch:error=>new InstallFailure({
    code:typeof error==='string' && codes.includes(error as typeof codes[number]) ? error as typeof codes[number] : 'transport',
  })}).pipe(
    // Install may fetch two signed-release resources, each with a 15s deadline.
    Effect.timeout(request.command==='install'||request.command==='reconcile'||request.command==='clean_failed'||request.command==='uninstall' ? '35 seconds' : '5 seconds'),
    Effect.catchTag('TimeoutError',()=>Effect.fail(new InstallFailure({code:'transport'}))),
    Effect.flatMap(value=>Schema.decodeUnknownEffect(InstallStatus,{onExcessProperty:'error'})(value).pipe(
      Effect.mapError(()=>new InstallFailure({code:'schema'})),
    )),
  ),
});
export type InstallViewState = {readonly status:InstallStatus|null;readonly busy:boolean;
  readonly needsInspection:boolean;readonly error:InstallFailure['code']|null};
export const operationActive = (status:InstallStatus) => {
  const phase=status.native.operation.operation?.state;
  return phase==='starting'||phase==='running'||phase==='cancel_requested';
};

/** One application scope; observation interruption never cancels native work. */
export const makeInstallWorkflow = Effect.gen(function*(){
  const bridge=yield* InstallBridge;
  const gate=yield* Semaphore.make(1);
  const state=yield* Ref.make<InstallViewState>({status:null,busy:false,needsInspection:true,error:null});
  const events=yield* Effect.acquireRelease(PubSub.sliding<InstallViewState>({capacity:1,replay:1}),PubSub.shutdown);
  const publish=(next:InstallViewState)=>Ref.set(state,next).pipe(Effect.andThen(PubSub.publish(events,next)),Effect.asVoid);
  yield* publish(yield* Ref.get(state));
  const failure=(error:InstallFailure)=>Ref.get(state).pipe(Effect.flatMap(current=>publish({...current,needsInspection:true,error:error.code})));
  const accept=(next:InstallStatus)=>Effect.gen(function*(){
    const current=yield* Ref.get(state);
    if(current.status && (next.native.operation.revision<current.status.native.operation.revision ||
      next.native.preferences.revision<current.status.native.preferences.revision)) {
      return yield* Effect.fail(new InstallFailure({code:'stale_revision'}));
    }
    yield* publish({...current,status:next,needsInspection:next.native.requires_reopen,
      error:next.native.requires_reopen?'persistence_uncertain':null});
    return next;
  });
  const inspectUnlocked=bridge.invoke({command:'inspect',schema_version:1}).pipe(
    Effect.retry({times:2,schedule:Schedule.exponential('100 millis'),while:error=>error.code==='transport'}),
    Effect.flatMap(accept),Effect.tapError(failure),
  );
  const inspect=inspectUnlocked.pipe(Semaphore.withPermits(gate,1));
  const mutate=(request:(status:InstallStatus)=>InstallRequest, accepts:(status:InstallStatus)=>boolean=()=>true)=>Effect.gen(function*(){
    // Always refresh before admitting an intent, including changes made by settings.
    const latest=yield* inspectUnlocked;
    if(latest.native.requires_reopen) return yield* Effect.fail(new InstallFailure({code:'persistence_uncertain'}));
    if(!accepts(latest))return yield* Effect.fail(new InstallFailure({code:'identity_conflict'}));
    const current=yield* Ref.get(state);
    yield* publish({...current,busy:true,needsInspection:true,error:null});
    // Never replay a mutation after a lost reply. Native work can outlive invoke.
    return yield* bridge.invoke(request(latest)).pipe(Effect.flatMap(accept));
  }).pipe(Effect.tapError(failure),
    Effect.ensuring(Ref.get(state).pipe(Effect.flatMap(current=>publish({...current,busy:false})))),
    Semaphore.withPermits(gate,1));
  const install=(id:string)=>mutate(status=>({command:'install',schema_version:1,operation_id:id,
    operation_revision:status.native.operation.revision,preferences_revision:status.native.preferences.revision}));
  const cancel=(id:string)=>mutate(()=>({command:'cancel',schema_version:1,operation_id:id}));
  const recover=(command:'resume'|'reconcile',id:string)=>mutate(status=>({command,schema_version:1,
    operation_id:id,operation_revision:status.native.operation.revision}));
  const observe=(id:string)=>Effect.gen(function*(){
    let status=yield* inspect;
    while(true) {
      if(status.native.requires_reopen) return yield* Effect.fail(new InstallFailure({code:'persistence_uncertain'}));
      if(status.native.operation.operation?.id!==id) return yield* Effect.fail(new InstallFailure({code:'unknown_operation'}));
      if(!operationActive(status)) return status;
      // No permit held while waiting; cancel/settings can proceed between polls.
      yield* Effect.sleep('250 millis');
      status=yield* inspect;
    }
  }).pipe(Effect.tapError(failure));
  const prepareRuntime=(id:string,installationId:string,previousId:string)=>mutate(status=>({
    command:'prepare_runtime',schema_version:1,operation_id:id,operation_revision:status.native.operation.revision,
    installation_id:installationId,
  }),status=>status.runtime_setup===installationId&&status.native.operation.operation?.id===previousId);
  const repair=(id:string,installationId:string)=>mutate(status=>({command:'repair',schema_version:1,
    operation_id:id,operation_revision:status.native.operation.revision,installation_id:installationId,confirmed:true}),
    status=>status.repair?.target?.installation_id===installationId);
  const repairAction=(command:'recover_repair'|'abandon_repair'|'cleanup_repair',id:string)=>mutate(status=>({
    command,schema_version:1,operation_id:id,operation_revision:status.native.operation.revision,confirmed:true,
  }),status=>status.native.operation.operation?.id===id && (command==='cleanup_repair'?status.repair?.cleanup===true:status.repair?.recovery===true));
  return {inspect,install,cancel,prepareRuntime,repair,repairAction,
    uninstall:(id:string,installationId:string)=>mutate(status=>({command:'uninstall',schema_version:1,operation_id:id,
      operation_revision:status.native.operation.revision,installation_id:installationId,confirmed:true})),resume:(id:string)=>recover('resume',id),
    cleanFailed:(id:string)=>mutate(status=>({command:'clean_failed',schema_version:1,operation_id:id,
      operation_revision:status.native.operation.revision,confirmed:true})),
    reconcile:(id:string)=>recover('reconcile',id),observe,snapshot:Ref.get(state),changes:Stream.fromPubSub(events)};
});
