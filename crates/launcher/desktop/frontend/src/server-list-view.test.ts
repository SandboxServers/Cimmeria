import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parseHTML} from 'linkedom';
import type {NativeSnapshot} from './contract';
import {makePanelContext} from './panel-context';
import {mountServerList,parseServerList,serverListText} from './server-list-view';
const html=readFileSync('ui/index.html','utf8');
const snapshot=(revision:number,login_servers:NativeSnapshot['preferences']['login_servers']=[]):NativeSnapshot=>({
 schema_version:1,operation:{schema_version:1,revision:0,operation:null},requires_reopen:false,adopted_login_servers:null,display_modes:['windowed' as const],launcher:{version:'0.1.0',tag:null},
 preferences:{schema_version:2,revision,install_directory:null,launcher_summary_consent:false,login_servers,client_patches_enabled:false,display:{mode:'windowed' as const,width:1024,height:768}},
});
function mounted(respond:(request:any)=>Promise<unknown>=async()=>null,refresh:()=>Promise<unknown>=async()=>{},
 first:NativeSnapshot=snapshot(3,[{name:'Old',url:'http://old.example:8081'}])) {
 const {document,window}=parseHTML(html);const calls:{command:string;request:any}[]=[];
 const feed=makePanelContext(()=>refresh());feed.publish(first);
 const app=mountServerList(document as unknown as Document,async(command,args)=>{const request=(args as {request:unknown}).request;calls.push({command,request});return respond(request);},feed.context);
 const box=document.getElementById('server-list-text') as unknown as HTMLTextAreaElement;
 const edit=(text:string)=>{box.value=text;box.dispatchEvent(new window.Event('input'));};
 const press=()=>(document.getElementById('save-server-list') as unknown as HTMLButtonElement).dispatchEvent(new window.Event('click'));
 const text=(id:string)=>document.getElementById(id)!.textContent;
 return {document,calls,feed,app,box,edit,press,text};
}

test('the parser follows the core crate: comments, blank lines, the first = and trimmed fields',()=>{
 assert.deepEqual(parseServerList('# mine\r\n\r\n  Home = http://home.example:8081  \r\nLab=https://lab.example/a=b\n'),
  {servers:[{name:'Home',url:'http://home.example:8081'},{name:'Lab',url:'https://lab.example/a=b'}]});
 assert.deepEqual(parseServerList('Home http://home.example'),{error:'line 1: expected `Name = URL`'});
 assert.deepEqual(parseServerList('\nHome = home.example:8081'),{error:'line 2: the URL must start with http:// or https://'});
 for(const bad of ['= http://x','Home =','Ho"me = http://x','Home = http://x\\y','Hôme = http://x','Ho\tme = http://x',`${'n'.repeat(201)} = http://x`])
  assert.deepEqual(parseServerList(bad),{error:'line 1: names and URLs must be plain ASCII without quotes or backslashes'},bad);
 // Rust `str::trim` edges: U+0085 and U+00A0 are trimmed, U+FEFF is not (so it is a non-ASCII name).
 assert.deepEqual(parseServerList('\u0085Home = http://home.example\u0085'),{servers:[{name:'Home',url:'http://home.example'}]});
 assert.deepEqual(parseServerList('﻿Home = http://home.example'),{error:'line 1: names and URLs must be plain ASCII without quotes or backslashes'});
 // 200 characters is the longest field the core crate accepts.
 assert.deepEqual(parseServerList(`${'n'.repeat(200)} = http://x`),{servers:[{name:'n'.repeat(200),url:'http://x'}]});
 // No entries means the defaults, where the core parser would refuse the text.
 assert.deepEqual(parseServerList(' \n# none\n'),{servers:[]});
 assert.equal(serverListText([{name:'A',url:'http://a'},{name:'B',url:'https://b'}]),'A = http://a\nB = https://b');
});

test('a bad line shows the error under the box and sends no command',async()=>{
 const m=mounted();
 assert.equal(m.box.value,'Old = http://old.example:8081');
 m.edit('Home = http://home.example:8081\nLab = lab.example:8081');m.press();await m.app.settled();
 assert.equal(m.text('server-list-error'),'line 2: the URL must start with http:// or https://');
 assert.equal(m.document.getElementById('server-list-error')!.hidden,false);
 assert.equal(m.text('server-list-status'),'Not saved. Fix the line shown above.');
 assert.deepEqual(m.calls,[]);
 // Editing clears the error; the player's text is kept over a later snapshot.
 m.edit('Home = http://home.example:8081');assert.equal(m.document.getElementById('server-list-error')!.hidden,true);
 m.feed.publish(snapshot(4));assert.equal(m.box.value,'Home = http://home.example:8081');
 await m.app.dispose();assert.equal(m.document.getElementById('server-list-title'),null);
});

test('Save sends the parsed list with the current revision, shows busy, then Saved',async()=>{
 let revision=3;
 const m=mounted(async request=>request,async()=>{m.feed.publish(snapshot(++revision,m.calls.at(-1)!.request.login_servers));});
 m.edit('Home = http://home.example:8081\n\nLab = https://lab.example');m.press();
 assert.equal(m.text('server-list-status'),'Saving login servers…');
 assert.equal((m.document.getElementById('save-server-list') as unknown as HTMLButtonElement).disabled,true);
 m.press();await m.app.settled();
 assert.equal(m.calls.length,1,'a second press while saving sends nothing');
 assert.equal(m.calls[0]!.command,'launcher_command');
 assert.deepEqual(m.calls[0]!.request,{command:'save_client_options',schema_version:1,expected_revision:3,
  login_servers:[{name:'Home',url:'http://home.example:8081'},{name:'Lab',url:'https://lab.example'}],client_patches_enabled:false});
 assert.equal(m.text('server-list-status'),'Saved. The next Play uses this list.');
 // An empty box saves the empty list, which means the default server.
 m.edit('');m.press();await m.app.settled();
 assert.deepEqual(m.calls[1]!.request.login_servers,[]);
 assert.equal(m.text('server-list-status'),'Saved. The next Play uses the default server.');
 await m.app.dispose();
});

test('a refused save says why and keeps the text',async()=>{
 const m=mounted(async()=>{throw 'invalid_server_list';});
 m.edit('Home = http://home.example');m.press();await m.app.settled();
 assert.equal(m.text('server-list-status'),'The launcher refused this list. Each line must be Name = http://host:port.');
 assert.equal(m.box.value,'Home = http://home.example');
 await m.app.dispose();
});

test('a stale save re-inspects, says so and keeps the text',async()=>{
 let refreshes=0;
 const m=mounted(async()=>{throw 'stale_revision';},async()=>{refreshes++;m.feed.publish(snapshot(5,[{name:'Other',url:'http://other.example'}]));});
 m.edit('Home = http://home.example');m.press();await m.app.settled();
 assert.equal(refreshes,1);
 assert.equal(m.text('server-list-status'),'Settings changed while saving, so nothing was saved. Press Save again to save this list.');
 assert.equal(m.box.value,'Home = http://home.example');
 // The next press carries the revision the refresh brought.
 m.press();await m.app.settled();assert.equal(m.calls[1]!.request.expected_revision,5);
 await m.app.dispose();
});

test('a refresh that fails after a stale save is reported to telemetry',async()=>{
 const uiCalls:unknown[]=[];
 Object.assign(globalThis,{window:{__TAURI__:{core:{invoke:async(command:string,args:unknown)=>{uiCalls.push([command,args]);}}}}});
 try{
  const m=mounted(async()=>{throw 'stale_revision';},async()=>{throw 'transport';});
  m.edit('Home = http://home.example:8081');m.press();await m.app.settled();
  assert.deepEqual(uiCalls,[['ui_event',{area:'server_list',event:'refresh_failed',code:'transport',level:'warn'}]]);
  await m.app.dispose();
 }finally{Reflect.deleteProperty(globalThis,'window');}
});

test('a save keeps the comments, and a failed reload keeps the text without claiming it reloaded',async()=>{
 let reload=true;
 const m=mounted(async request=>request,async()=>{if(!reload)throw 'transport';m.feed.publish(snapshot(4,[{name:'Home',url:'http://home.example'}]));});
 m.edit('# mine\nHome = http://home.example');m.press();await m.app.settled();
 assert.equal(m.text('server-list-status'),'Saved. The next Play uses this list.');
 assert.equal(m.box.value,'# mine\nHome = http://home.example','text that means the saved list is not rewritten');
 reload=false;m.edit('Lab = https://lab.example');m.press();await m.app.settled();
 assert.equal(m.text('server-list-status'),'Saved, but the launcher could not reload settings. Restart the launcher to see the saved list.');
 assert.equal(m.box.value,'Lab = https://lab.example');
 // Still the player's text: an old snapshot does not put the previous list back.
 m.feed.publish(snapshot(4,[{name:'Home',url:'http://home.example'}]));assert.equal(m.box.value,'Lab = https://lab.example');
 await m.app.dispose();
});

test('a refresh that brings no newer snapshot keeps the text and says it could not reload',async()=>{
 // The refresh resolves with the pre-save snapshot (revision 3) still current.
 const m=mounted(async()=>null,async()=>{m.feed.publish(snapshot(3,[{name:'Old',url:'http://old.example:8081'}]));});
 m.edit('Lab = https://lab.example');m.press();await m.app.settled();
 assert.equal(m.text('server-list-status'),'Saved, but the launcher could not reload settings. Restart the launcher to see the saved list.');
 assert.equal(m.box.value,'Lab = https://lab.example','the old snapshot does not put the previous list back');
 await m.app.dispose();
});

test('an adopted copy shows the servers it keeps and a save says Play does not use the new list',async()=>{
 const adopted=(revision:number)=>({...snapshot(revision),adopted_login_servers:[{name:'Reviewed',url:'http://reviewed.example:8081'}]});
 const m=mounted(async()=>null,async()=>{m.feed.publish(adopted(4));},adopted(3));
 const kept=m.document.getElementById('server-list-adopted')!;
 assert.equal(kept.hidden,false);
 assert.match(kept.textContent!,/keeps the servers it was adopted with: Reviewed = http:\/\/reviewed\.example:8081\./);
 m.edit('Home = http://home.example');m.press();await m.app.settled();
 assert.equal(m.text('server-list-status'),'Saved, but this copy keeps the servers it was adopted with.');
 await m.app.dispose();
});