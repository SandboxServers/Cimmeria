import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { parseHTML } from 'linkedom';
import { mountLauncher } from './view';
import { Command, NativeSnapshot } from './contract';

const html = await readFile(new URL('../ui/index.html', import.meta.url), 'utf8');
const initial = (): NativeSnapshot => ({schema_version:1, operation:{schema_version:1, revision:0, operation:null},
  preferences:{schema_version:1, revision:0, install_directory:null, launcher_summary_consent:false}, requires_reopen:false});
const flush = () => new Promise<void>(resolve => setImmediate(resolve));

function dom() {
  const {document, window} = parseHTML(html);
  const get = <T extends HTMLElement = HTMLElement>(id:string) => document.getElementById(id)! as unknown as T;
  const click = (id:string) => get(id).dispatchEvent(new window.Event('click'));
  return {document: document as unknown as Document, get, click, window};
}

test('approved tabs and settings work without inventing installed state', {timeout:5000}, async () => {
  const ui = dom();
  const calls:string[]=[];
  const app = mountLauncher(ui.document, async command => { calls.push(command); return initial(); });
  try {
    await app.ready; await flush();
    assert.equal(ui.get('play').closest('[hidden]'), null, 'Play must not be nested inside hidden settings');
    assert.equal(ui.get('adoption-title').closest('section')!.parentElement, ui.get('gear'), 'adoption and legacy import are sibling settings sections');
    assert.equal(ui.get<HTMLInputElement>('telemetry').checked, false);
    assert.equal(ui.get<HTMLInputElement>('telemetry').disabled, false);
    assert.equal(ui.get<HTMLButtonElement>('install').disabled, true);
    assert.equal(ui.get<HTMLButtonElement>('repair').disabled, true);
    assert.equal(ui.get<HTMLButtonElement>('uninstall').disabled, true);
    ui.click('notes'); assert.equal(ui.get('play').hidden, true); assert.equal(ui.get('patches').hidden, false);
    assert.equal(ui.get('patches').closest('[hidden]'), null, 'Patch Notes must not inherit hidden settings');
    ui.click('settings'); assert.equal(ui.get('gear').hidden, false);
    assert.equal(ui.get('play').hidden, true);
    assert.equal(ui.get('patches').hidden, true, 'settings occupies the content panel');
    ui.click('home'); assert.equal(ui.get('play').hidden, false);
    assert.equal(ui.get('gear').hidden, true);
    assert.equal(ui.get('settings').getAttribute('aria-expanded'), 'false');
    await app.settled();
    assert.deepEqual(calls, ['launcher_command', 'fetch_patch_notes']);
  } finally { await app.dispose(); }
  ui.click('settings'); assert.equal(ui.get('gear').hidden, true, 'dispose removes handlers');
});

test('consent waits for native acknowledgement and rolls back on failed save', {timeout:5000}, async () => {
  const ui = dom(); let saves=0;
  let rejectSave!: (reason:unknown) => void;
  const app = mountLauncher(ui.document, async (_command,args) => {
    const request = args!.request as Command;
    if(request.command === 'inspect') return initial();
    saves++;
    return new Promise((_resolve,reject) => {rejectSave=reject;});
  });
  try {
    await app.ready; await flush();
    const checkbox=ui.get<HTMLInputElement>('telemetry');
    checkbox.checked=true; checkbox.dispatchEvent(new ui.window.Event('change'));
    await flush();
    assert.equal(checkbox.checked,true); assert.equal(checkbox.disabled,true);
    assert.match(ui.get('status').textContent!, /Saving/);
    rejectSave('io'); await app.settled(); await flush();
    assert.equal(saves,1); assert.equal(checkbox.checked,false);
    assert.equal(checkbox.disabled,true, 'inspection is required after failed write');
    assert.equal(ui.get('retry').hidden,false);
    assert.match(ui.get('status').textContent!, /could not read or save/);
    ui.click('retry'); await app.settled(); await flush();
    assert.equal(checkbox.disabled,false);
  } finally {await app.dispose();}
});

test('native folder choice saves once, preserves consent, and cancellation changes nothing', {timeout:5000}, async () => {
  const ui=dom(); let state=initial(); let folder:string|null='/selected/game'; let saves=0; let opens=0;
  const app=mountLauncher(ui.document, async (command,args) => {
    if(command === 'choose_install_directory') return folder;
    if(command === 'show_install_directory') { opens++; return null; }
    const request=args!.request as Command;
    if(request.command === 'save_preferences') {
      saves++;
      assert.equal(request.expected_revision,state.preferences.revision);
      state={...state,preferences:{...state.preferences, revision:state.preferences.revision+1,
        install_directory:request.install_directory, launcher_summary_consent:request.launcher_summary_consent}};
    }
    return state;
  });
  try {
    await app.ready; await flush();
    ui.click('choose'); await app.settled(); await flush();
    assert.equal(saves,1); assert.equal(ui.get<HTMLInputElement>('install-path').value,'/selected/game');
    assert.equal(state.preferences.launcher_summary_consent,false);
    ui.click('folder'); await app.settled(); assert.equal(opens,1);
    folder=null;
    ui.click('choose'); await app.settled(); await flush();
    assert.equal(saves,1); assert.equal(ui.get<HTMLInputElement>('install-path').value,'/selected/game');
    assert.match(ui.get('status').textContent!,/cancelled/);
  } finally { await app.dispose(); }
});

test('disposing during pending IPC removes controls without claiming a saved result', {timeout:5000}, async () => {
  const ui = dom(); let calls=0;
  const app = mountLauncher(ui.document, async () => { calls++; return new Promise(() => {}); });
  await flush();
  assert.equal(calls,1);
  await app.dispose();
  ui.click('settings');
  assert.equal(ui.get('gear').hidden,true);
  assert.equal(ui.get<HTMLInputElement>('telemetry').disabled,true);
  assert.equal(calls,1);
});

test('native Launch ownership gates directory changes until observed exit',async()=>{
 const ui=dom();let status:NativeSnapshot={...initial(),operation:{schema_version:1,revision:1,operation:{id:'launch',kind:'launch',state:'running',intent_digest:Array(32).fill(0)}}};
 const app=mountLauncher(ui.document,async()=>status);
 try{await app.ready;await flush();assert.equal(ui.get<HTMLButtonElement>('choose').disabled,true);
 status={...status,operation:{...status.operation,revision:2,operation:{...status.operation.operation!,state:'reconciliation_required'}}};await app.refresh();await flush();assert.equal(ui.get<HTMLButtonElement>('choose').disabled,true);
 status={...status,operation:{...status.operation,revision:3,operation:{...status.operation.operation!,state:'succeeded'}}};await app.refresh();await flush();assert.equal(ui.get<HTMLButtonElement>('choose').disabled,false);
 }finally{await app.dispose();}
});
