import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parseHTML} from 'linkedom';
import {makePanelContext} from './panel-context';
import {mountLogUpload,logUploadUnavailable} from './log-upload-view';
const html=readFileSync('ui/index.html','utf8');

// Scaffold (LX-02): LX-05 replaces this with the panel's real behaviour.
test('the log-upload panel renders its heading and says it is unavailable, without calling native code',async()=>{
 const {document}=parseHTML(html);const calls:string[]=[];
 const app=mountLogUpload(document as unknown as Document,async command=>{calls.push(command);return null;},makePanelContext().context);
 await app.ready;
 assert.equal(document.getElementById('log-upload-title')!.textContent,'Debug logs');
 assert.equal(document.getElementById('log-upload-status')!.textContent,logUploadUnavailable);
 assert.deepEqual(calls,[]);
 await app.dispose();assert.equal(document.getElementById('log-upload-title'),null);
});
