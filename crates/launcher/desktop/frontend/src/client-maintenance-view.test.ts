import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parseHTML} from 'linkedom';
import {mountClientMaintenance,clientMaintenanceUnavailable} from './client-maintenance-view';
const html=readFileSync('ui/index.html','utf8');

// Scaffold (LX-02): LX-06 replaces this with the panel's real behaviour.
test('the client-maintenance panel renders its heading and says it is unavailable, without calling native code',async()=>{
 const {document}=parseHTML(html);const calls:string[]=[];
 const app=mountClientMaintenance(document as unknown as Document,async command=>{calls.push(command);return null;});
 await app.ready;
 assert.equal(document.getElementById('client-maintenance-title')!.textContent,'Client cache and settings');
 assert.equal(document.getElementById('client-maintenance-status')!.textContent,clientMaintenanceUnavailable);
 assert.deepEqual(calls,[]);
 await app.dispose();assert.equal(document.getElementById('client-maintenance-title'),null);
});
