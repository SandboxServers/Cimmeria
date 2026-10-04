import { mountPatchNotes } from "./patch-notes";
import { Context, Effect, Fiber, Layer, ManagedRuntime, Result, Schema, Stream } from "effect";
import { BridgeFailure, safeFailure } from "./contract";
import { bridgeLayer, makeLauncher, ScreenState } from "./workflows";

export type Invoke = (command: string, args?: Record<string, unknown>) => Promise<unknown>;
class Launcher extends Context.Service<Launcher, Effect.Success<typeof makeLauncher>>()('launcher/Application') {}

const errorText: Record<BridgeFailure['code'], string> = {
  transport: 'The launcher could not confirm the result. Recheck status before trying again.',
  schema: 'The interface and launcher versions do not match. Restart with a matching build.',
  in_use: 'Another launcher is using these settings. Close it, then recheck status.',
  io: 'The launcher could not read or save local settings. Check permissions and available disk space.',
  corrupt: 'Local settings could not be read. They have been preserved for recovery.',
  unsupported_schema: 'These settings belong to a newer launcher. Use that version to continue.',
  too_large: 'The local state file is too large to read safely. It has been preserved.',
  unsafe_file: 'The local state location is not a regular file. Nothing was overwritten.',
  invalid_directory: 'Choose an accessible local game folder. It must exist before it can be opened.',
  stale_revision: 'Settings changed while this request was pending. Recheck status.',
  busy: 'An operation is using the current game folder. Wait for it to finish before changing folders.',
  persistence_uncertain: 'The last save could not be confirmed. Restart the launcher to inspect the saved state.',
};

/** Mount once per application. Tab changes only hide panels; they do not own workflows. */
export function mountLauncher(document: Document, invoke: Invoke) {
  const runtime = ManagedRuntime.make(Layer.effect(Launcher, makeLauncher).pipe(
    Layer.provide(bridgeLayer(request => invoke('launcher_command', {request}))),
  ));
  const get = <T extends HTMLElement = HTMLElement>(id: string): T => {
    const value = document.getElementById(id);
    if (!value) throw new Error(`Missing launcher element ${id}`);
    return value as T;
  };
  const patchNotes = mountPatchNotes(document, () => invoke('fetch_patch_notes'));
  const checkbox = get<HTMLInputElement>('telemetry');
  const path = get<HTMLInputElement>('install-path');
  const abort = new AbortController();
  let current: ScreenState = {native:null, busy:false, needsInspection:true, error:null};
  let active: string | null = null;
  let notice: string | null = null;
  let localError: BridgeFailure['code'] | null = null;
  let pendingConsent: boolean | null = null;
  let disposed = false;
  let settled: Promise<void> = Promise.resolve();
  const listeners: (() => void)[] = [];

  const render = () => {
    if (disposed) return;
    const native = current.native;
    const enabled = !!native && !native.requires_reopen && !current.needsInspection && !current.busy && !active;
    checkbox.checked = pendingConsent ?? native?.preferences.launcher_summary_consent ?? false;
    checkbox.disabled = !enabled;
    path.value = native?.preferences.install_directory ?? '';
    get<HTMLButtonElement>('choose').disabled = !enabled;
    get<HTMLButtonElement>('folder').disabled = !enabled || !native?.preferences.install_directory;
    const failure = localError ?? current.error;
    get('status').textContent = failure ? errorText[failure] : active ?? notice ??
      (native ? 'Settings loaded. Game installation is not connected in this build.' : 'Opening launcher…');
    get('retry').hidden = !failure || native?.requires_reopen === true;
    get<HTMLButtonElement>('retry').disabled = !!active || current.busy;
    get('gear').setAttribute('aria-busy', String(!!active || current.busy));
  };
  const watcher = runtime.runFork(Effect.flatMap(Launcher, launcher => Stream.runForEach(launcher.changes,
    next => Effect.sync(() => { current = next; render(); }))));

  const run = (effect: Effect.Effect<unknown, BridgeFailure, Launcher>, activity: string, success: string) => {
    if (disposed || active) return;
    active = activity; notice = null; localError = null; render();
    settled = runtime.runPromise(Effect.result(effect), {signal:abort.signal}).then(result => {
      if (disposed) return;
      active = null; pendingConsent = null;
      if (Result.isFailure(result)) localError = result.failure.code;
      else notice = success;
      render();
    }).catch(() => {
      if (disposed) return;
      active = null; pendingConsent = null; localError = 'transport'; render();
    });
  };
  const on = (id: string, type: string, listener: () => void) => {
    get(id).addEventListener(type, listener);
    listeners.push(() => get(id).removeEventListener(type, listener));
  };
  const showTab = (notes: boolean) => {
    get('patches').hidden = !notes; get('play').hidden = notes;
    get('home').setAttribute('aria-pressed', String(!notes));
    get('notes').setAttribute('aria-pressed', String(notes));
  };
  on('home', 'click', () => showTab(false));
  on('notes', 'click', () => {showTab(true); patchNotes.open();});
  on('settings', 'click', () => {
    get('gear').hidden = !get('gear').hidden;
    get('settings').setAttribute('aria-expanded', String(!get('gear').hidden));
  });
  on('retry', 'click', () => run(Effect.flatMap(Launcher, launcher => launcher.inspect),
    'Checking saved state…', 'Saved state checked.'));
  on('telemetry', 'change', () => {
    if (checkbox.disabled) return;
    const consent = checkbox.checked; pendingConsent = consent;
    run(Effect.flatMap(Launcher, launcher => Effect.gen(function* () {
      const state = yield* launcher.snapshot;
      return yield* launcher.savePreferences(state.native?.preferences.install_directory ?? null, consent);
    })), 'Saving your diagnostics choice…', 'Diagnostics choice saved. This build sends nothing.');
  });
  on('choose', 'click', () => {
    if (get<HTMLButtonElement>('choose').disabled) return;
    let selected = false;
    const effect = Effect.gen(function* () {
      const raw = yield* Effect.tryPromise({try: () => invoke('choose_install_directory'), catch: safeFailure});
      const directory = yield* Schema.decodeUnknownEffect(Schema.NullOr(Schema.String))(raw).pipe(
        Effect.mapError(() => new BridgeFailure({code:'schema'})),
      );
      if (directory === null) return;
      selected = true;
      const launcher = yield* Launcher;
      const state = yield* launcher.snapshot;
      yield* launcher.savePreferences(directory, state.native?.preferences.launcher_summary_consent ?? false);
    });
    run(effect, 'Choose a game folder in the native dialog…', 'Folder selection finished.');
    settled = settled.then(() => {
      if (!disposed && !localError) { notice = selected ? 'Game folder saved.' : 'Folder selection cancelled. Nothing changed.'; render(); }
    });
  });
  on('folder', 'click', () => {
    if (get<HTMLButtonElement>('folder').disabled) return;
    run(Effect.tryPromise({try: () => invoke('show_install_directory'), catch: safeFailure}),
      'Showing the saved game folder…', 'Game folder shown.');
  });
  run(Effect.flatMap(Launcher, launcher => launcher.inspect), 'Opening launcher…', 'Settings loaded.');
  const ready = settled;
  return {
    ready,
    settled: () => Promise.all([settled, patchNotes.settled()]),
    dispose: async () => {
      if (disposed) return;
      disposed = true; abort.abort(); listeners.forEach(remove => remove());
      await Effect.runPromise(Fiber.interrupt(watcher));
      await settled;
      await patchNotes.dispose();
      await runtime.dispose();
    },
  };
}
