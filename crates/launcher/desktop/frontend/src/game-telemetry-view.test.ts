import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parseHTML} from 'linkedom';
import {gameTelemetryText,mountGameTelemetry} from './game-telemetry-view';
const html=readFileSync('ui/index.html','utf8');
type Outcome='attached'|'session_unavailable'|'endpoint_refused'|'session_not_written'|null;
const initial=(available=true)=>({schema_version:1,available,opted_in:false,last_outcome:null as Outcome});
const settle=()=>new Promise(resolve=>setImmediate(resolve));
const page=()=>{const {document,window}=parseHTML(html);const box=document.getElementById('game-telemetry') as HTMLInputElement;
 return {document,box,text:()=>document.getElementById('game-telemetry-status')!.textContent!,recheck:document.getElementById('inspect-game-telemetry') as HTMLButtonElement,
  toggle:(on:boolean)=>{box.checked=on;box.dispatchEvent(new window.Event('change'));},click:(id:string)=>document.getElementById(id)!.dispatchEvent(new window.Event('click'))};};

test('off by default; one press gives immediate feedback, one save, and never touches setup diagnostics',async()=>{
 let status=initial();const commands:string[]=[];const sets:boolean[]=[];let finish!:()=>void;const wait=new Promise<void>(resolve=>{finish=resolve;});
 const ui=page();const app=mountGameTelemetry(ui.document,async(command,args:any)=>{commands.push(command);const r=args.request;
  if(r.command==='set'){sets.push(r.opted_in);await wait;status={...status,opted_in:r.opted_in};}return status;});
 try{await app.ready;await settle();
  assert.equal(ui.box.checked,false);assert.equal(ui.box.disabled,false);assert.match(ui.text(),/^Off\. Play loads no diagnostics module/);assert.equal(ui.recheck.hidden,true);
  ui.toggle(true);ui.toggle(true);
  // First press: the box, its busy state and the status line all answer before the native reply.
  assert.equal(ui.box.checked,true);assert.equal(ui.box.getAttribute('aria-busy'),'true');assert.equal(ui.box.disabled,true);assert.match(ui.text(),/Turning game diagnostics on/);
  finish();await app.settled();await settle();
  assert.deepEqual(sets,[true]);assert.equal(ui.box.checked,true);assert.equal(ui.box.disabled,false);assert.equal(ui.box.getAttribute('aria-busy'),'false');
  assert.match(ui.text(),/^On\. The next Play loads the diagnostics module/);
  ui.toggle(false);await app.settled();await settle();assert.deepEqual(sets,[true,false]);assert.match(ui.text(),/^Off\./);
  assert.deepEqual([...new Set(commands)],['game_telemetry_command']);
 }finally{await app.dispose();}
});

test('a build without the module cannot be opted in, and says why',async()=>{
 let status=initial(false);let sets=0;const ui=page();
 const app=mountGameTelemetry(ui.document,async(_,args:any)=>{if(args.request.command==='set'){sets++;throw 'platform_unavailable';}return status;});
 try{await app.ready;await settle();assert.equal(ui.box.disabled,true);assert.match(ui.text(),/Not available in this build/);
  ui.toggle(true);await app.settled();await settle();assert.equal(sets,0);assert.equal(ui.box.checked,false);
  // A choice saved by a build that had the module can still be turned off here.
  status={...status,opted_in:true};await app.refresh();await settle();assert.equal(ui.box.disabled,false);assert.match(ui.text(),/does not include the module\. Nothing is sent/);
 }finally{await app.dispose();}
});

test('a lost reply is never replayed: the choice stays unconfirmed until rechecked',async()=>{
 let status=initial();let sets=0;let lose=true;const ui=page();
 const app=mountGameTelemetry(ui.document,async(_,args:any)=>{if(args.request.command==='set'){sets++;status={...status,opted_in:args.request.opted_in};if(lose)throw 'transport';}return status;});
 try{await app.ready;await settle();ui.toggle(true);await app.settled();await settle();
  assert.equal(sets,1);assert.match(ui.text(),/could not be confirmed.*nothing was retried/);assert.equal(ui.box.disabled,true);assert.equal(ui.recheck.hidden,false);
  // Pressing again while unconfirmed sends nothing.
  ui.toggle(true);await app.settled();await settle();assert.equal(sets,1);
  lose=false;ui.click('inspect-game-telemetry');await app.settled();await settle();
  // The recheck shows what native storage holds: the save had landed.
  assert.equal(sets,1);assert.equal(ui.box.checked,true);assert.equal(ui.box.disabled,false);assert.equal(ui.recheck.hidden,true);assert.match(ui.text(),/^On\./);
 }finally{await app.dispose();}
});

test('the status line reports what the last Play did, without claiming an upload',()=>{
 const on=(last_outcome:Outcome)=>gameTelemetryText({status:{schema_version:1,available:true,opted_in:true,last_outcome},pending:false,uncertain:false,error:null});
 assert.match(on('attached'),/loaded the diagnostics module with a server session/);assert.doesNotMatch(on('attached'),/upload|received|sent/i);
 assert.match(on('session_unavailable'),/server gave no session.*started without diagnostics/);
 assert.match(on('endpoint_refused'),/refused the server address/);
 assert.match(on('session_not_written'),/could not be saved beside the game/);
 assert.match(gameTelemetryText({status:null,pending:false,uncertain:true,error:'platform_unavailable'}),/does not include the game diagnostics module/);
});
