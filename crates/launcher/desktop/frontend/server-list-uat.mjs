// Logic UAT, scaffold (LX-02): the real Settings page with the server-list panel
// mounted. LX-03 replaces this with its feature's native-backed UAT.
import {readFile} from 'node:fs/promises';
import assert from 'node:assert/strict';
import {parseHTML} from 'linkedom';
import {mountServerList,serverListUnavailable} from './.test-build/server-list-view.mjs';
const {document}=parseHTML(await readFile(new URL('./ui/index.html',import.meta.url),'utf8'));
const calls=[];
const app=mountServerList(document,async command=>{calls.push(command);return null;},{snapshot:()=>undefined,onSnapshot:()=>()=>{},refresh:async()=>{}});
try {
 await app.ready;
 const panel=document.getElementById('server-list-panel');
 assert.ok(panel.closest('#gear'),'the panel sits in Settings');
 assert.equal(document.getElementById('server-list-title').textContent,'Login servers');
 assert.equal(document.getElementById('server-list-status').textContent,serverListUnavailable);
 await app.refresh();assert.deepEqual(calls,[]);
 console.log('Login servers scaffold UAT passed: the Settings panel mounts with its heading and unavailable line, and sends no native command.');
 console.log('Excluded: everything LX-03 adds; the packaged window and visual layout.');
}finally{await app.dispose();}
