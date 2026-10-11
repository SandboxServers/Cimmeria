import type { NativeSnapshot } from "./contract";

/**
 * What `mountLauncher` gives every Settings panel besides `document` and `invoke`
 * (LX-02). Panels read preferences, including the revision a save needs, from
 * here instead of asking native code themselves.
 */
export type PanelContext = {
  /** The latest native snapshot, or undefined before the first inspect answers. */
  snapshot: () => NativeSnapshot | undefined;
  /** Calls `listener` with each later snapshot. Returns the unsubscribe. */
  onSnapshot: (listener: (snapshot: NativeSnapshot) => void) => () => void;
  /** Re-inspect after a panel saved something, so every view sees the new revision. */
  refresh: () => Promise<unknown>;
};

/** The feeding side, kept by `mountLauncher`: `publish` each snapshot it receives. */
export function makePanelContext(refresh: () => Promise<unknown> = async () => {}) {
  let latest: NativeSnapshot | undefined;
  const listeners = new Set<(snapshot: NativeSnapshot) => void>();
  const context: PanelContext = {
    snapshot: () => latest,
    onSnapshot: listener => { listeners.add(listener); return () => { listeners.delete(listener); }; },
    refresh,
  };
  const publish = (snapshot: NativeSnapshot) => { latest = snapshot; for (const listener of [...listeners]) listener(snapshot); };
  return { context, publish };
}
