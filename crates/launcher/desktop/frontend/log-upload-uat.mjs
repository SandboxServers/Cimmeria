// Logic UAT: the real Settings page with the production log-upload panel, driven
// as a player would. The native host is a scripted stand-in that answers with the
// engine's codes; the engine's own tests cover the zip, the ledger and the PUT.
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
// The engine's ledger: the same logs a second time are not sent.
let sent=0;const ledger=command=>command==='inspect'?{schema_version:1,state:'ready'}:{schema_version:1,state:sent++===0?'uploaded':'nothing_new'};
let ui=page(ledger);
try{
 await ui.app.ready;
 assert.ok(ui.document.getElementById('log-upload-panel').closest('#gear'),'the panel sits in Settings');
 assert.equal(ui.button().hidden,false);
 // First press: busy at once, before native work finishes.
 ui.press();await tick();assert.equal(ui.text(),'Uploading…');assert.equal(ui.button().textContent,'Uploading…');assert.equal(ui.button().disabled,true);
 ui.press();ui.release();await ui.app.settled();assert.equal(ui.text(),'Uploaded');
 ui.press();await tick();ui.release();await ui.app.settled();assert.equal(ui.text(),'Nothing new to upload');
 assert.deepEqual(ui.calls,['inspect','upload','upload']);
 await ui.app.dispose();
 // No store compiled in: the button is gone.
 ui=page(()=>({schema_version:1,state:'disabled'}));await ui.app.ready;
 assert.equal(ui.button().hidden,true);await ui.app.dispose();
 // A stuck operation refuses the upload with busy.
 ui=page(command=>{if(command==='upload')throw 'busy';return {schema_version:1,state:'ready'};});await ui.app.ready;
 ui.press();await tick();ui.release();await ui.app.settled();
 assert.match(ui.text(),/Finish or reconcile it first, then upload again\./);assert.equal(ui.button().disabled,false);
 await ui.app.dispose();
 // Over the 64 MiB cap the newest logs go and the line says older ones were left
 // out; an uninstall seen on a later re-inspect replaces that line.
 let installed=true;
 ui=page(command=>command==='upload'?{schema_version:1,state:'uploaded_newest'}:{schema_version:1,state:installed?'ready':'not_installed'});await ui.app.ready;
 ui.press();await tick();ui.release();await ui.app.settled();
 assert.match(ui.text(),/Older logs were left out to keep the upload under 64 MiB\./);
 await ui.app.refresh();assert.match(ui.text(),/Older logs were left out/);
 installed=false;await ui.app.refresh();assert.match(ui.text(),/Install the game first/);assert.equal(ui.button().disabled,true);
 console.log('Debug logs UAT passed: first press shows Uploading… then Uploaded; a repeat says Nothing new to upload; no store hides the button; a busy refusal says to finish or reconcile the running operation; an over-cap upload says older logs were left out, and a later uninstall replaces that line.');
 console.log('Excluded: the real native host and network (engine tests cover the zip, ledger and PUT); the packaged window and visual layout.');
}finally{await ui.app.dispose();}
