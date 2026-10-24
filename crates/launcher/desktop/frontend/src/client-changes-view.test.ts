import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parseHTML} from 'linkedom';
import {makePanelContext} from './panel-context';
import type {NativeSnapshot} from './contract';
import {mountClientChanges,clientChangesUnavailable,clientChangesNothingInstalled,clientChangesRecordUnreadable} from './client-changes-view';
import {mountDesktop} from './composition';
const html=readFileSync('ui/index.html','utf8');
const snapshot={} as NativeSnapshot;

const row=(group:string,heading:string,title:string,status:string,status_label:string,patch_id:string|null=null)=>
 ({group,heading,title,description:`${title} description`,status,status_label,patch_id});
const installedStatus=(patchStatus='applied',patchLabel='applied')=>({schema_version:1,installed:true,changes:[
 row('setup','Launcher setup','Login servers','every_launch','before every launch'),
 row('patch','Patched files','Dialog portraits',patchStatus,patchLabel,'008-dialog-portraits'),
 row('launch','Added when the game starts','Client patches DLL','on_launch','on'),
 row('launch','Added when the game starts','Telemetry','off','off'),
 row('server','Sent by the server while you play','Cooked game data','during_play','during play'),
],install_record_unreadable:false});

function mount(answers:unknown[]) {
 const {document}=parseHTML(html);const calls:unknown[]=[];const feed=makePanelContext();feed.publish(snapshot);
 const app=mountClientChanges(document as unknown as Document,async(command,args)=>{
  calls.push([command,args]);const answer=answers.length>1?answers.shift():answers[0];
  if(answer instanceof Error)throw answer.message;return answer;
 },feed.context);
 const text=(id:string)=>document.getElementById(id)?.textContent;
 return {document,calls,feed,app,text};
}

test('renders native rows grouped under their headings, with status and patch id',async()=>{
 const {document,calls,app,text}=mount([installedStatus()]);
 await app.ready;
 assert.deepEqual(calls,[['client_changes_command',{request:{command:'inspect',schema_version:1}}]]);
 assert.match(text('client-changes-status')!,/^5 changes/);
 assert.equal(document.getElementById('client-changes-panel')!.closest('main')?.id,'play');
 assert.equal(document.getElementById('client-changes-details')!.hasAttribute('open'),false);
 const headings=Array.from(document.querySelectorAll('#client-changes-list h4')).map(h=>h.textContent);
 assert.deepEqual(headings,['Launcher setup','Patched files','Added when the game starts','Sent by the server while you play']);
 const launch=document.querySelector('#client-changes-list ul[data-group="launch"]')!;
 assert.deepEqual(Array.from(launch.querySelectorAll('li strong')).map(s=>s.textContent),['Client patches DLL','Telemetry']);
 const patch=document.querySelector('li[data-patch-id="008-dialog-portraits"]')!;
 assert.equal(patch.querySelector('strong')!.textContent,'Dialog portraits');
 assert.equal(patch.querySelector('.client-change-status')!.textContent,' (applied)');
 assert.equal(patch.querySelector('p')!.textContent,'Dialog portraits description');
 await app.dispose();assert.equal(document.getElementById('client-changes-title'),null);
});

test('with nothing installed it explains why the list is empty',async()=>{
 const {document,app,text}=mount([{schema_version:1,installed:false,changes:[],install_record_unreadable:false}]);
 await app.ready;
 assert.equal(text('client-changes-status'),clientChangesNothingInstalled);
 assert.equal(document.querySelectorAll('#client-changes-list li').length,0);
 await app.dispose();
});

test('native failures and malformed answers each show a line, never a stale list',async()=>{
 for(const [answer,line] of [
  [new Error('busy'),/busy with another operation/],
  [new Error('unavailable'),new RegExp(clientChangesUnavailable.replace(/\./g,'\\.'))],
  [new Error('corrupt'),/\(corrupt\)/],
  [{schema_version:1,installed:true,changes:[{title:'no other fields'}]},/\(transport\)/],
  [{...installedStatus(),extra:true},/\(transport\)/],
 ] as const){
  const {document,app,text}=mount([installedStatus(),answer]);
  await app.ready;assert.equal(document.querySelectorAll('#client-changes-list li').length,5);
  await app.refresh();
  assert.match(text('client-changes-status')!,line);
  assert.equal(document.querySelectorAll('#client-changes-list li').length,0);
  await app.dispose();
 }
});

test('each snapshot re-reads the list; after dispose snapshots are ignored',async()=>{
 const {document,calls,feed,app}=mount([installedStatus('pending','applied on the next Install / Update'),installedStatus()]);
 await app.ready;
 const status=()=>document.querySelector('li[data-patch-id="008-dialog-portraits"]')!.getAttribute('data-status');
 assert.equal(status(),'pending');
 feed.publish(snapshot);await app.settled();
 assert.equal(calls.length,2);assert.equal(status(),'applied');
 await app.dispose();feed.publish(snapshot);await app.settled();
 assert.equal(calls.length,2);
});

test('waits for the launcher\'s first snapshot instead of racing its inspect',async()=>{
 const {document}=parseHTML(html);const feed=makePanelContext();const calls:string[]=[];
 const app=mountClientChanges(document as unknown as Document,async command=>{calls.push(command);return installedStatus();},feed.context);
 await app.settled();await new Promise(resolve=>setImmediate(resolve));
 assert.deepEqual(calls,[]);assert.match(document.getElementById('client-changes-status')!.textContent!,/^Reading/);
 feed.publish(snapshot);await app.ready;
 assert.deepEqual(calls,['client_changes_command']);
 await app.dispose();
});

test('dispose does not wait for a native read that never answers',async()=>{
 const {document}=parseHTML(html);const feed=makePanelContext();feed.publish(snapshot);
 const app=mountClientChanges(document as unknown as Document,()=>new Promise(()=>{}),feed.context);
 await app.dispose();await app.ready;
 assert.equal(document.getElementById('client-changes-panel')!.textContent,'');
});

test('a burst of snapshots during one read queues a single re-read',async()=>{
 const {document}=parseHTML(html);const feed=makePanelContext();feed.publish(snapshot);let calls=0;
 let release!:()=>void;const gate=new Promise<void>(resolve=>{release=resolve;});
 const app=mountClientChanges(document as unknown as Document,async()=>{calls++;if(calls===1)await gate;return installedStatus();},feed.context);
 while(calls===0)await Promise.resolve();
 feed.publish(snapshot);feed.publish(snapshot);feed.publish(snapshot);
 release();await app.ready;await app.settled();
 assert.equal(calls,2);
 await app.dispose();
});

test('an unreadable install record shows a line saying Repair may fix it',async()=>{
 const {document,app,text}=mount([{...installedStatus('pending','applied on the next Install / Update'),install_record_unreadable:true},installedStatus()]);
 await app.ready;
 const record=document.getElementById('client-changes-record')!;
 assert.equal(record.hidden,false);assert.equal(text('client-changes-record'),clientChangesRecordUnreadable);
 assert.equal(document.querySelectorAll('#client-changes-list li').length,5,'the list still renders');
 await app.refresh();assert.equal(record.hidden,true);
 await app.dispose();
});

test('opening Settings re-reads, even when no snapshot ever arrived',async()=>{
 const {document,window}=parseHTML(html);const feed=makePanelContext();const calls:string[]=[];
 const app=mountClientChanges(document as unknown as Document,async command=>{calls.push(command);return installedStatus();},feed.context);
 const gear=document.getElementById('gear')!;
 // The gear's own handler (view.ts) toggles it in the same click; stand in for it.
 const open=()=>{gear.hidden=!gear.hidden;document.getElementById('settings')!.dispatchEvent(new window.Event('click'));};
 open();await app.ready;
 assert.deepEqual(calls,['client_changes_command']);assert.match(document.getElementById('client-changes-status')!.textContent!,/^5 changes/);
 open();await app.settled();await new Promise(resolve=>setImmediate(resolve));
 assert.equal(calls.length,1,'closing Settings reads nothing');
 open();await app.settled();await new Promise(resolve=>setImmediate(resolve));
 assert.equal(calls.length,2);
 await app.dispose();open();await new Promise(resolve=>setImmediate(resolve));
 assert.equal(calls.length,2,'dispose removes the listener');
});

// Composition: a game-telemetry save and a finished install publish a launcher
// snapshot, so the list re-reads without Settings being reopened.
const nativeSnapshot=(revision=0):NativeSnapshot=>({schema_version:1,operation:{schema_version:1,revision,operation:null},
 preferences:{schema_version:2,revision:0,install_directory:null,launcher_summary_consent:false,login_servers:[],client_patches_enabled:true,display:{mode:'windowed' as const,width:1024,height:768}},requires_reopen:false,adopted_login_servers:null,display_modes:['windowed' as const],launcher:{version:'0.1.0',tag:null}});
const until=async(done:()=>boolean,what:()=>string)=>{for(let i=0;i<500&&!done();i++)await new Promise(resolve=>setImmediate(resolve));assert.ok(done(),what());};

test('a game-telemetry save re-reads the list through a launcher snapshot',async()=>{
 const {document,window}=parseHTML(html);let reads=0;let telemetry={schema_version:1,available:true,opted_in:false,last_outcome:null};
 const desktop=mountDesktop(document as unknown as Document,async(command,args:any)=>{
  if(command==='launcher_command')return nativeSnapshot();
  if(command==='client_changes_command'){reads++;return installedStatus();}
  if(command==='game_telemetry_command'){if(args.request.command==='set')telemetry={...telemetry,opted_in:args.request.opted_in};return telemetry;}
  throw 'unavailable';
 });
 try{
  await until(()=>reads>0,()=>'never read');
  const box=document.getElementById('game-telemetry') as unknown as HTMLInputElement;
  await until(()=>!box.disabled,()=>'telemetry box never enabled');
  // Startup itself re-inspects a few times; count from quiet.
  for(let i=0;i<50;i++)await new Promise(resolve=>setImmediate(resolve));
  await desktop.app.settled();const before=reads;
  box.checked=true;box.dispatchEvent(new window.Event('change'));
  await until(()=>reads>before,()=>`after save: ${reads} of ${before}, ${document.getElementById('game-telemetry-status')!.textContent}`);
 }finally{await desktop.dispose();}
});
