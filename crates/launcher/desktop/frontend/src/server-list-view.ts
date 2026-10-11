import type {Invoke} from './view';
import type {PanelContext} from './panel-context';
import {scaffoldFailure,type LoginServer} from './contract';
/**
 * Settings panel (LX-03): the login servers `LoginInternal.lua` offers, one `Name = URL` per line,
 * saved through `launcher_command` (`save_client_options`). The parser mirrors
 * `cimmeria_launcher_core::client_setup::login_servers::parse` rule for rule, with one difference:
 * a box with no entries is the empty list, which means the default server, where the core parser
 * refuses it. The engine checks the list again on save and again at Play.
 */
export type ServerListParse={servers:LoginServer[]}|{error:string};
// Rust `str::trim` trims Unicode White_Space; JS `trim` also strips U+FEFF and keeps U+0085.
const space='[\\t\\n\\v\\f\\r \\u0085\\u00a0\\u1680\\u2000-\\u200a\\u2028\\u2029\\u202f\\u205f\\u3000]+';
const edges=new RegExp(`^${space}|${space}$`,'g');
const trim=(s:string)=>s.replace(edges,'');
// The field goes into a Lua string literal: printable ASCII or space, no quote or backslash.
const validField=(s:string)=>s.length>0&&s.length<=200&&/^[\x20-\x7e]*$/.test(s)&&!/["\\]/.test(s);
function check(server:LoginServer):string|null {
 if(!validField(server.name)||!validField(server.url))return 'names and URLs must be plain ASCII without quotes or backslashes';
 if(!(server.url.startsWith('http://')||server.url.startsWith('https://')))return 'the URL must start with http:// or https://';
 return null;
}
export function parseServerList(text:string):ServerListParse {
 const servers:LoginServer[]=[];const lines=text.split('\n');
 for(let n=0;n<lines.length;n++){
  const line=trim(lines[n]!);
  if(line===''||line.startsWith('#'))continue;
  const at=line.indexOf('=');
  if(at<0)return {error:`line ${n+1}: expected \`Name = URL\``};
  const server={name:trim(line.slice(0,at)),url:trim(line.slice(at+1))};
  const problem=check(server);if(problem)return {error:`line ${n+1}: ${problem}`};
  servers.push(server);
 }
 return {servers};
}
/** The box text for `servers`, like the core crate's `to_text`. */
export const serverListText=(servers:readonly LoginServer[])=>servers.map(s=>`${s.name} = ${s.url}`).join('\n');
export const serverListHint='One server per line, as Name = http://host:port. Leave it empty to use the default Cimmeria server.';
const saveErrorText:Record<string,string>={
 stale_revision:'Settings changed while saving, so nothing was saved. Press Save again to save this list.',
 busy:'The launcher is installing, repairing or updating. Save again when it finishes.',
 invalid_server_list:'The launcher refused this list. Each line must be Name = http://host:port.',
 persistence_uncertain:'The save could not be confirmed. Restart the launcher to check the saved list.',
 in_use:'Another launcher is using these settings. Close it, then save again.',
 io:'The list could not be saved. Check permissions and free disk space.',
};
export function mountServerList(document:Document,invoke:Invoke,context:PanelContext) {
 const panel=document.getElementById('server-list-panel');
 if(!panel)throw new Error('Missing launcher element server-list-panel');
 const make=<K extends keyof HTMLElementTagNameMap>(tag:K,id:string)=>{const e=document.createElement(tag);e.id=id;return e;};
 const heading=make('h3','server-list-title');heading.textContent='Login servers';
 const label=make('label','server-list-label');label.setAttribute('for','server-list-text');label.textContent=serverListHint;
 const box=make('textarea','server-list-text');box.rows=4;box.spellcheck=false;box.setAttribute('aria-describedby','server-list-error');
 const error=make('p','server-list-error');error.setAttribute('role','alert');error.hidden=true;
 const save=make('button','save-server-list');save.textContent='Save servers';
 const status=make('p','server-list-status');status.setAttribute('role','status');status.setAttribute('aria-live','polite');
 // An adopted copy plays with the list its adoption reviewed (LX-03), so the box would not change what it logs in to.
 const kept=make('p','server-list-adopted');kept.hidden=true;
 panel.textContent='';panel.append(heading,kept,label,box,error,save,status);
 let dirty=false;let saving=false;let disposed=false;let mutation:Promise<void>=Promise.resolve();
 const show=(problem:string|null)=>{error.textContent=problem??'';error.hidden=problem===null;box.setAttribute('aria-invalid',String(problem!==null));};
 const adopted=()=>context.snapshot()?.adopted_login_servers??null;
 // A saved list fills the box until the player edits it; their text is never overwritten. Text that already
 // means the saved list is left alone too, so a save keeps the player's comments and spacing.
 const sync=()=>{
  const snapshot=context.snapshot();if(disposed||!snapshot)return;
  const list=snapshot.adopted_login_servers;kept.hidden=list===null;
  kept.textContent=list===null?'':`This copy was adopted, so Play keeps the servers it was adopted with: ${list.length?list.map(s=>`${s.name} = ${s.url}`).join('; '):'the default server'}. Saving the list below does not change this copy.`;
  if(dirty||saving)return;
  const text=serverListText(snapshot.preferences.login_servers);const current=parseServerList(box.value);
  if(!('servers' in current)||serverListText(current.servers)!==text)box.value=text;
 };
 const unsubscribe=context.onSnapshot(()=>sync());
 const onInput=()=>{dirty=true;show(null);};
 const onSave=()=>{
  if(disposed||saving)return;
  const parsed=parseServerList(box.value);
  if('error' in parsed){show(parsed.error);status.textContent='Not saved. Fix the line shown above.';return;}
  show(null);
  const snapshot=context.snapshot();
  if(!snapshot){status.textContent='Settings are still loading. Try again in a moment.';return;}
  saving=true;save.disabled=true;box.readOnly=true;save.setAttribute('aria-busy','true');status.textContent='Saving login servers…';
  const request={command:'save_client_options',schema_version:1,expected_revision:snapshot.preferences.revision,
   login_servers:parsed.servers,client_patches_enabled:snapshot.preferences.client_patches_enabled};
  mutation=invoke('launcher_command',{request}).then(async()=>{
   // Until the reload lands the box keeps the player's text: the old snapshot must not overwrite what was saved.
   const refreshed=await context.refresh().then(()=>true,()=>false);
   if(disposed)return;
   // A refresh that brought no snapshot newer than the one this save was based on did not reload it.
   const reloaded=refreshed&&(context.snapshot()?.preferences.revision??-1)>request.expected_revision;
   if(!reloaded){dirty=true;status.textContent='Saved, but the launcher could not reload settings. Restart the launcher to see the saved list.';return;}
   dirty=false;
   status.textContent=adopted()!==null?'Saved, but this copy keeps the servers it was adopted with.'
    :parsed.servers.length?'Saved. The next Play uses this list.':'Saved. The next Play uses the default server.';
  },async failure=>{
   const code=scaffoldFailure(failure);
   if(code==='stale_revision')await context.refresh().catch(()=>{});
   if(!disposed)status.textContent=saveErrorText[code]??'The list could not be saved. Recheck status, then try again.';
  }).finally(()=>{saving=false;if(disposed)return;save.disabled=false;box.readOnly=false;save.setAttribute('aria-busy','false');sync();});
 };
 box.addEventListener('input',onInput);save.addEventListener('click',onSave);
 sync();
 const ready=Promise.resolve();
 return {ready,refresh:()=>{sync();return mutation;},settled:()=>mutation,dispose:async()=>{
  disposed=true;unsubscribe();box.removeEventListener('input',onInput);save.removeEventListener('click',onSave);await mutation;panel.textContent='';}};
}
