/**
 * Check whether the app is running inside a Tauri webview context.
 */
export function isTauri(): boolean {
    return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/**
 * Invoke a Tauri IPC command if running in the Tauri context.
 * Falls back to a warning log and returns undefined when running in a browser.
 *
 * @param cmd - The Tauri command name (must match a #[tauri::command] handler)
 * @param args - Optional arguments object passed to the command
 * @returns The command result, or undefined if not in Tauri
 */
export async function tauriInvoke<T>(
    cmd: string,
    args?: Record<string, unknown>,
): Promise<T | undefined> {
    if (!isTauri()) {
        console.warn(`[tauri] not in Tauri context, skipping invoke: ${cmd}`);
        return undefined;
    }

    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<T>(cmd, args);
}
