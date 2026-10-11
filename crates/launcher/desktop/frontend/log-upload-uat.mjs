// Logic UAT, scaffold (LX-02): the real Settings page with the log-upload panel
// mounted. LX-05 replaces this with its feature's native-backed UAT.
import {readFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
import {parseHTML} from 'linkedom';
import {mountLogUpload,logUploadUnavailable} from './.test-build/log-upload-view.mjs';
const {document}=parseHTML(await readFile(new URL('./ui/index.html',import.meta.url),'utf8'));
const calls=[];
const app=mountLogUpload(document,async command=>{calls.push(command);return null;},{snapshot:()=>undefined,onSnapshot:()=>()=>{},refresh:async()=>{}});
try {
 await app.ready;
 const panel=document.getElementById('log-upload-panel');
 assert.ok(panel.closest('#gear'),'the panel sits in Settings');
 assert.equal(document.getElementById('log-upload-title').textContent,'Debug logs');
 assert.equal(document.getElementById('log-upload-status').textContent,logUploadUnavailable);
 await app.refresh();assert.deepEqual(calls,[]);
 console.log('Debug logs scaffold UAT passed: the Settings panel mounts with its heading and unavailable line, and sends no native command.');
 console.log('Excluded: everything LX-05 adds; the packaged window and visual layout.');
}finally{await app.dispose();}
