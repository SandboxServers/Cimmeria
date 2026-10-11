import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parseHTML} from 'linkedom';
import type {NativeSnapshot} from './contract';
import {makePanelContext} from './panel-context';
import {mountServerList,parseServerList,serverListText} from './server-list-view';
const html=readFileSync('ui/index.html','utf8');
const snapshot=(revision:number,login_servers:NativeSnapshot['preferences']['login_servers']=[]):NativeSnapshot=>({
 schema_version:1,operation:{schema_version:1,revision:0,operation:null},requires_reopen:false,
 preferences:{schema_version:2,revision,install_directory:null,launcher_summary_consent:false,login_servers,client_patches_enabled:false},
});
function mounted(respond:(request:any)=>Promise<unknown>=async()=>null) {
 const {document,window}=parseHTML(html);const calls:{command:string;request:any}[]=[];
 const feed=makePanelContext(async()=>{});feed.publish(snapshot(3,[{name:'Old',url:'http://old.example:8081'}]));
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
 const m=mounted();
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
