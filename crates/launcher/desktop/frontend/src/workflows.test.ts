import test from "node:test";
import assert from "node:assert/strict";
import { Effect, Fiber, Result, Stream } from "effect";
import { TestClock } from "effect/testing";
import { NativeSnapshot } from "./contract";
import { bridgeLayer, makeLauncher } from "./workflows";

const initial = (): NativeSnapshot => ({schema_version: 1,
  operation: {schema_version: 1, revision: 0, operation: null},
  preferences: {schema_version: 1, revision: 0, install_directory: null, launcher_summary_consent: false},
  requires_reopen: false});

function failureCode(result: Result.Result<unknown, {code: string}>) {
  assert.ok(Result.isFailure(result));
  return result.failure.code;
}

test("inspection retries transport failures only and publishes authoritative state", {timeout: 5_000}, async () => {
  let calls = 0;
  await Effect.runPromise(Effect.scoped(Effect.gen(function* () {
    const launcher = yield* makeLauncher;
    const inspecting = yield* launcher.inspect.pipe(Effect.forkScoped);
    yield* TestClock.adjust("1 second");
    yield* Fiber.join(inspecting);
    assert.equal(calls, 3);
    assert.equal((yield* launcher.snapshot).needsInspection, false);
  }).pipe(Effect.provide(bridgeLayer(async () => {
    if (++calls < 3) throw new Error("temporary connection error");
    return initial();
  })), Effect.provide(TestClock.layer()))));
});

test("schema mismatch fails immediately and never copies raw error details", {timeout: 5_000}, async () => {
  let calls = 0;
  await Effect.runPromise(Effect.scoped(Effect.gen(function* () {
    const launcher = yield* makeLauncher;
    const result = yield* Effect.result(launcher.inspect);
    assert.equal(failureCode(result), "schema");
    assert.equal(calls, 1);
    assert.equal((yield* launcher.snapshot).native, null);
  }).pipe(Effect.provide(bridgeLayer(async () => { calls++; return {...initial(), schema_version: 99}; })))));
});

test("lost save reply is never retried and next intent reconciles first", {timeout: 5_000}, async () => {
  let state = initial();
  const commands: string[] = [];
  let saves = 0;
  await Effect.runPromise(Effect.scoped(Effect.gen(function* () {
    const launcher = yield* makeLauncher;
    yield* launcher.inspect;
    const lost = yield* Effect.result(launcher.savePreferences(null, true));
    assert.equal(failureCode(lost), "transport");
    assert.equal(saves, 1);
    assert.equal((yield* launcher.snapshot).needsInspection, true);
    assert.equal((yield* launcher.snapshot).native?.preferences.launcher_summary_consent, false);
    yield* launcher.savePreferences(null, false);
    assert.deepEqual(commands, ["inspect", "save_preferences", "inspect", "save_preferences"]);
    assert.equal((yield* launcher.snapshot).native?.preferences.revision, 2);
    assert.equal((yield* launcher.snapshot).native?.preferences.launcher_summary_consent, false);
  }).pipe(Effect.provide(bridgeLayer(async request => {
    commands.push(request.command);
    if (request.command === "save_preferences") {
      assert.equal(request.expected_revision, state.preferences.revision);
      state = {...state, preferences: {...state.preferences, revision: state.preferences.revision + 1,
        launcher_summary_consent: request.launcher_summary_consent}};
      if (++saves === 1) throw new Error("reply lost; native already persisted");
    }
    return state;
  })))));
});

test("failed native preference write keeps saved consent and allows inspection", {timeout: 5_000}, async () => {
  await Effect.runPromise(Effect.scoped(Effect.gen(function* () {
    const launcher = yield* makeLauncher;
    yield* launcher.inspect;
    const result = yield* Effect.result(launcher.savePreferences(null, true));
    assert.equal(failureCode(result), "io");
    assert.equal((yield* launcher.snapshot).native?.preferences.launcher_summary_consent, false);
    assert.equal((yield* launcher.snapshot).busy, false);
    yield* launcher.inspect;
    assert.equal((yield* launcher.snapshot).needsInspection, false);
  }).pipe(Effect.provide(bridgeLayer(async request => {
    if (request.command === "save_preferences") throw "io";
    return initial();
  })))));
});

test("stale snapshot cannot overwrite saved preference revision", {timeout: 5_000}, async () => {
  let calls = 0;
  const newer = {...initial(), preferences: {...initial().preferences, revision: 2, launcher_summary_consent: true}};
  await Effect.runPromise(Effect.scoped(Effect.gen(function* () {
    const launcher = yield* makeLauncher;
    yield* launcher.inspect;
    const result = yield* Effect.result(launcher.inspect);
    assert.equal(failureCode(result), "stale_revision");
    assert.equal((yield* launcher.snapshot).native?.preferences.revision, 2);
    assert.equal((yield* launcher.snapshot).native?.preferences.launcher_summary_consent, true);
  }).pipe(Effect.provide(bridgeLayer(async () => ++calls === 1 ? newer : initial())))));
});

test("scope interruption cleans subscriptions and retains native uncertainty", {timeout: 5_000}, async () => {
  let entered!: () => void;
  const started = new Promise<void>(resolve => { entered = resolve; });
  let finish!: (snapshot: NativeSnapshot) => void;
  const reply = new Promise<NativeSnapshot>(resolve => { finish = resolve; });
  await Effect.runPromise(Effect.scoped(Effect.gen(function* () {
    const launcher = yield* makeLauncher;
    const observed: boolean[] = [];
    const watcher = yield* Stream.runForEach(launcher.changes,
      value => Effect.sync(() => { observed.push(value.busy); })).pipe(Effect.forkScoped);
    yield* launcher.inspect;
    const saving = yield* launcher.savePreferences(null, true).pipe(Effect.forkScoped);
    yield* Effect.promise(() => started);
    yield* Fiber.interrupt(saving);
    assert.equal((yield* launcher.snapshot).busy, false);
    assert.equal((yield* launcher.snapshot).needsInspection, true);
    assert.equal((yield* launcher.snapshot).native?.preferences.launcher_summary_consent, false);
    yield* Fiber.interrupt(watcher);
    const count = observed.length;
    finish(initial());
    yield* Effect.yieldNow;
    assert.equal(observed.length, count);
  }).pipe(Effect.provide(bridgeLayer(async request => {
    if (request.command === "inspect") return initial();
    entered(); return reply;
  })))));
});

test("read retries stop after two retries using the test clock", {timeout: 5_000}, async () => {
  let calls = 0;
  await Effect.runPromise(Effect.scoped(Effect.gen(function* () {
    const launcher = yield* makeLauncher;
    const inspecting = yield* Effect.result(launcher.inspect).pipe(Effect.forkScoped);
    yield* TestClock.adjust("1 second");
    assert.equal(failureCode(yield* Fiber.join(inspecting)), "transport");
    assert.equal(calls, 3);
    assert.equal((yield* launcher.snapshot).needsInspection, true);
  }).pipe(Effect.provide(bridgeLayer(async () => { calls++; throw new Error("private filesystem path"); })),
    Effect.provide(TestClock.layer()))));
});

test("an IPC timeout leaves a single save uncertain and releases the UI busy state", {timeout: 5_000}, async () => {
  let saves = 0;
  await Effect.runPromise(Effect.scoped(Effect.gen(function* () {
    const launcher = yield* makeLauncher;
    yield* launcher.inspect;
    const saving = yield* Effect.result(launcher.savePreferences(null, true)).pipe(Effect.forkScoped);
    yield* TestClock.adjust("6 seconds");
    assert.equal(failureCode(yield* Fiber.join(saving)), "transport");
    assert.equal(saves, 1);
    assert.equal((yield* launcher.snapshot).needsInspection, true);
    assert.equal((yield* launcher.snapshot).busy, false);
  }).pipe(Effect.provide(bridgeLayer(async request => {
    if (request.command === "inspect") return initial();
    saves++;
    return new Promise(() => {});
  })), Effect.provide(TestClock.layer()))));
});
