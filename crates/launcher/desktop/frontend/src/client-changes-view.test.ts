import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parseHTML} from 'linkedom';
import {mountClientChanges,clientChangesUnavailable} from './client-changes-view';
const html=readFileSync('ui/index.html','utf8');

// Scaffold (LX-02): LX-07 replaces this with the panel's real behaviour.
test('the client-changes panel renders its heading and says it is unavailable, without calling native code',async()=>{
 const {document}=parseHTML(html);const calls:string[]=[];
 const app=mountClientChanges(document as unknown as Document,async command=>{calls.push(command);return null;});
 await app.ready;
 assert.equal(document.getElementById('client-changes-title')!.textContent,'Changes to your client');
 assert.equal(document.getElementById('client-changes-status')!.textContent,clientChangesUnavailable);
 assert.deepEqual(calls,[]);
 await app.dispose();assert.equal(document.getElementById('client-changes-title'),null);
});
