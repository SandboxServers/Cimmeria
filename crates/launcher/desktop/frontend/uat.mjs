// Sequential JS logic UAT against the real Rust command handler and disk store.
// Run after building engine/examples/state_bridge through the repository lane.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, rm } from 'node:fs/promises';
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
  await third.close();
  console.log('PASS second restart: opt-out durable; no GUI or network used');
} finally {
  for (const child of children) {
    if (child.exitCode === null && child.signalCode === null) {
      const exit = once(child, 'exit'); child.kill(); await exit;
    }
  }
  await rm(root, {recursive:true, force:true});
}
