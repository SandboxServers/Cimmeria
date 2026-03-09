let tauriModule: typeof import('@tauri-apps/api/core') | null = null;

export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

async function getTauri() {
  if (!isTauri()) return null;
  if (!tauriModule) {
    tauriModule = await import('@tauri-apps/api/core');
  }
  return tauriModule;
}

export async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const tauri = await getTauri();
  if (!tauri) {
    throw new Error(`Tauri IPC not available (command: ${command})`);
  }
  return tauri.invoke<T>(command, args);
}
