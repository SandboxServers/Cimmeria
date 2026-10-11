import {Effect} from 'effect';
import type {Invoke} from './view';
import type {PanelContext} from './panel-context';
import {decodeLogUploadStatus,logUploadFailure,type LogUploadError,type LogUploadStatus,type NativeSnapshot} from './contract';
/** Settings panel for `log_upload_command`. Native code picks the files and the store; the view sends only the command. */
export const logUploadText:Record<LogUploadStatus['state'],string>={
 disabled:'',
 not_installed:'Install the game first: there are no game logs to upload yet.',
 ready:'Sends the game\'s debug and session logs to the developers, to help with a problem report.',
 uploaded:'Uploaded',
 nothing_new:'Nothing new to upload',
 no_logs:'No logs to upload yet: the game has not written any.',
};
export const logUploadUploading='Uploading…';
export const logUploadErrorText:Record<LogUploadError|'transport'|'schema',string>={
 busy:'Another launcher operation is still running or needs to be reconciled. Finish or reconcile it first, then upload again.',
 network:'The upload did not reach the log store. Check your connection and try again.',
 rejected:'The log store refused the upload. Try again later.',
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
 transport:'The launcher did not answer. Try again.',
 schema:'The launcher sent an answer this page does not understand.',
};
export function mountLogUpload(document:Document,invoke:Invoke,context:PanelContext) {
 const panel=document.getElementById('log-upload-panel');
 if(!panel)throw new Error('Missing launcher element log-upload-panel');
 const heading=document.createElement('h3');heading.id='log-upload-title';heading.textContent='Debug logs';
 const button=document.createElement('button');button.id='log-upload-button';button.type='button';button.textContent='Upload debug logs';button.hidden=true;button.disabled=true;
 const status=document.createElement('p');status.id='log-upload-status';status.setAttribute('role','status');status.setAttribute('aria-live','polite');status.textContent='Checking debug logs…';
 panel.textContent='';panel.append(heading,button,status);
 let disposed=false;let uploading=false;let showingResult=false;let work:Promise<void>=Promise.resolve();
 type Reply={error:null;state:LogUploadStatus['state']}|{error:keyof typeof logUploadErrorText};
 const call=async(command:'inspect'|'upload'):Promise<Reply>=>{let value:unknown;
  try{value=await invoke('log_upload_command',{request:{command,schema_version:1}});}catch(error){return {error:logUploadFailure(error)};}
  try{return {error:null,state:(await Effect.runPromise(decodeLogUploadStatus(value))).state};}catch{return {error:'schema'};}};
 const show=(state:LogUploadStatus['state'])=>{
  // A build with no store compiled in hides the button: there is nowhere to send logs.
  button.hidden=state==='disabled';button.disabled=state==='disabled'||state==='not_installed';};
 const inspect=()=>{if(disposed||uploading)return work;
  work=call('inspect').then(reply=>{if(disposed||uploading)return;
   if(reply.error===null){show(reply.state);if(!showingResult)status.textContent=logUploadText[reply.state];}
   else{button.hidden=false;button.disabled=false;if(!showingResult)status.textContent=logUploadErrorText[reply.error];}});return work;};
 const upload=()=>{if(disposed||uploading||button.disabled)return work;
  // Feedback before the native reply: the press is visibly taken at once.
  uploading=true;showingResult=true;button.disabled=true;button.setAttribute('aria-busy','true');button.textContent=logUploadUploading;status.textContent=logUploadUploading;
  work=call('upload').then(reply=>{uploading=false;if(disposed)return;
   button.setAttribute('aria-busy','false');button.textContent='Upload debug logs';
   if(reply.error===null){show(reply.state);status.textContent=logUploadText[reply.state];}
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
