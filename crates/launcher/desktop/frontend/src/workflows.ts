import { Context, Effect, Layer, PubSub, Ref, Schedule, Semaphore, Stream } from "effect";
import { BridgeFailure, Command, decodeSnapshot, NativeSnapshot, safeFailure } from "./contract";

export class NativeBridge extends Context.Service<NativeBridge, {
  readonly invoke: (request: Command) => Effect.Effect<NativeSnapshot, BridgeFailure>;
}>()("launcher/NativeBridge") {}

/** The same adapter accepts Tauri invoke in the app and a process transport in UAT. */
export const bridgeLayer = (invoke: (request: Command) => Promise<unknown>) => Layer.succeed(NativeBridge, {
  invoke: (request) => Effect.tryPromise({try: () => invoke(request), catch: safeFailure}).pipe(
    Effect.timeout("5 seconds"),
    Effect.catchTag("TimeoutError", () => Effect.fail(new BridgeFailure({code: "transport"}))),
    Effect.flatMap(decodeSnapshot),
  ),
});

export type ScreenState = {
  readonly native: NativeSnapshot | null;
  readonly busy: boolean;
  readonly needsInspection: boolean;
  readonly error: BridgeFailure["code"] | null;
};

/** Application scope, not a tab/view scope. Closing views never cancels native work. */
export const makeLauncher = Effect.gen(function* () {
  const bridge = yield* NativeBridge;
  const gate = yield* Semaphore.make(1);
  const state = yield* Ref.make<ScreenState>({native: null, busy: false, needsInspection: true, error: null});
  const events = yield* Effect.acquireRelease(PubSub.sliding<ScreenState>({capacity: 1, replay: 1}), PubSub.shutdown);
  const publish = (next: ScreenState) => Ref.set(state, next).pipe(Effect.andThen(PubSub.publish(events, next)), Effect.asVoid);
  yield* publish(yield* Ref.get(state));

  const accept = (next: NativeSnapshot) => Effect.gen(function* () {
    const current = yield* Ref.get(state);
    // An old response must not undo a saved preference or an operation observation.
    if (current.native && (next.operation.revision < current.native.operation.revision ||
        next.preferences.revision < current.native.preferences.revision)) {
      return yield* Effect.fail(new BridgeFailure({code: "stale_revision"}));
    }
    yield* publish({native: next, busy: current.busy, needsInspection: next.requires_reopen,
      error: next.requires_reopen ? "persistence_uncertain" : null});
    return next;
  });

  const inspectUnlocked = bridge.invoke({command: "inspect", schema_version: 1}).pipe(
    Effect.retry({times: 2, schedule: Schedule.exponential("100 millis"), while: error => error.code === "transport"}),
    Effect.flatMap(accept),
  );
  const recordFailure = (error: BridgeFailure) => Ref.get(state).pipe(
    Effect.flatMap(current => publish({...current, needsInspection: true, error: error.code})),
  );
  const inspect = inspectUnlocked.pipe(Effect.tapError(recordFailure), Semaphore.withPermits(gate, 1));

  const savePreferences = (installDirectory: string | null, consent: boolean) => Effect.gen(function* () {
    let current = yield* Ref.get(state);
    if (current.needsInspection || !current.native) {
      yield* inspectUnlocked;
      current = yield* Ref.get(state);
    }
    if (!current.native || current.native.requires_reopen) {
      return yield* Effect.fail(new BridgeFailure({code: "persistence_uncertain"}));
    }
    yield* publish({...current, busy: true, error: null});
    const request: Command = {command: "save_preferences", schema_version: 1,
      expected_revision: current.native.preferences.revision, install_directory: installDirectory,
      launcher_summary_consent: consent};
    // Exactly one mutation call. Transport failure is uncertain; only an inspection
    // may clear the gate. Interruption also leaves inspection required.
    yield* publish({...current, busy: true, needsInspection: true, error: null});
    return yield* bridge.invoke(request).pipe(Effect.flatMap(accept));
  }).pipe(
    Effect.tapError(recordFailure),
    Effect.ensuring(Ref.get(state).pipe(Effect.flatMap(current => publish({...current, busy: false})))),
    Semaphore.withPermits(gate, 1),
  );

  return {inspect, savePreferences, snapshot: Ref.get(state), changes: Stream.fromPubSub(events)};
});
