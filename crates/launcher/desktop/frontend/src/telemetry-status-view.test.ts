import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parseHTML} from 'linkedom';
import {mountTelemetryStatus,telemetryStatusText,type TelemetryStatus} from './telemetry-status-view';

const html=readFileSync('ui/index.html','utf8');
const idle:TelemetryStatus={last_outcome:null,last_reason:null,last_success_age_s:null,failures_since_success:0};
const settle=()=>new Promise(resolve=>setTimeout(resolve,5));
const failed=(last_reason:string|null,last_outcome:TelemetryStatus['last_outcome']='mint_failed'):TelemetryStatus=>({...idle,last_outcome,last_reason,failures_since_success:1});
const tail=' The launcher will retry.';

test('each status text branch',()=>{
 assert.equal(telemetryStatusText(idle,false),'Diagnostics are off.');
 assert.equal(telemetryStatusText(idle,true),'Diagnostics on. Nothing sent yet this run.');
 assert.equal(telemetryStatusText({...idle,last_outcome:'sent',last_success_age_s:125},true),'Diagnostics sent 2 min ago.');
 assert.equal(telemetryStatusText(failed('timeout'),true),`Diagnostics not sent: the server could not be reached.${tail}`);
 assert.equal(telemetryStatusText(failed('transport'),true),`Diagnostics not sent: the server could not be reached.${tail}`);
 assert.equal(telemetryStatusText(failed('http_status'),true),`Diagnostics not sent: the server refused the request.${tail}`);
 assert.equal(telemetryStatusText(failed('endpoint_refused'),true),`Diagnostics not sent: the server refused the request.${tail}`);
 assert.equal(telemetryStatusText(failed('decode'),true),`Diagnostics not sent: the server's answer was unreadable.${tail}`);
 assert.equal(telemetryStatusText(failed(null,'dropped'),true),`Diagnostics not sent: the server's limit was reached.${tail}`);
 assert.equal(telemetryStatusText(failed('transport','dropped'),true),`Diagnostics not sent: the server's limit was reached.${tail}`);
 assert.equal(telemetryStatusText(failed('no_https_endpoint'),true),`Diagnostics not sent: the launcher could not send them.${tail}`);
 assert.equal(telemetryStatusText(failed(null,'held'),true),`Diagnostics not sent: the launcher could not send them.${tail}`);
 assert.equal(telemetryStatusText(failed('transport','held'),true),`Diagnostics not sent: the server could not be reached.${tail}`);
});

test('mount shows the status on mount and again at once when the checkbox changes',async()=>{
 const {document,window}=parseHTML(html);
 const box=document.getElementById('game-telemetry') as HTMLInputElement;
 const line=()=>document.getElementById('telemetry-status')!.textContent;
 const commands:string[]=[];let status:TelemetryStatus=idle;
 const app=mountTelemetryStatus(document,async command=>{commands.push(command);return status;});
 try{
  await settle();
  assert.deepEqual(commands,['telemetry_status']);
  assert.equal(line(),'Diagnostics are off.');
  status=failed('decode');box.checked=true;box.dispatchEvent(new window.Event('change'));
  assert.equal(line(),'Diagnostics on. Nothing sent yet this run.');
  await settle();
  assert.deepEqual(commands,['telemetry_status','telemetry_status']);
  assert.equal(line(),`Diagnostics not sent: the server's answer was unreadable.${tail}`);
 }finally{app.dispose();}
});

test('a checkbox set in code shows on the next refresh',async()=>{
 const {document}=parseHTML(html);
 const box=document.getElementById('game-telemetry') as HTMLInputElement;
 const line=()=>document.getElementById('telemetry-status')!.textContent;
 const app=mountTelemetryStatus(document,async()=>({...idle,last_outcome:'sent',last_success_age_s:0}));
 try{
  await settle();
  assert.equal(line(),'Diagnostics are off.');
  box.checked=true;
  await app.refresh();
  assert.equal(line(),'Diagnostics sent 0 min ago.');
 }finally{app.dispose();}
});

test('a failed read reports once per run of failures',async()=>{
 const calls:unknown[][]=[];
 (globalThis as {window?:unknown}).window={__TAURI__:{core:{invoke:async(cmd:string,args:unknown)=>{calls.push([cmd,args]);return undefined;}}}};
 const {document}=parseHTML(html);
 const app=mountTelemetryStatus(document,async()=>{throw new Error('down');});
 try{
  await settle();
  await app.refresh();
  assert.deepEqual(calls,[['ui_event',{area:'telemetry',event:'status_read_failed',code:'read',level:'warn'}]]);
 }finally{app.dispose();delete (globalThis as {window?:unknown}).window;}
});
