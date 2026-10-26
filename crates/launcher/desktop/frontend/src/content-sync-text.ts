import type {ContentSync,ContentSyncFailure,LaunchState} from './launch-workflow';
// The English of the game data check (LP-24). The shell sends codes only; every word here is the frontend's.
const megabytes=(bytes:number)=>(bytes/1_000_000).toFixed(1);
// What an io_kind reads as in words; an absent or unknown kind reads as "an error".
const ioWords:Record<string,string>={sharing_violation:'the file is in use',lock_violation:'the file is locked',storage_full:'the disk is full',
  permission_denied:'access was denied',not_found:'it was not found',connection_refused:'the connection was refused',connection_reset:'the connection was reset',
  timed_out:'it timed out',host_unreachable:"the server can't be reached from this network",network_unreachable:"the server can't be reached from this network"};
const ioText=(kind:string|null)=>kind!==null&&Object.hasOwn(ioWords,kind)?ioWords[kind]!:'an error';
// The step as the first half of "Couldn't {action}:".
function actionText(f:ContentSyncFailure,server:string,pak:string):string {
 switch(f.step){
  case 'prepare':return 'get ready to check game data';
  case 'index':return `get the game data list from ${server}`;
  case 'plan':return `compare your game data with ${server}`;
  case 'download':return `download ${pak} from ${server}`;
  case 'stage':return `build the updated ${pak}`;
  case 'commit':return `save the updated ${pak}`;
  default:return 'update game data';
 }
}
// Why the check failed, then the remedy; a reason this copy does not know gets the generic line.
function reasonText(f:ContentSyncFailure,server:string,pak:string):string {
 const io=ioText(f.io_kind);
 switch(f.reason){
  case 'profile_unavailable':return "Windows didn't give the launcher your Documents folder. Check that Documents exists, then press Retry.";
  case 'no_login_server':return 'No login server is set. Add one under Login servers.';
  case 'endpoint_refused':return "The login server address isn't one the launcher may download from. Check it under Login servers.";
  case 'unsafe_path':return 'Your game data folder is a link to another place, so the launcher won\'t write into it. Replace the link with a normal folder or use Reset client cache, then press Retry.';
  case 'connect_failed':return `The server didn't accept the connection (${io}). Check that the server is online and your network works, then press Retry.`;
  case 'timeout':return 'The server stopped answering. Press Retry.';
  case 'catalog_unavailable':return `The server is still preparing its game data.${f.retry_after_s===null?'':` It asked to wait ${f.retry_after_s} s.`} Press Retry in a minute.`;
  case 'catalog_failed':return "The server couldn't prepare its game data. Tell the server's operator; Retry won't help until they fix it.";
  case 'rate_limited':return 'The server is limiting requests from your network. Press Retry in a minute.';
  case 'http_error':return `The server answered with an error${f.http_status===null?'':` (HTTP ${f.http_status})`}. Press Retry. If it keeps happening, tell the server's operator.`;
  case 'too_large':case 'bad_index':case 'bad_body':return "The server's answer was damaged or not understood. Press Retry. If it keeps happening, tell the server's operator.";
  case 'unknown_category':case 'version_changed':return "The server's game data changed during the update. Press Retry.";
  case 'source_newer':return `Your game install has newer ${pak} data (version ${f.source_version??'unknown'}) than the server (version ${f.served_version??'unknown'}). Tell the server's operator, or use Repair if you changed the game files yourself.`;
  case 'source_missing':return `Your game install is missing ${pak}. Use Repair, then press Retry.`;
  case 'source_unreadable':return `The launcher couldn't read ${pak} from your game install: ${io}. Close Stargate Worlds, then press Retry, or use Repair.`;
  case 'base_missing':case 'digest_mismatch':return `Your game install's ${pak} doesn't match what the server's data builds on, so nothing was saved. Use Repair, then press Retry.`;
  case 'pak_in_use':return `${pak} is open in another program: ${io}. Close Stargate Worlds and any tool that has the file open, then press Retry.`;
  case 'write_failed':return `Windows refused to save ${pak}: ${io}. Free disk space or check the folder's permissions, then press Retry.`;
  case 'deadline':return `It took longer than ${f.deadline_s===null?'the allowed time':`${f.deadline_s} seconds`}. Press Retry; a slow connection may need another try.`;
  default:return `An unexpected error (${f.reason}). Press Retry. If it keeps happening, use Upload debug logs.`;
 }
}
// The failure text: the action and why in one line, then the details the shell sent, each present field named.
function failureText(c:ContentSync):string {
 const blank:ContentSyncFailure={step:'unknown',reason:c.reason??'unknown',category_id:null,http_status:null,os_code:null,retry_after_s:null,
  source_version:null,served_version:null,deadline_s:null,pak:null,io_kind:null};
 const f=c.failure??blank;
 const server=c.server??'the server';
 const pak=f.pak??'a game data file';
 let details=`Details: ${f.step} / ${f.reason}`;
 if(f.category_id!==null)details+=`, category ${f.category_id}`;
 if(f.http_status!==null)details+=`, HTTP ${f.http_status}`;
 if(f.os_code!==null)details+=`, Windows error ${f.os_code}`;
 return `Couldn't ${actionText(f,server,pak)}: ${reasonText(f,server,pak)}\n${details}`;
}
/** The line under Play that says what the game data check is doing or did, or null when it says nothing. */
export function contentSyncText(s:LaunchState):string|null {
 const c=s.status?.content_sync;
 if(!c)return null;
 const server=c.server??'the server';
 switch(c.phase){
  case 'checking':return `Checking game data with ${server}…`;
  case 'downloading':case 'writing':return `Updating game data: ${c.categories_done} of ${c.categories_total} categories (${megabytes(c.bytes_done)} of ${megabytes(c.bytes_total)} MB)…`;
  case 'committing':return 'Saving updated game data…';
  default:break;
 }
 // The check is done: its success line lives only while the game is starting, and a skip says why Character select is slow.
 const phase=s.status?.observation?.phase;
 const starting=phase==='preparing'||phase==='host_started'||phase==='process_started';
 switch(c.outcome){
  case 'up_to_date':return starting?'Game data is up to date.':null;
  case 'synced':return starting?`Game data updated (${c.categories_total} categories).`:null;
  case 'skipped':
   if(c.reason==='disabled')return 'The server turned off the game data check. Character select may take about a minute.';
   if(c.reason==='server_unsupported')return "This server doesn't offer game data updates. Character select may take about a minute.";
   return null;
  case 'failed':return failureText(c);
  default:return null;
 }
}
