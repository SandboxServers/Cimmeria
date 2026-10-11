import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parseHTML} from 'linkedom';
import {mountClientPatches,clientPatchesUnavailable} from './client-patches-view';
const html=readFileSync('ui/index.html','utf8');

// Scaffold (LX-02): LX-04 replaces this with the panel's real behaviour.
test('the client-patches panel renders its heading and says it is unavailable, without calling native code',async()=>{
 const {document}=parseHTML(html);const calls:string[]=[];
 const app=mountClientPatches(document as unknown as Document,async command=>{calls.push(command);return null;});
 await app.ready;
 assert.equal(document.getElementById('client-patches-title')!.textContent,'Client patches');
 assert.equal(document.getElementById('client-patches-status')!.textContent,clientPatchesUnavailable);
 assert.deepEqual(calls,[]);
 await app.dispose();assert.equal(document.getElementById('client-patches-title'),null);
});
