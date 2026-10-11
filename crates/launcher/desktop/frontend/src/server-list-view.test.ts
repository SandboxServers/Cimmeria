import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parseHTML} from 'linkedom';
import {mountServerList,serverListUnavailable} from './server-list-view';
const html=readFileSync('ui/index.html','utf8');

// Scaffold (LX-02): LX-03 replaces this with the panel's real behaviour.
test('the server-list panel renders its heading and says it is unavailable, without calling native code',async()=>{
 const {document}=parseHTML(html);const calls:string[]=[];
 const app=mountServerList(document as unknown as Document,async command=>{calls.push(command);return null;});
 await app.ready;
 assert.equal(document.getElementById('server-list-title')!.textContent,'Login servers');
 assert.equal(document.getElementById('server-list-status')!.textContent,serverListUnavailable);
 assert.deepEqual(calls,[]);
 await app.dispose();assert.equal(document.getElementById('server-list-title'),null);
});
