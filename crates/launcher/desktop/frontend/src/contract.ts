import { Data, Effect, Schema } from "effect";

const Revision = Schema.Int.check(Schema.isBetween({ minimum: 0, maximum: Number.MAX_SAFE_INTEGER }));
const Operation = Schema.Struct({
  id: Schema.String,
  kind: Schema.Literals(["install", "prepare_runtime", "repair", "uninstall", "launch", "adopt", "update"]),
  intent_digest: Schema.Array(Schema.Int.check(Schema.isBetween({minimum: 0, maximum: 255})))
    .check(Schema.isBetweenLength(32, 32)),
  state: Schema.Literals(["starting", "running", "cancel_requested", "succeeded", "failed", "cancelled", "reconciliation_required"]),
});
/** True while an operation has not reached a terminal state; an absent operation is not open. */
export const operationOpen=(op:{state:string}|null|undefined):boolean=>!!op&&!['succeeded','failed','cancelled'].includes(op.state);
/** `cimmeria_launcher_core::client_setup::login_servers::LoginServer`. */
export const LoginServer = Schema.Struct({name: Schema.String, url: Schema.String});
export type LoginServer = typeof LoginServer.Type;
/** Engine `storage::display_options` (C-PD6). macOS offers `windowed` only. */
export const DisplayMode = Schema.Literals(["windowed", "fullscreen"]);
export type DisplayMode = typeof DisplayMode.Type;
/** Engine `display_options::FrameCap` (C-PD8): DXVK's limit on macOS; Windows offers none. */
export const FrameCap = Schema.Literals(["fps30", "fps60", "display"]);
export type FrameCap = typeof FrameCap.Type;
// `frame_cap`: the engine omits fps30 (today's cap), and a save without it stores fps30.
export const DisplayOptions = Schema.Struct({mode: DisplayMode, width: Schema.Number, height: Schema.Number, frame_cap: Schema.optionalKey(FrameCap)});
export type DisplayOptions = typeof DisplayOptions.Type;
/** Engine `display_targets::DisplayInfo` (C-PD8): one display, in the units the game sees. */
export const DisplayInfo = Schema.Struct({
  index: Schema.Number, main: Schema.Boolean, x: Schema.Number, y: Schema.Number,
  width: Schema.Number, height: Schema.Number, scale_percent: Schema.Number, refresh_hz: Schema.Number,
});
export type DisplayInfo = typeof DisplayInfo.Type;
/** Engine `display_options::MonitorRect` (C-PD8): the `x, y, width, height` of one of the snapshot's `displays`. */
export type MonitorRect = {x: number; y: number; width: number; height: number};
export const NativeSnapshot = Schema.Struct({
  schema_version: Schema.Literal(1),
  operation: Schema.Struct({schema_version: Schema.Literal(1), revision: Revision, operation: Schema.NullOr(Operation)}),
  preferences: Schema.Struct({
    schema_version: Schema.Literal(2), revision: Revision,
    install_directory: Schema.NullOr(Schema.String), launcher_summary_consent: Schema.Boolean,
    // Empty means the built-in default server list.
    login_servers: Schema.Array(LoginServer), client_patches_enabled: Schema.Boolean,
    // The launcher writes this into the client profile before every Play.
    display: DisplayOptions,
  }),
  requires_reopen: Schema.Boolean,
  // An adopted copy plays with the list its adoption reviewed, whatever is saved above.
  adopted_login_servers: Schema.NullOr(Schema.Array(LoginServer)),
  // The running build (engine `LauncherBuild`); `tag` is null for a development build.
  launcher: Schema.Struct({version: Schema.String, tag: Schema.NullOr(Schema.String)}),
  // Whether setup diagnostics have an https endpoint to go to (engine `setup_diagnostics_endpoint`).
  setup_diagnostics_endpoint: Schema.optionalKey(Schema.Boolean),
  // The display modes this platform offers (engine `display_options::allowed_modes`).
  display_modes: Schema.Array(DisplayMode),
  // The active displays and, per display, the window sizes that fit it (C-PD8,
  // engine `display_targets`). Optional so older fixtures decode; the engine always sends both.
  displays: Schema.optionalKey(Schema.Array(DisplayInfo)),
  display_sizes: Schema.optionalKey(Schema.Array(Schema.Array(Schema.Tuple([Schema.Number, Schema.Number])))),
  // The `index` in `displays` the current display setup remembers; null means the main display
  // (C-PD8, engine `display_options::placement_for`). Optional so older fixtures decode.
  display_monitor: Schema.optionalKey(Schema.NullOr(Schema.Number)),
  // The frame caps this platform offers (engine `display_options::frame_caps`): all three on macOS,
  // empty elsewhere. Optional so older fixtures decode; the engine always sends it.
  frame_caps: Schema.optionalKey(Schema.Array(FrameCap)),
});
export type NativeSnapshot = typeof NativeSnapshot.Type;
export type Preferences = NativeSnapshot["preferences"];
export type Command = {command: "inspect"; schema_version: 1} | {
  command: "save_preferences"; schema_version: 1; expected_revision: number;
  install_directory: string | null; launcher_summary_consent: boolean;
} | {
  command: "save_client_options"; schema_version: 1; expected_revision: number;
  login_servers: readonly LoginServer[]; client_patches_enabled: boolean;
} | {
  command: "save_display_options"; schema_version: 1; expected_revision: number; display: DisplayOptions;
  // Remembered for the current display setup; absent or null means the main display (C-PD8).
  monitor?: MonitorRect | null;
};

// Native storage codes as `StorageError` serializes them.
export type StorageCode = "in_use" | "io" | "corrupt" | "unsupported_schema" | "too_large" | "unsafe_file" |
  "invalid_directory" | "stale_revision" | "busy" | "persistence_uncertain" | "invalid_server_list" |
  "invalid_display_options";
/** The three scaffold commands (LX-02) fail with one flat code string: a storage code or `"unavailable"`. */
export type ScaffoldError = StorageCode | "unavailable";

// `log_upload_command` (engine `storage/log_upload/`, LX-05).
export type LogUploadCommand = {command: "inspect"; schema_version: 1} | {command: "upload"; schema_version: 1};
const logUploadPlain = <S extends string>(state: S) =>
  Schema.Struct({schema_version: Schema.Literal(1), state: Schema.Literal(state)});
// Flat on the wire (`{"schema_version":1,"state":"sent","lines_sent":3,...}`), one struct per state so
// `onExcessProperty: "error"` still rejects a field the state does not carry (LF-15).
// `sent` and `partial` carry the line counts and the id the server logs the rows under; `unreachable` and
// `ready` answer Inspect. `nothing_new` and `no_logs` are answers of the ingest path.
export const LogUploadStatus = Schema.Union([
  logUploadPlain("not_installed"), logUploadPlain("ready"), logUploadPlain("nothing_new"),
  logUploadPlain("no_logs"), logUploadPlain("unreachable"),
  Schema.Struct({
    schema_version: Schema.Literal(1), state: Schema.Literal("sent"),
    lines_sent: Schema.Number, lines_total: Schema.Number, upload_id: Schema.String,
  }),
  Schema.Struct({
    schema_version: Schema.Literal(1), state: Schema.Literal("partial"),
    lines_sent: Schema.Number, lines_total: Schema.Number, upload_id: Schema.String, retry_after_secs: Schema.Number,
  }),
]);
export type LogUploadStatus = typeof LogUploadStatus.Type;
/** One flat code string: a storage code (`"busy"` is the operation gate) or an upload code. */
export type LogUploadError = StorageCode | "collect_failed" | "network";

// `client_maintenance_command` (engine `storage/client_maintenance/`, LX-06).
export type ClientMaintenanceCommand = {command: "inspect"; schema_version: 1} |
  {command: "reset_cache"; schema_version: 1} |
  {command: "reset_all_client_state"; schema_version: 1; confirmed: boolean};
/** One folder the client writes: shown to the player, never sent back. */
const TreeStatus = Schema.Struct({path: Schema.String, exists: Schema.Boolean});
export const ClientMaintenanceStatus = Schema.Struct({
  schema_version: Schema.Literal(1),
  /** `Firesky\SGWGame\Cache.en-US`. */
  cache: TreeStatus,
  /** The whole `Firesky` tree. */
  client_state: TreeStatus,
  /** Whether the reset removed anything; null for `inspect`. */
  removed: Schema.NullOr(Schema.Boolean),
});
export type ClientMaintenanceStatus = typeof ClientMaintenanceStatus.Type;
/** `ClientMaintenanceError`: a storage code or one of its own flat codes. */
export type ClientMaintenanceError = StorageCode | "confirmation_required" | "profile_unavailable" | "unsafe_path";
const clientMaintenanceCodes = new Set(["confirmation_required", "profile_unavailable", "unsafe_path"]);
export function clientMaintenanceFailure(error: unknown): ClientMaintenanceError | "transport" {
  return typeof error === "string" && clientMaintenanceCodes.has(error)
    ? error as ClientMaintenanceError : safeFailure(error).code as StorageCode | "transport";
}
export const decodeClientMaintenanceStatus = (value: unknown) => Schema.decodeUnknownEffect(ClientMaintenanceStatus,
  {onExcessProperty: "error"})(value).pipe(Effect.mapError(() => new BridgeFailure({code: "schema"})));

// `client_changes_command` (engine `storage/client_changes/`, filled by LX-07).
export type ClientChangesCommand = {command: "inspect"; schema_version: 1};
export const ClientChangeRow = Schema.Struct({
  group: Schema.Literals(["setup", "patch", "launch", "server"]), heading: Schema.String,
  title: Schema.String, description: Schema.String,
  status: Schema.Literals(["every_launch", "at_install", "applied", "pending", "on_launch", "off", "during_play", "missing"]),
  status_label: Schema.String, patch_id: Schema.NullOr(Schema.String),
});
export type ClientChangeRow = typeof ClientChangeRow.Type;
export const ClientChangesStatus = Schema.Struct({
  schema_version: Schema.Literal(1), installed: Schema.Boolean, changes: Schema.Array(ClientChangeRow),
  install_record_unreadable: Schema.Boolean,
});
export type ClientChangesStatus = typeof ClientChangesStatus.Type;
export type ClientChangesError = ScaffoldError;

// `physx_command` (engine `storage/runtime_setup/windows_physx/`, LX-12). Native Windows only.
export type PhysxCommand = {command: "inspect" | "install" | "install_legacy"; schema_version: 1};
export const PhysxResult = Schema.Union([
  Schema.Struct({state: Schema.Literal("installed"), restart_required: Schema.Boolean}),
  Schema.Struct({state: Schema.Literal("cancelled_at_uac")}),
  Schema.Struct({state: Schema.Literal("cancelled_in_installer")}),
  Schema.Struct({state: Schema.Literal("failed"), exit_code: Schema.Int}),
  Schema.Struct({state: Schema.Literal("not_detected"), exit_code: Schema.Int}),
  Schema.Struct({state: Schema.Literal("other_version_blocks")}),
  Schema.Struct({state: Schema.Literal("start_failed"), win32_error: Schema.Int}),
]);
export type PhysxResult = typeof PhysxResult.Type;
export const PhysxStatus = Schema.Struct({
  schema_version: Schema.Literal(1),
  supported: Schema.Boolean,
  // `core_present` is `<PhysXCore Path>\v2.6.3\PhysXCore.dll`, the file the game's loader opens.
  detection: Schema.NullOr(Schema.Struct({core_registered: Schema.Boolean, core_present: Schema.Boolean})),
  installer_available: Schema.Boolean,
  installing: Schema.Boolean,
  last_result: Schema.NullOr(PhysxResult),
});
export type PhysxStatus = typeof PhysxStatus.Type;
export type PhysxError = ScaffoldError | "installer_unavailable" | "package_mismatch" | "download_failed";
export const decodePhysxStatus = (value: unknown) =>
  Schema.decodeUnknownEffect(PhysxStatus, {onExcessProperty: "error"})(value);
/** A `physx_command` rejection, or `"transport"` for anything else it could be. */
export function physxFailure(error: unknown): PhysxError | "transport" {
  return error === "installer_unavailable" || error === "package_mismatch" || error === "download_failed" ? error : scaffoldFailure(error);
}

export class BridgeFailure extends Data.TaggedError("BridgeFailure")<{
  readonly code: "transport" | "schema" | "in_use" | "io" | "corrupt" | "unsupported_schema" |
    "too_large" | "unsafe_file" | "invalid_directory" | "stale_revision" | "busy" | "persistence_uncertain" |
    "invalid_server_list" | "invalid_display_options";
}> {}

const nativeCodes = new Set(["in_use", "io", "corrupt", "unsupported_schema", "too_large", "unsafe_file",
  "invalid_directory", "stale_revision", "busy", "persistence_uncertain", "invalid_server_list",
  "invalid_display_options"]);
export function safeFailure(error: unknown): BridgeFailure {
  return new BridgeFailure({code: typeof error === "string" && nativeCodes.has(error)
    ? error as BridgeFailure["code"] : "transport"});
}
/** A scaffold command's rejection, or `"transport"` for anything else it could be. */
export function scaffoldFailure(error: unknown): ScaffoldError | "transport" {
  return error === "unavailable" ? error : safeFailure(error).code as StorageCode | "transport";
}

const logUploadCodes = new Set(["collect_failed", "network"]);
/** A `log_upload_command` rejection, or `"transport"` for anything else it could be. */
export function logUploadFailure(error: unknown): LogUploadError | "transport" {
  return typeof error === "string" && logUploadCodes.has(error)
    ? error as LogUploadError : safeFailure(error).code as StorageCode | "transport";
}
export const decodeLogUploadStatus = (value: unknown) => Schema.decodeUnknownEffect(LogUploadStatus,
  {onExcessProperty: "error"})(value).pipe(Effect.mapError(() => new BridgeFailure({code: "schema"})));

export const decodeSnapshot =(value: unknown) => Schema.decodeUnknownEffect(NativeSnapshot,
  {onExcessProperty: "error"})(value).pipe(Effect.mapError(() => new BridgeFailure({code: "schema"})));

// `seed_archive_command` and `choose_seed_archive` (engine `storage/seed_archive`, AI-06). The archive is
// picked natively; the webview names no path. Command tags are `command`, as the other commands use.
export type SeedArchiveCommand = {command: "inspect" | "cancel" | "forget" | "accept_offer"; schema_version: 1};
export const SeedArchiveErrorCode = Schema.Union([
  Schema.Literal("not_a_file"), Schema.Literal("size_mismatch"), Schema.Literal("hash_mismatch"),
  Schema.Literal("changed"), Schema.Literal("cancelled"), Schema.Literal("busy"), Schema.Literal("io"),
  Schema.Literal("release_unavailable"),
  Schema.Literal("signing_key_unavailable"), Schema.Literal("release_verification_failed"),
]);
export type SeedArchiveError = typeof SeedArchiveErrorCode.Type;
export const SeedArchiveStatus = Schema.Struct({
  schema_version: Schema.Literal(1),
  native: NativeSnapshot,
  chosen: Schema.NullOr(Schema.Struct({file_name: Schema.String, size: Schema.Int})),
  // A file in Downloads with the seed's exact size and name; no folder is sent.
  offer: Schema.NullOr(Schema.Struct({file_name: Schema.String, size: Schema.Int})),
  // Present while a check runs and has reported progress.
  verifying: Schema.NullOr(Schema.Struct({
    phase: Schema.Union([Schema.Literal("download"), Schema.Literal("extraction")]),
    current: Schema.Int, total: Schema.Int,
  })),
  last_error: Schema.NullOr(SeedArchiveErrorCode),
  picker_cancelled: Schema.optionalKey(Schema.Literal(true)),
});
export type SeedArchiveStatus = typeof SeedArchiveStatus.Type;
export const decodeSeedArchiveStatus = (value: unknown) =>
  Schema.decodeUnknownEffect(SeedArchiveStatus, {onExcessProperty: "error"})(value);
