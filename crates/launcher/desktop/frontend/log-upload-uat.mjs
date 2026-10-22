// Logic UAT: the real Settings page with the production log-upload panel, driven
// as a player would. The native host is a scripted stand-in that answers with the
// engine's states (LF-15: line ingest); the engine's own tests cover the reads,
// the paced posts and the offsets.
import {readFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
import {parseHTML} from 'linkedom';
import {mountLogUpload} from './.test-build/log-upload-view.mjs';
const html=await readFile(new URL('./ui/index.html',import.meta.url),'utf8');
const context={snapshot:()=>undefined,onSnapshot:()=>()=>{},refresh:async()=>{}};
function page(native){
 const {document,window}=parseHTML(html);const calls=[];let release=()=>{};
 const invoke=async(command,{request})=>{assert.equal(command,'log_upload_command');assert.deepEqual(Object.keys(request).sort(),['command','schema_version']);calls.push(request.command);
  if(request.command==='upload')await new Promise(resolve=>{release=resolve;});return native(request.command);};
 const app=mountLogUpload(document,invoke,context);const button=()=>document.getElementById('log-upload-button');
 return {document,app,calls,release:()=>release(),button,press:()=>button().dispatchEvent(new window.Event('click')),text:()=>document.getElementById('log-upload-status').textContent};
}
const tick=()=>new Promise(resolve=>setImmediate(resolve));
// The engine's offsets: the same lines a second time are not sent.
const SENT={lines_sent:4,lines_total:4,upload_id:'0123456789abcdef'};
let sent=0;const ledger=command=>command==='inspect'?{schema_version:1,state:'ready'}:sent++===0?{schema_version:1,state:'sent',...SENT}:{schema_version:1,state:'nothing_new'};
let ui=page(ledger);
try{
 await ui.app.ready;
 assert.ok(ui.document.getElementById('log-upload-panel').closest('#gear'),'the panel sits in Settings');
 assert.equal(ui.button().hidden,false);
 // First press: busy at once, before native work finishes.
 ui.press();await tick();assert.equal(ui.text(),'Uploading…');assert.equal(ui.button().textContent,'Uploading…');assert.equal(ui.button().disabled,true);
 ui.press();ui.release();await ui.app.settled();assert.match(ui.text(),/Sent 4 of 4 log lines\. Quote 0123456789abcdef in your report\./);
 ui.press();await tick();ui.release();await ui.app.settled();assert.equal(ui.text(),'Nothing new to upload');
 assert.deepEqual(ui.calls,['inspect','upload','upload']);
 await ui.app.dispose();
 // No store compiled in: the button is gone.
 ui=page(()=>({schema_version:1,state:'disabled'}));await ui.app.ready;
 assert.equal(ui.button().hidden,true);await ui.app.dispose();
 // A second upload while one runs is refused with busy.
 ui=page(command=>{if(command==='upload')throw 'busy';return {schema_version:1,state:'ready'};});await ui.app.ready;
 ui.press();await tick();ui.release();await ui.app.settled();
 assert.match(ui.text(),/Another log upload is still running/);assert.equal(ui.button().disabled,false);
 await ui.app.dispose();
 // A server pause sends part of the lines and says when to press again; the line
 // stays through a ready re-inspect, and an uninstall seen later replaces it.
 let installed=true;
 ui=page(command=>command==='upload'?{schema_version:1,state:'partial',lines_sent:2,lines_total:4,upload_id:'0123456789abcdef',retry_after_secs:60}:{schema_version:1,state:installed?'ready':'not_installed'});await ui.app.ready;
 ui.press();await tick();ui.release();await ui.app.settled();
 assert.match(ui.text(),/Sent 2 of 4 log lines/);assert.match(ui.text(),/Press again in 60 seconds/);
 await ui.app.refresh();assert.match(ui.text(),/Sent 2 of 4 log lines/);
 installed=false;await ui.app.refresh();assert.match(ui.text(),/Install the game first/);assert.equal(ui.button().disabled,true);
 await ui.app.dispose();
 // No log server for this install: the line says so and the button stays usable.
 ui=page(()=>({schema_version:1,state:'unreachable'}));await ui.app.ready;
 assert.match(ui.text(),/could not be reached/);assert.equal(ui.button().hidden,false);assert.equal(ui.button().disabled,false);
 console.log('Debug logs UAT passed: first press shows Uploading� then the sent line count and upload id; a repeat says Nothing new to upload; no store hides the button; a busy refusal says another upload is running; a partial send says when to press again and survives a ready re-inspect; an unreachable server keeps the button usable.');
 console.log('Excluded: the real native host and network (engine tests cover the reads, paced posts and offsets); the packaged window and visual layout.');
}finally{await ui.app.dispose();}
