// Logic UAT (LX-07): the real Settings page with the "Changes to your client"
// panel mounted, driven through a player's journey by a scripted native side
// that answers `client_changes_command` the way the engine serializes it.
import {readFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
import {parseHTML} from 'linkedom';
import {mountClientChanges,clientChangesNothingInstalled,clientChangesRecordUnreadable} from './.test-build/client-changes-view.mjs';
const {document,window}=parseHTML(await readFile(new URL('./ui/index.html',import.meta.url),'utf8'));
const row=(group,heading,title,status,status_label,patch_id=null)=>({group,heading,title,description:`What ${title} changes.`,status,status_label,patch_id});
const installed=(telemetry,patchesDll='on_launch',unreadable=false)=>({schema_version:1,installed:true,install_record_unreadable:unreadable,changes:[
 row('setup','Launcher setup','Login servers','every_launch','before every launch'),
 row('setup','Launcher setup','Bundled data folder renamed','at_install','at install'),
 row('patch','Patched files','Dialog portraits','applied','applied','008-dialog-portraits'),
 row('patch','Patched files','999-new-patch','pending','applied on the next Install / Update','999-new-patch'),
 row('launch','Added when the game starts','Client patches DLL',patchesDll,patchesDll==='missing'?'missing from this build':'on'),
 row('launch','Added when the game starts','Telemetry',telemetry?'on_launch':'off',telemetry?'on':'off'),
 row('server','Sent by the server while you play','Cooked game data','during_play','during play'),
]});
// The native side: what the engine would answer at each step of the journey.
let native={ok:{schema_version:1,installed:false,changes:[],install_record_unreadable:false}};
const calls=[];
const invoke=async(command,args)=>{calls.push([command,args]);if(native.error)throw native.error;return native.ok;};
const listeners=new Set();
const context={snapshot:()=>undefined,onSnapshot:l=>{listeners.add(l);return()=>listeners.delete(l);},refresh:async()=>{}};
const publish=()=>{for(const l of [...listeners])l({});};
const app=mountClientChanges(document,invoke,context);
const status=()=>document.getElementById('client-changes-status').textContent;
const items=()=>[...document.querySelectorAll('#client-changes-list li')];
const rowStatus=title=>items().find(li=>li.querySelector('strong').textContent===title)?.dataset.status;
try {
 // The launcher's own inspect publishes the first snapshot; nothing is read before it.
 assert.equal(calls.length,0);publish();await app.ready;
 const panel=document.getElementById('client-changes-panel');
 assert.ok(panel.closest('#gear'),'the panel sits in Settings');
 assert.equal(document.getElementById('client-changes-title').textContent,'Changes to your client');
 // 1. A new player opens Settings before installing: the empty list says why.
 assert.equal(status(),clientChangesNothingInstalled);assert.equal(items().length,0);
 // 2. The install finishes; the next snapshot re-reads the list without a click.
 native={ok:installed(false)};publish();await app.settled();
 assert.match(status(),/^7 changes/);
 assert.deepEqual([...document.querySelectorAll('#client-changes-list h4')].map(h=>h.textContent),
  ['Launcher setup','Patched files','Added when the game starts','Sent by the server while you play']);
 assert.equal(document.querySelector('li[data-patch-id="999-new-patch"] strong').textContent,'999-new-patch','an unknown patch is listed by its id');
 assert.equal(rowStatus('Dialog portraits'),'applied');assert.equal(rowStatus('999-new-patch'),'pending');
 assert.equal(rowStatus('Telemetry'),'off');
 // 3. The player opts in to telemetry elsewhere in Settings; the row follows.
 native={ok:installed(true)};publish();await app.settled();
 assert.equal(rowStatus('Telemetry'),'on_launch');
 assert.equal(items().find(li=>li.dataset.status==='on_launch'&&li.textContent.includes('Telemetry')).querySelector('.client-change-status').textContent,' (on)');
 // 4. An Update holds the installation: the list is replaced by a busy line, then comes back.
 native={error:'busy'};publish();await app.settled();
 assert.match(status(),/busy with another operation/);assert.equal(items().length,0);
 native={ok:installed(true)};await app.refresh();
 assert.equal(items().length,7);
 // 5. Opening Settings re-reads with no snapshot: this build lacks the patches
 //    DLL, so the row says Play cannot start, and the patch record is damaged.
 const gear=document.getElementById('gear');gear.hidden=true;
 const openSettings=async()=>{gear.hidden=!gear.hidden;document.getElementById('settings').dispatchEvent(new window.Event('click'));
  await new Promise(resolve=>setImmediate(resolve));await app.settled();};
 native={ok:installed(true,'missing',true)};await openSettings();
 assert.equal(rowStatus('Client patches DLL'),'missing');
 assert.equal(document.getElementById('client-changes-record').hidden,false);
 assert.equal(document.getElementById('client-changes-record').textContent,clientChangesRecordUnreadable);
 // Closing Settings reads nothing.
 await openSettings();
 // 6. Every read was the one read-only inspect request.
 assert.ok(calls.every(([c,a])=>c==='client_changes_command'&&JSON.stringify(a)==='{"request":{"command":"inspect","schema_version":1}}'));
 assert.equal(calls.length,6);
 console.log('Changes to your client UAT passed: empty-install explanation, list appears after install, rows grouped with applied/pending patches and DLL status, telemetry row follows the opt-in, busy line during an Update then recovery, opening Settings re-reads (missing DLL row, unreadable patch record line), one read-only inspect per read.');
 console.log('Excluded: the native engine (covered by engine tests, including DLL rows matching Play), the packaged window and visual layout.');
}finally{await app.dispose();}
assert.equal(listeners.size,0,'dispose unsubscribes from snapshots');
