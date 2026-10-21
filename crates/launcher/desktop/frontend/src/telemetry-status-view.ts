import {Effect,Schema} from 'effect';
import type {Invoke} from './view';
import {reportUiEvent} from './ui-event';

/** The `telemetry_status` answer (C13). Decoded strictly, so a new field is a visible error. */
export const TelemetryStatus=Schema.Struct({
 last_outcome:Schema.NullOr(Schema.Literals(['sent','held','mint_failed','dropped'])),
 last_reason:Schema.NullOr(Schema.String),
 last_success_age_s:Schema.NullOr(Schema.Int),
 failures_since_success:Schema.Int,
});
export type TelemetryStatus=typeof TelemetryStatus.Type;

/** The status line refreshes this often, in milliseconds. */
export const REFRESH_MS=15_000;

/** The reasons the launcher gives for a send that did not go through (C13). */
const reasonTexts:Record<string,string>={
 transport:'the server could not be reached',
 timeout:'the server could not be reached',
 http_status:'the server refused the request',
 endpoint_refused:'the server refused the request',
 decode:'the server\'s answer was unreadable',
 dropped:'the server\'s limit was reached',
};

/** The status line for the checkbox state and the last telemetry status (C13). */
export function telemetryStatusText(status:TelemetryStatus,optedIn:boolean):string {
 if(!optedIn)return 'Diagnostics are off.';
 const outcome=status.last_outcome;
 if(outcome===null)return 'Diagnostics on. Nothing sent yet this run.';
 if(outcome==='sent')return `Diagnostics sent ${Math.floor((status.last_success_age_s??0)/60)} min ago.`;
 const known=outcome==='dropped'?reasonTexts.dropped:(status.last_reason===null?undefined:reasonTexts[status.last_reason]);
 const reason=known??'the launcher could not send them';
 return `Diagnostics not sent: ${reason}. The launcher will retry.`;
}

/**
 * Keeps the status line under `#game-telemetry` current: on mount, every 15 s, and
 * when the checkbox changes, so a toggle gets feedback on the first press. A failed
 * read leaves the last text in place.
 */
export function mountTelemetryStatus(document:Document,invoke:Invoke) {
 const line=document.getElementById('telemetry-status');
 const checkbox=document.getElementById('game-telemetry') as HTMLInputElement | null;
 let last:TelemetryStatus|null=null;let disposed=false;let failing=false;
 const render=()=>{if(line&&last&&!disposed)line.textContent=telemetryStatusText(last,checkbox?.checked??false);};
 const refresh=async()=>{
  let code='read';
  try{
   const value=await invoke('telemetry_status');
   code='schema';
   const status=await Effect.runPromise(Schema.decodeUnknownEffect(TelemetryStatus,{onExcessProperty:'error'})(value));
   failing=false;
   if(!disposed){last=status;render();}
  }catch{/* The line keeps its last text; the next refresh tries again. */if(!failing){failing=true;reportUiEvent('telemetry','status_read_failed',code,'warn');}}
 };
 const change=()=>{render();void refresh();};
 checkbox?.addEventListener('change',change);
 const timer=setInterval(()=>{void refresh();},REFRESH_MS);
 void refresh();
 return {refresh,dispose:()=>{disposed=true;clearInterval(timer);checkbox?.removeEventListener('change',change);}};
}
