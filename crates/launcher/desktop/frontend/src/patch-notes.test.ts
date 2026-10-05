import test from 'node:test';
import assert from 'node:assert/strict';
import {parseHTML} from 'linkedom';
import {mountPatchNotes} from './patch-notes';
const fixture = {schema_version:1, patches:[{id:'one',title:'<img src=x onerror=alert(1)>',description:'<script>evil()</script>'}]};
function setup(invoke:()=>Promise<unknown>) {
  const {document,window}=parseHTML('<div id="notes-status"></div><button id="notes-refresh"></button><div id="entries"></div>');
  const app=mountPatchNotes(document as unknown as Document,invoke);
  return {document,app,refresh:()=>document.getElementById('notes-refresh')!.dispatchEvent(new window.Event('click'))};
}
test('signed notes render as text; refresh failure retains explicitly stale notes', async () => {
  let calls=0;
  const ui=setup(async()=>{calls++; if(calls>1) throw 'signature'; return fixture;});
  try {
    ui.app.open(); ui.app.open(); await ui.app.settled();
    assert.equal(calls,1);
    assert.equal(ui.document.querySelectorAll('script,img').length,0);
    assert.match(ui.document.getElementById('entries')!.textContent!, /<img/);
    ui.refresh(); await ui.app.settled();
    assert.match(ui.document.getElementById('notes-status')!.textContent!,/previously verified/);
    assert.equal(ui.document.querySelectorAll('article').length,1);
  } finally {await ui.app.dispose();}
});
test('unsupported response is rejected; disposal interrupts observation without replay', async () => {
  const ui=setup(async()=>({schema_version:2,patches:[]}));
  ui.app.open(); await ui.app.settled();
  assert.match(ui.document.getElementById('notes-status')!.textContent!, /unsupported/);
  assert.equal(ui.document.querySelectorAll('article').length,0);
  await ui.app.dispose();
  let calls=0;
  const pending=setup(()=>{calls++;return new Promise(()=>{});});
  pending.app.open(); await new Promise(resolve=>setImmediate(resolve));
  await pending.app.dispose(); pending.refresh();
  assert.equal(calls,1);
});
