import { Effect, Schema } from 'effect';
import { BridgeFailure, safeFailure } from './contract';
import type { Invoke } from './view';

/**
 * The native folder dialog, then one save of the chosen folder. The save is the caller's
 * (Settings owns the preference revision). Resolves `true` when a folder was saved and
 * `false` when the dialog was cancelled, which changes nothing.
 */
export function chooseInstallFolder<R>(
  invoke: Invoke,
  save: (directory: string) => Effect.Effect<unknown, BridgeFailure, R>,
): Effect.Effect<boolean, BridgeFailure, R> {
  return Effect.gen(function* () {
    const raw = yield* Effect.tryPromise({try: () => invoke('choose_install_directory'), catch: safeFailure});
    const directory = yield* Schema.decodeUnknownEffect(Schema.NullOr(Schema.String))(raw).pipe(
      Effect.mapError(() => new BridgeFailure({code: 'schema'})),
    );
    if (directory === null) return false;
    yield* save(directory);
    return true;
  });
}
