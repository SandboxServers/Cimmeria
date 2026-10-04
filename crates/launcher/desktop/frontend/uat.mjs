// Sequential JS logic UAT against the real Rust command handler and disk store.
// Run after building engine/examples/state_bridge through the repository lane.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, rm, readFile, writeFile } from 'node:fs/promises';
import { parseHTML } from 'linkedom';
import { mountLauncher } from './.test-build/view.mjs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { createInterface } from 'node:readline';
import { Effect } from 'effect';
import { bridgeLayer, makeLauncher } from './.test-build/workflows.mjs';

const binary = resolve(process.argv[2] ?? '');
assert.ok(process.argv[2], 'pass the native state_bridge executable');
const root = await mkdtemp(join(tmpdir(), 'cimmeria-logic-uat-'));
const children = [];
function start() {
  const child = spawn(binary, [root], { stdio: ['pipe', 'pipe', 'inherit'] });
  children.push(child);
  const pending = [];
  const lines = createInterface({ input: child.stdout });
  lines.on('line', line => {
    const task = pending.shift();
    if (!task) throw new Error('unexpected native reply');
    clearTimeout(task.timer);
    const reply = JSON.parse(line);
    if ('error' in reply) task.reject(reply.error); else task.resolve(reply.ok);
  });
  const fail = error => { for (const task of pending.splice(0)) { clearTimeout(task.timer); task.reject(error); } };
  child.on('error', fail);
  child.on('exit', () => fail(new Error('native helper exited')));
  return {
    invoke: request => new Promise((resolve, reject) => {
      const task = { resolve, reject, timer: setTimeout(() => reject(new Error('native reply timed out')), 3000) };
      pending.push(task);
      child.stdin.write(JSON.stringify(request) + '\n');
    }),
    close: async () => {
      const exited = once(child, 'exit'); child.stdin.end();
      const [code] = await exited; assert.equal(code, 0); lines.close();
    },
  };
}
try {
  const first = start();
  await Effect.runPromise(Effect.scoped(Effect.gen(function* () {
    const launcher = yield* makeLauncher;
    const initial = yield* launcher.inspect;
    assert.equal(initial.preferences.launcher_summary_consent, false);
    console.log('PASS first inspection: consent off, no operation');
    const saved = yield* launcher.savePreferences(join(root, 'Game'), true);
    assert.equal(saved.preferences.revision, 1);
    assert.equal(saved.preferences.launcher_summary_consent, true);
    assert.equal((yield* launcher.snapshot).busy, false);
    console.log('PASS Effect save: native preference acknowledged at revision 1');
  }).pipe(Effect.provide(bridgeLayer(first.invoke)))));
  await first.close();

  const second = start();
  await Effect.runPromise(Effect.scoped(Effect.gen(function* () {
    const launcher = yield* makeLauncher;
    const restored = yield* launcher.inspect;
    assert.equal(restored.preferences.install_directory, join(root, 'Game'));
    assert.equal(restored.preferences.launcher_summary_consent, true);
    assert.equal(restored.preferences.revision, 1);
    console.log('PASS process restart: install directory and consent persisted');
    yield* launcher.savePreferences(restored.preferences.install_directory, false);
    assert.equal((yield* launcher.snapshot).native.preferences.launcher_summary_consent, false);
    console.log('PASS opt-out: native acknowledgement and frontend agree');
  }).pipe(Effect.provide(bridgeLayer(second.invoke)))));
  await second.close();

  const third = start();
  const final = await third.invoke({command:'inspect', schema_version:1});
  assert.equal(final.preferences.launcher_summary_consent, false);
  assert.equal(final.preferences.revision, 2);
  console.log('PASS second restart: opt-out durable');
  const {document, window} = parseHTML(await readFile(new URL('./ui/index.html', import.meta.url), 'utf8'));
  const app = mountLauncher(document, (command, args) => {
    if (command === 'launcher_command') return third.invoke(args.request);
    if (command === 'fetch_patch_notes') return Promise.resolve({schema_version:1, patches:[{id:'fixture',title:'Fixture <patch>',description:'Verified-response fixture; no live network.'}]});
    if (command === 'choose_install_directory') return Promise.resolve(null);
    throw new Error('native GUI commands are not exercised by logic UAT');
  });
  const flush = () => new Promise(resolve => setImmediate(resolve));
  try {
    await app.ready; await flush();
    const consent = document.getElementById('telemetry');
    assert.equal(consent.checked, false);
    consent.checked = true; consent.dispatchEvent(new window.Event('change'));
    await app.settled(); await flush();
    assert.equal(consent.checked, true);
    assert.equal((await third.invoke({command:'inspect', schema_version:1})).preferences.revision, 3);
    document.getElementById('settings').dispatchEvent(new window.Event('click'));
    assert.equal(document.getElementById('gear').hidden, false);
    assert.equal(document.getElementById('install-path').value, join(root, 'Game'));
    document.getElementById('choose').dispatchEvent(new window.Event('click'));
    await app.settled(); await flush();
    assert.equal((await third.invoke({command:'inspect', schema_version:1})).preferences.revision, 3);
    document.getElementById('notes').dispatchEvent(new window.Event('click'));
    await app.settled(); await flush();
    assert.equal(document.getElementById('entries').querySelector('h3').textContent, 'Fixture <patch>');
    assert.equal(document.getElementById('entries').querySelectorAll('patch').length, 0);
    assert.equal(consent.checked, true);
    console.log('PASS patch tab: fixture rendered as text while persisted consent remains unchanged; signature and transport tested separately in Rust');
    console.log('PASS rendered controls: consent saved natively, settings show saved path, cancelled chooser does not save');
  } finally { await app.dispose(); }
  await third.close();
  const fourth = start();
  assert.equal((await fourth.invoke({command:'inspect', schema_version:1})).preferences.launcher_summary_consent, true);
  await fourth.close();
  console.log('PASS restart after UI action: checkbox change persisted; no desktop or network used');
  // Seed only a journal fixture: this proves native reopen and JS decoding, not
  // an admitted Update plan or content replacement.
  await writeFile(join(root, 'operation.json'), JSON.stringify({schema_version:1,revision:8,
    operation:{id:'310ba1b3-1ca1-4af8-a1df-6785b3e824b6',kind:'update',state:'running',intent_digest:Array(32).fill(7)}}));
  const update = start();
  const calls=[];
  await Effect.runPromise(Effect.scoped(Effect.gen(function*(){
    const launcher=yield* makeLauncher;
    yield* launcher.inspect;
    const value=(yield* launcher.snapshot).native.operation;
    assert.equal(value.operation.kind,'update');
    assert.equal(value.operation.state,'reconciliation_required');
    assert.equal(value.revision,9);
  }).pipe(Effect.provide(bridgeLayer(request=>{calls.push(request);return update.invoke(request);})))));
  assert.deepEqual(calls,[{command:'inspect',schema_version:1}]);
  await update.close();
  const savedUpdate=JSON.parse(await readFile(join(root,'operation.json'),'utf8'));
  assert.equal(savedUpdate.operation.state,'reconciliation_required');
  const reopenedUpdate=start();
  assert.deepEqual((await reopenedUpdate.invoke({command:'inspect',schema_version:1})).operation,savedUpdate);
  await reopenedUpdate.close();
  console.log('PASS Update journal: native reopen persists reconciliation once; Effect accepts kind and revision without redispatch. Not covered: Update admission, confirmation, replacement, recovery UI or visual rendering.');

} finally {
  for (const child of children) {
    if (child.exitCode === null && child.signalCode === null) {
      const exit = once(child, 'exit'); child.kill(); await exit;
    }
  }
  await rm(root, {recursive:true, force:true});
}
