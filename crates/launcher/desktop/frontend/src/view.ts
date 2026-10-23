import { Context, Effect, Fiber, Layer, ManagedRuntime, Result, Schema, Stream } from "effect";
import { BridgeFailure, operationOpen, safeFailure } from "./contract";
import { bridgeLayer, makeLauncher, ScreenState } from "./workflows";
import { mountServerList } from "./server-list-view";
import { mountClientPatches } from "./client-patches-view";
import { mountLogUpload } from "./log-upload-view";
import { mountClientMaintenance } from "./client-maintenance-view";
import { mountClientChanges } from "./client-changes-view";
import { mountPhysx } from "./physx-view";
import { mountSeedArchive } from "./seed-archive-view";
import { makePanelContext } from "./panel-context";
import { aboutText, buildBadge } from "./about";
import { displayPath } from "./display-path";

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
  // Required by the Record over every code; this view's commands never return it (the server list panel shows its own).
  invalid_server_list: 'The login server list was refused. Fix it under Login servers, then try again.',
};

/** Mount once per application. Tab changes only hide panels; they do not own workflows. */
export function mountLauncher(document: Document, invoke: Invoke, onSettingsChanged: () => void = () => {}, onArchiveVerified:()=>Promise<unknown>=async()=>{}) {
  const runtime = ManagedRuntime.make(Layer.effect(Launcher, makeLauncher).pipe(
    Layer.provide(bridgeLayer(request => invoke('launcher_command', {request}))),
  ));
  const get = <T extends HTMLElement = HTMLElement>(id: string): T => {
    const value = document.getElementById(id);
    if (!value) throw new Error(`Missing launcher element ${id}`);
    return value as T;
  };
  // Settings panels, one file each (LX-02 scaffold; LX-03 to LX-07 fill them).
  // One line per panel so parallel packets change separate lines.
  const panelFeed = makePanelContext(() => refresh());
  const panels: {settled: () => Promise<unknown>; dispose: () => Promise<void>}[] = [
    mountServerList(document, invoke, panelFeed.context),
    mountClientPatches(document, invoke, panelFeed.context),
    mountLogUpload(document, invoke, panelFeed.context),
    mountClientMaintenance(document, invoke, panelFeed.context),
    mountClientChanges(document, invoke, panelFeed.context),
    mountPhysx(document, invoke, panelFeed.context),
    mountSeedArchive(document, invoke, panelFeed.context,undefined,onArchiveVerified),
  ];
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
    path.value = displayPath(native?.preferences.install_directory ?? '');
    get<HTMLButtonElement>('choose').disabled = !enabled || operationOpen(native?.operation.operation);
    get<HTMLButtonElement>('folder').disabled = !enabled || !native?.preferences.install_directory;
    const failure = localError ?? current.error;
    get('status').textContent = failure ? errorText[failure] : active ?? notice ??
      (native ? 'Settings loaded.' : 'Opening launcher…');
    get('retry').hidden = !failure || native?.requires_reopen === true;
    get<HTMLButtonElement>('retry').disabled = !!active || current.busy;
    get('gear').setAttribute('aria-busy', String(!!active || current.busy));
    get('about').textContent = aboutText(native?.launcher ?? null, !!failure);
    get('build-badge').textContent = buildBadge(native?.launcher ?? null);
  };
  const watcher = runtime.runFork(Effect.flatMap(Launcher, launcher => Stream.runForEach(launcher.changes,
    next => Effect.sync(() => { current = next; render(); if (!disposed && next.native) panelFeed.publish(next.native); }))));

  const run = (effect: Effect.Effect<unknown, BridgeFailure, Launcher>, activity: string, success: string) => {
    if (disposed || active) return;
    active = activity; notice = null; localError = null; render();
    settled = runtime.runPromise(Effect.result(effect), {signal:abort.signal}).then(result => {
      if (disposed) return;
      active = null; pendingConsent = null;
      if (Result.isFailure(result)) localError = result.failure.code;
      else { notice = success; onSettingsChanged(); }
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
  const showHome = () => {
    get('gear').hidden = true; get('play').hidden = false;
    get('settings').setAttribute('aria-expanded', 'false');
    get('home').setAttribute('aria-pressed', 'true');
  };
  on('home', 'click', showHome);
  on('settings', 'click', () => {
    const opened = get('gear').hidden;
    get('gear').hidden = !opened; get('play').hidden = opened;
    get('settings').setAttribute('aria-expanded', String(opened));
    get('home').setAttribute('aria-pressed', String(!opened));
  });
  on('retry', 'click', () => run(Effect.flatMap(Launcher, launcher => launcher.inspect),
    'Checking saved state…', 'Saved state checked.'));
  on('telemetry', 'change', () => {
    if (checkbox.disabled) return;
    const consent = checkbox.checked; pendingConsent = consent;
    // The done text says where the summaries go, from the endpoint the snapshot reports.
    const done = !consent ? 'Diagnostics choice saved. Nothing is sent.'
      : current.native?.setup_diagnostics_endpoint ? 'Diagnostics choice saved. Setup diagnostics are sent over https to your login server.'
      : 'Diagnostics choice saved. Setup diagnostics are saved but not sent yet: this server has no secure (https) address for them.';
    run(Effect.flatMap(Launcher, launcher => Effect.gen(function* () {
      const state = yield* launcher.snapshot;
      return yield* launcher.savePreferences(state.native?.preferences.install_directory ?? null, consent);
    })), 'Saving your diagnostics choice…', done);
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
  // A refresh resolves only after an inspect that started after the call. While any run is in
  // flight, every refresh asked for joins one queued inspect after it: returning the in-flight
  // run would hand a panel's post-save refresh a snapshot read before its save committed.
  let queuedRefresh: Promise<void> | null = null;
  const refresh = (): Promise<void> => {
    if (disposed) return settled;
    if (!active) { run(Effect.flatMap(Launcher, launcher => launcher.inspect), 'Checking settings…', 'Settings loaded.'); return settled; }
    // Re-entering refresh (not run) queues again if another run took the slot first.
    queuedRefresh ??= settled.then(() => { queuedRefresh = null; return refresh(); });
    return queuedRefresh;
  };
  return {
    ready,
    context: panelFeed.context,
    refresh,
    settled: () => Promise.all([queuedRefresh ?? settled, ...panels.map(panel => panel.settled())]),
    dispose: async () => {
      if (disposed) return;
      disposed = true; abort.abort(); listeners.forEach(remove => remove());
      await Effect.runPromise(Fiber.interrupt(watcher));
      await settled;
      await Promise.all(panels.map(panel => panel.dispose()));
      await runtime.dispose();
    },
  };
}
