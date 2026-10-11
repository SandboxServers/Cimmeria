// Logic UAT, scaffold (LX-02): the real Settings page with the client-maintenance panel
// mounted. LX-06 replaces this with its feature's native-backed UAT.
import {readFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
import {parseHTML} from 'linkedom';
import {mountClientMaintenance,clientMaintenanceUnavailable} from './.test-build/client-maintenance-view.mjs';
const {document}=parseHTML(await readFile(new URL('./ui/index.html',import.meta.url),'utf8'));
const calls=[];
const app=mountClientMaintenance(document,async command=>{calls.push(command);return null;});
try {
 await app.ready;
 const panel=document.getElementById('client-maintenance-panel');
 assert.ok(panel.closest('#gear'),'the panel sits in Settings');
 assert.equal(document.getElementById('client-maintenance-title').textContent,'Client cache and settings');
 assert.equal(document.getElementById('client-maintenance-status').textContent,clientMaintenanceUnavailable);
 await app.refresh();assert.deepEqual(calls,[]);
 console.log('Client cache and settings scaffold UAT passed: the Settings panel mounts with its heading and unavailable line, and sends no native command.');
 console.log('Excluded: everything LX-06 adds; the packaged window and visual layout.');
}finally{await app.dispose();}
