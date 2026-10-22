import {Effect} from 'effect';
import type {Invoke} from './view';
import type {PanelContext} from './panel-context';
import {reportUiEvent} from './ui-event';
import {decodeLogUploadStatus,logUploadFailure,type LogUploadError,type LogUploadStatus,type NativeSnapshot} from './contract';
/** Settings panel for `log_upload_command`. Native code picks the files and the store; the view sends only the command. */
export const logUploadText:Record<LogUploadStatus['state'],string>={
 not_installed:'Install the game first: there are no game logs to upload yet.',
 ready:'Ready to send debug logs.',
 nothing_new:'No new log lines since the last upload.',
 no_logs:'No logs to upload yet: the game has not written any.',
 sent:'Sent',
 partial:'Part of the logs was sent. Press again to send the rest.',
 unreachable:"Debug logs can't be sent: this install has no telemetry server.",
};
/** The line shown for a status: `sent` and `partial` say how many lines went and the id to quote in a report. */
export function logUploadMessage(status:LogUploadStatus):string {
 if(status.state==='sent')return `Sent ${status.lines_sent} log lines. Reference ${status.upload_id}.`;
 if(status.state==='partial')return `Sent ${status.lines_sent} of ${status.lines_total} lines. The server asked us to wait; press again in about ${Math.max(1,Math.ceil(status.retry_after_secs/60))} min.`;
 return logUploadText[status.state];
}
export const logUploadUploading='Sending debug logs… this can take a few minutes.';
/** Under the button: what is sent. Game logs hold more personal text, and pressing is the consent (D-LU1). */
export const logUploadNote="Sends the launcher's and the game's recent log lines (file paths and addresses removed; game logs can include chat) to the project's diagnostics server. Pressing it sends logs even when Send game diagnostics is off.";
export const logUploadErrorText:Record<LogUploadError|'transport'|'schema',string>={
 busy:'Another log upload is still running. Wait for it to finish, then try again.',
 network:'The upload did not reach the log store. Check your connection and try again.',
 collect_failed:'The game\'s logs could not be read. Close the game and try again.',
 in_use:'The launcher state is in use. Try again in a moment.',
 io:'The launcher could not read its state. Try again.',
 corrupt:'The launcher state is damaged, so nothing was uploaded.',
 unsupported_schema:'This launcher build does not understand that request.',
 too_large:'The launcher state is too large to read.',
 unsafe_file:'The game folder is not safe to read, so nothing was uploaded.',
 invalid_directory:'The game folder is not valid, so nothing was uploaded.',
 stale_revision:'The launcher settings changed. Try again.',
 persistence_uncertain:'Restart the launcher before uploading logs.',
 invalid_server_list:'The saved login servers are not valid. Check them under Login servers.',
 transport:'The launcher did not answer. Try again.',
 schema:'The launcher sent an answer this page does not understand.',
};
export function mountLogUpload(document:Document,invoke:Invoke,context:PanelContext) {
 const panel=document.getElementById('log-upload-panel');
 if(!panel)throw new Error('Missing launcher element log-upload-panel');
 const heading=document.createElement('h3');heading.id='log-upload-title';heading.textContent='Debug logs';
 const button=document.createElement('button');button.id='log-upload-button';button.type='button';button.textContent='Upload debug logs';button.hidden=true;button.disabled=true;
 const note=document.createElement('p');note.id='log-upload-note';note.textContent=logUploadNote;
 const status=document.createElement('p');status.id='log-upload-status';status.setAttribute('role','status');status.setAttribute('aria-live','polite');status.textContent='Checking debug logs…';
 panel.textContent='';panel.append(heading,button,note,status);
 let disposed=false;let uploading=false;let showingResult=false;let work:Promise<void>=Promise.resolve();
 type Reply={error:null;status:LogUploadStatus}|{error:keyof typeof logUploadErrorText};
 const call=async(command:'inspect'|'upload'):Promise<Reply>=>{let value:unknown;
  try{value=await invoke('log_upload_command',{request:{command,schema_version:1}});}catch(error){return {error:logUploadFailure(error)};}
  try{return {error:null,status:await Effect.runPromise(decodeLogUploadStatus(value))};}catch{reportUiEvent('log_upload','status_decode_failed','schema');return {error:'schema'};}};
 const show=(state:LogUploadStatus['state'])=>{
  // The button is always shown: a press is consent (D-LU1), and only a missing install or server disables it.
  button.hidden=false;button.disabled=state==='not_installed'||state==='unreachable';};
 const inspect=()=>{if(disposed||uploading)return work;
  work=call('inspect').then(reply=>{if(disposed||uploading)return;
   // A `ready` re-inspect keeps the last upload's result on screen; anything else
   // (uninstalled, busy, an error) is news and replaces it.
   if(reply.error===null&&reply.status.state==='ready'&&showingResult){show(reply.status.state);return;}
   showingResult=false;
   if(reply.error===null){show(reply.status.state);status.textContent=logUploadMessage(reply.status);}
   else{button.hidden=false;button.disabled=false;status.textContent=logUploadErrorText[reply.error];}});return work;};
 const upload=()=>{if(disposed||uploading||button.disabled)return work;
  // Feedback before the native reply: the press is visibly taken at once.
  uploading=true;showingResult=true;button.disabled=true;button.setAttribute('aria-busy','true');button.textContent=logUploadUploading;status.textContent=logUploadUploading;
  work=call('upload').then(reply=>{uploading=false;if(disposed)return;
   button.setAttribute('aria-busy','false');button.textContent='Upload debug logs';
   if(reply.error===null){show(reply.status.state);status.textContent=logUploadMessage(reply.status);}
   else{button.disabled=false;status.textContent=logUploadErrorText[reply.error];}});return work;};
 const click=()=>{void upload();};button.addEventListener('click',click);
 // A finished install or uninstall changes whether there is anything to upload,
 // so re-inspect when the journal's operation moves, not on every snapshot.
 const operationKey=(snapshot:NativeSnapshot)=>{const op=snapshot.operation.operation;return op?`${op.id}:${op.state}`:'';};
 const seen=context.snapshot();let lastKey=seen?operationKey(seen):undefined;
 const unsubscribe=context.onSnapshot(snapshot=>{const key=operationKey(snapshot);if(lastKey!==undefined&&key!==lastKey)void inspect();lastKey=key;});
 const ready=inspect();
 // Dispose does not wait for native work: a reply after it changes nothing.
 return {ready,refresh:inspect,upload,settled:()=>work,dispose:async()=>{disposed=true;unsubscribe();button.removeEventListener('click',click);panel.textContent='';}};
}
