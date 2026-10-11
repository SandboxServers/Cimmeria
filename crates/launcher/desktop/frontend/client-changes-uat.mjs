// Logic UAT, scaffold (LX-02): the real Settings page with the client-changes panel
// mounted. LX-07 replaces this with its feature's native-backed UAT.
import {readFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
import {parseHTML} from 'linkedom';
import {mountClientChanges,clientChangesUnavailable} from './.test-build/client-changes-view.mjs';
const {document}=parseHTML(await readFile(new URL('./ui/index.html',import.meta.url),'utf8'));
const calls=[];
const app=mountClientChanges(document,async command=>{calls.push(command);return null;},{snapshot:()=>undefined,onSnapshot:()=>()=>{},refresh:async()=>{}});
try {
 await app.ready;
 const panel=document.getElementById('client-changes-panel');
 assert.ok(panel.closest('#gear'),'the panel sits in Settings');
 assert.equal(document.getElementById('client-changes-title').textContent,'Changes to your client');
 assert.equal(document.getElementById('client-changes-status').textContent,clientChangesUnavailable);
 await app.refresh();assert.deepEqual(calls,[]);
 console.log('Changes to your client scaffold UAT passed: the Settings panel mounts with its heading and unavailable line, and sends no native command.');
 console.log('Excluded: everything LX-07 adds; the packaged window and visual layout.');
}finally{await app.dispose();}
