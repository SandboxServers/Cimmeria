import {Context,Data,Effect,Layer,PubSub,Ref,Stream} from 'effect';
import {decodeSeedArchiveStatus,type SeedArchiveCommand,type SeedArchiveStatus} from './contract';
// Its own bridge: the archive is picked and checked natively, never by a path the webview names.
export class SeedArchiveFailure extends Data.TaggedError('SeedArchiveFailure')<{readonly code:string}> {}
const codes=new Set(['not_a_file','size_mismatch','hash_mismatch','changed','cancelled','busy','io','release_unavailable']);
const failure=(e:unknown)=>new SeedArchiveFailure({code:typeof e==='string'&&codes.has(e)?e:'transport'});
/** `'choose'` opens the native file window; the other requests go to `seed_archive_command`. */
export type SeedArchiveRequest=SeedArchiveCommand|'choose';
export class SeedArchiveBridge extends Context.Service<SeedArchiveBridge,{call:(request:SeedArchiveRequest)=>Effect.Effect<SeedArchiveStatus,SeedArchiveFailure>}>()('launcher/SeedArchiveBridge') {}
export const seedArchiveBridgeLayer=(invoke:(command:string,args?:Record<string,unknown>)=>Promise<unknown>)=>Layer.succeed(SeedArchiveBridge,{
 // Choosing waits on the person in the file window, so only the plain commands get a timeout.
 call:request=>(request==='choose'
  ?Effect.tryPromise({try:()=>invoke('choose_seed_archive'),catch:failure})
  :Effect.tryPromise({try:()=>invoke('seed_archive_command',{request}),catch:failure}).pipe(Effect.timeout('5 seconds'),
   Effect.catchTag('TimeoutError',()=>Effect.fail(new SeedArchiveFailure({code:'transport'}))))).pipe(
  Effect.flatMap(value=>decodeSeedArchiveStatus(value).pipe(Effect.mapError(()=>new SeedArchiveFailure({code:'schema'})))))
});
export type SeedArchiveState={status:SeedArchiveStatus|null;phase:'choosing'|'checking'|'inspecting'|'forgetting'|null;cancelling:boolean;error:string|null};
/** `pollMillis` spaces the progress checks while a check runs. */
export const makeSeedArchiveWorkflow=(pollMillis:number=500)=>Effect.gen(function*(){
 const bridge=yield* SeedArchiveBridge;
 const state=yield* Ref.make<SeedArchiveState>({status:null,phase:null,cancelling:false,error:null});
 const events=yield* Effect.acquireRelease(PubSub.sliding<SeedArchiveState>({capacity:1,replay:1}),PubSub.shutdown);
 const publish=(s:SeedArchiveState)=>Ref.set(state,s).pipe(Effect.andThen(PubSub.publish(events,s)),Effect.asVoid);
 const patch=(p:Partial<SeedArchiveState>)=>Ref.get(state).pipe(Effect.flatMap(s=>publish({...s,...p})));
 const busy=()=>new SeedArchiveFailure({code:'busy'});
 // Follows a running check: each reply is shown. The loop keeps going while a check reports progress, and
 // `awaiting` lets the reply that starts a check get one more poll, since its first report may not be there yet.
 const follow=(status:SeedArchiveStatus,awaiting:boolean):Effect.Effect<SeedArchiveStatus,SeedArchiveFailure>=>status.verifying===null&&!awaiting
  ?Effect.succeed(status)
  :Effect.sleep(`${pollMillis} millis`).pipe(Effect.andThen(bridge.call({command:'inspect',schema_version:1})),
   Effect.tap(s=>patch({status:s,...(s.verifying?{phase:'checking' as const}:{})})),Effect.flatMap(s=>follow(s,false)));
 // Sent once. A lost reply shows as a transport failure and is never replayed; the next inspect re-reads native state.
 const choose=Effect.gen(function*(){
  const before=yield* Ref.get(state);
  if(before.phase!==null)return yield* Effect.fail(busy());
  yield* publish({...before,phase:'choosing',error:null});
  return yield* bridge.call('choose').pipe(
   Effect.tap(s=>patch({status:s,phase:s.verifying?'checking':'choosing'})),
   Effect.flatMap(s=>follow(s,s.last_error===null)),
   Effect.tap(s=>patch({status:s,error:s.last_error})),
   Effect.tapError(e=>patch({error:e.code})),
   Effect.ensuring(patch({phase:null,cancelling:false})));
 });
 // Sent once, like choose: the native side verifies the Downloads copy it already found.
 const accept=Effect.gen(function*(){
  const before=yield* Ref.get(state);
  if(before.phase!==null)return yield* Effect.fail(busy());
  yield* publish({...before,phase:'checking',error:null});
  return yield* bridge.call({command:'accept_offer',schema_version:1}).pipe(
   Effect.tap(s=>patch({status:s})),
   Effect.flatMap(s=>follow(s,s.last_error===null)),
   Effect.tap(s=>patch({status:s,error:s.last_error})),
   Effect.tapError(e=>patch({error:e.code})),
   Effect.ensuring(patch({phase:null,cancelling:false})));
 });
 // Allowed only while a check runs, and only once per check.
 const cancel=Effect.gen(function*(){
  const before=yield* Ref.get(state);
  if(before.phase!=='checking'||before.cancelling)return yield* Effect.fail(busy());
  yield* patch({cancelling:true,error:null});
  return yield* bridge.call({command:'cancel',schema_version:1}).pipe(Effect.tap(s=>patch({status:s})),
   Effect.tapError(e=>patch({cancelling:false,error:e.code})));
 });
 const gated=(phase:'inspecting'|'forgetting',request:SeedArchiveCommand)=>Effect.gen(function*(){
  const before=yield* Ref.get(state);
  if(before.phase!==null)return yield* Effect.fail(busy());
  yield* publish({...before,phase,error:null});
  return yield* bridge.call(request).pipe(Effect.tap(s=>patch({status:s,error:s.last_error})),
   Effect.tapError(e=>patch({error:e.code})),Effect.ensuring(patch({phase:null})));
 });
 yield* publish(yield* Ref.get(state));
 return {
  inspect:gated('inspecting',{command:'inspect',schema_version:1}),
  forget:gated('forgetting',{command:'forget',schema_version:1}),
  choose,accept,cancel,snapshot:Ref.get(state),changes:Stream.fromPubSub(events),
 };
});
