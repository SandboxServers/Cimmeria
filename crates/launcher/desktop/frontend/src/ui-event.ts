import type {Invoke} from './view';
declare global { interface Window { __TAURI__: { core: { invoke: Invoke } } } }

/** The screens and subsystems a `ui_event` may name; the native side checks the same list (C13). */
export type UiArea='install'|'adoption'|'update'|'game_update'|'launch'|'repair'|'settings'|'seed_archive'|'server_list'|'log_upload'|'physx'|'migration'|'patch_notes'|'client_patches'|'client_maintenance'|'telemetry';

/** Sends a swallowed frontend error to the launcher's telemetry as `ui_event`. Fire and forget: the player is never told. */
export function reportUiEvent(area:UiArea,event:string,code:string,level:'info'|'warn'='warn'):void {
 const tauri=(globalThis as {window?:Window}).window?.__TAURI__;
 if(!tauri) return; // Not inside the launcher window (tests, UAT scripts): nothing to report to.
 tauri.core.invoke('ui_event',{area,event,code,level}).catch(() => undefined); // Telemetry is best effort; a refused report is not an error for the player.
}
