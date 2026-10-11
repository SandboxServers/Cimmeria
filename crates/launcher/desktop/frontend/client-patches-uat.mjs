// Logic UAT, scaffold (LX-02): the real Settings page with the client-patches panel
// mounted. LX-04 replaces this with its feature's native-backed UAT.
import {readFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
import {parseHTML} from 'linkedom';
import {mountClientPatches,clientPatchesUnavailable} from './.test-build/client-patches-view.mjs';
const {document}=parseHTML(await readFile(new URL('./ui/index.html',import.meta.url),'utf8'));
const calls=[];
const app=mountClientPatches(document,async command=>{calls.push(command);return null;},{snapshot:()=>undefined,onSnapshot:()=>()=>{},refresh:async()=>{}});
try {
 await app.ready;
 const panel=document.getElementById('client-patches-panel');
 assert.ok(panel.closest('#gear'),'the panel sits in Settings');
 assert.equal(document.getElementById('client-patches-title').textContent,'Client patches');
 assert.equal(document.getElementById('client-patches-status').textContent,clientPatchesUnavailable);
 await app.refresh();assert.deepEqual(calls,[]);
 console.log('Client patches scaffold UAT passed: the Settings panel mounts with its heading and unavailable line, and sends no native command.');
 console.log('Excluded: everything LX-04 adds; the packaged window and visual layout.');
}finally{await app.dispose();}
