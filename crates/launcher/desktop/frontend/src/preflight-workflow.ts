import {Context,Data,Effect,Layer,Schema} from 'effect';
import {FailureCode} from './failure-codes';
import {jobErrorFailure} from './launch-workflow';
import {reportUiEvent,type UiArea} from './ui-event';
// The preflight report (C-PD4), mirroring the Rust `preflight::Report` field for field.
export const PreflightOperation=Schema.Literals(['play','install','repair']);
export type PreflightOperation=typeof PreflightOperation.Type;
export const CheckId=Schema.Literals(['rosetta','wine_runtime','compatibility_helper','disk_space','game_folder','release_manifest','app_location','physx']);
export type CheckId=typeof CheckId.Type;
const Bytes=Schema.NullOr(Schema.Number);
export const Check=Schema.Struct({id:CheckId,status:Schema.Literals(['ok','missing','warning','unknown']),
  action:Schema.Literals(['none','auto_install','download','ask_user','manual']),failure:Schema.NullOr(FailureCode),blocking:Schema.Boolean,
  bytes_needed:Bytes,bytes_free:Bytes,download_bytes:Bytes});
export type Check=typeof Check.Type;
export const Report=Schema.Struct({schema_version:Schema.Literal(1),operation:PreflightOperation,checks:Schema.Array(Check),ready:Schema.Boolean});
export type Report=typeof Report.Type;
export const decodeReport=(value:unknown)=>Schema.decodeUnknownEffect(Report,{onExcessProperty:'error'})(value);
export type Request={command:'inspect';schema_version:1;operation:PreflightOperation}|{command:'install';schema_version:1;operation:PreflightOperation;check:CheckId};
/** A refused or failed preflight call: a C-PD1 code for a native `JobError`, else `transport` or `schema`. */
export class PreflightFailure extends Data.TaggedError('PreflightFailure')<{readonly code:FailureCode|'transport'|'schema'}> {}
export class PreflightBridge extends Context.Service<PreflightBridge,{invoke:(request:Request)=>Effect.Effect<Report,PreflightFailure>}>()('launcher/PreflightBridge') {}
// The guarded operation's screen names a decode failure (no `preflight` UI area exists).
const area:Record<PreflightOperation,UiArea>={play:'launch',install:'install',repair:'repair'};
const refusalCode=(error:unknown):FailureCode|'transport'=>typeof error==='string'&&Object.hasOwn(jobErrorFailure,error)?jobErrorFailure[error]!:'transport';
export const preflightBridgeLayer=(invoke:(request:Request)=>Promise<unknown>)=>Layer.succeed(PreflightBridge,{
  invoke:(request)=>Effect.tryPromise({try:()=>invoke(request),catch:error=>new PreflightFailure({code:refusalCode(error)})}).pipe(
    Effect.flatMap(value=>decodeReport(value).pipe(Effect.mapError(()=>{reportUiEvent(area[request.operation],'preflight_decode_failed','schema');return new PreflightFailure({code:'schema'});}))),
  )});
export const makePreflightWorkflow=Effect.gen(function*(){
 const bridge=yield* PreflightBridge;
 // Inspect probes Rosetta (5 s) and may fetch the release manifest; an install waits on an
 // administrator prompt and a download, so it has no timeout.
 const inspectPreflight=(operation:PreflightOperation)=>bridge.invoke({command:'inspect',schema_version:1,operation}).pipe(
  Effect.timeout('30 seconds'),Effect.catchTag('TimeoutError',()=>Effect.fail(new PreflightFailure({code:'transport'}))));
 const installDependency=(operation:PreflightOperation,check:CheckId)=>bridge.invoke({command:'install',schema_version:1,operation,check});
 return {inspectPreflight,installDependency};
});
export type PreflightWorkflow=Effect.Success<typeof makePreflightWorkflow>;
/** The workflow bound to the native `preflight_command`. */
export const preflightWorkflow=(invoke:(command:string,args?:Record<string,unknown>)=>Promise<unknown>):PreflightWorkflow=>
 Effect.runSync(makePreflightWorkflow.pipe(Effect.provide(preflightBridgeLayer(request=>invoke('preflight_command',{request})))));
