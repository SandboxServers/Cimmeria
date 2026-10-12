import { Data, Effect, Schema } from "effect";

const Revision = Schema.Int.check(Schema.isBetween({ minimum: 0, maximum: Number.MAX_SAFE_INTEGER }));
const Operation = Schema.Struct({
  id: Schema.String,
  kind: Schema.Literals(["install", "prepare_runtime", "repair", "uninstall", "launch", "adopt", "update"]),
  intent_digest: Schema.Array(Schema.Int.check(Schema.isBetween({minimum: 0, maximum: 255})))
    .check(Schema.isBetweenLength(32, 32)),
  state: Schema.Literals(["starting", "running", "cancel_requested", "succeeded", "failed", "cancelled", "reconciliation_required"]),
});
/** `cimmeria_launcher_core::client_setup::login_servers::LoginServer`. */
export const LoginServer = Schema.Struct({name: Schema.String, url: Schema.String});
export type LoginServer = typeof LoginServer.Type;
export const NativeSnapshot = Schema.Struct({
  schema_version: Schema.Literal(1),
  operation: Schema.Struct({schema_version: Schema.Literal(1), revision: Revision, operation: Schema.NullOr(Operation)}),
  preferences: Schema.Struct({
    schema_version: Schema.Literal(2), revision: Revision,
    install_directory: Schema.NullOr(Schema.String), launcher_summary_consent: Schema.Boolean,
    // Empty means the built-in default server list.
    login_servers: Schema.Array(LoginServer), client_patches_enabled: Schema.Boolean,
  }),
  requires_reopen: Schema.Boolean,
  // An adopted copy plays with the list its adoption reviewed, whatever is saved above.
  adopted_login_servers: Schema.NullOr(Schema.Array(LoginServer)),
  // The running build (engine `LauncherBuild`); `tag` is null for a development build.
  launcher: Schema.Struct({version: Schema.String, tag: Schema.NullOr(Schema.String)}),
});
export type NativeSnapshot = typeof NativeSnapshot.Type;
export type Preferences = NativeSnapshot["preferences"];
export type Command = {command: "inspect"; schema_version: 1} | {
  command: "save_preferences"; schema_version: 1; expected_revision: number;
  install_directory: string | null; launcher_summary_consent: boolean;
} | {
  command: "save_client_options"; schema_version: 1; expected_revision: number;
  login_servers: readonly LoginServer[]; client_patches_enabled: boolean;
};

// Native storage codes as `StorageError` serializes them.
export type StorageCode = "in_use" | "io" | "corrupt" | "unsupported_schema" | "too_large" | "unsafe_file" |
  "invalid_directory" | "stale_revision" | "busy" | "persistence_uncertain" | "invalid_server_list";
/** The three scaffold commands (LX-02) fail with one flat code string: a storage code or `"unavailable"`. */
export type ScaffoldError = StorageCode | "unavailable";

// `log_upload_command` (engine `storage/log_upload/`, LX-05).
export type LogUploadCommand = {command: "inspect"; schema_version: 1} | {command: "upload"; schema_version: 1};
export const LogUploadStatus = Schema.Struct({
  schema_version: Schema.Literal(1),
  // `disabled`: no log store compiled in. `ready` answers Inspect; the last four answer Upload.
  state: Schema.Literals(["disabled", "not_installed", "ready", "uploaded", "uploaded_newest", "nothing_new", "no_logs"]),
});
export type LogUploadStatus = typeof LogUploadStatus.Type;
/** One flat code string: a storage code (`"busy"` is the operation gate) or an upload code. */
export type LogUploadError = StorageCode | "collect_failed" | "logs_too_large" | "network" | "rejected";

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
export type PhysxCommand = {command: "inspect"; schema_version: 1} | {command: "install"; schema_version: 1};
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
export type PhysxError = ScaffoldError | "installer_unavailable" | "package_mismatch";
export const decodePhysxStatus = (value: unknown) =>
  Schema.decodeUnknownEffect(PhysxStatus, {onExcessProperty: "error"})(value);
/** A `physx_command` rejection, or `"transport"` for anything else it could be. */
export function physxFailure(error: unknown): PhysxError | "transport" {
  return error === "installer_unavailable" || error === "package_mismatch" ? error : scaffoldFailure(error);
}

export class BridgeFailure extends Data.TaggedError("BridgeFailure")<{
  readonly code: "transport" | "schema" | "in_use" | "io" | "corrupt" | "unsupported_schema" |
    "too_large" | "unsafe_file" | "invalid_directory" | "stale_revision" | "busy" | "persistence_uncertain" |
    "invalid_server_list";
}> {}

const nativeCodes = new Set(["in_use", "io", "corrupt", "unsupported_schema", "too_large", "unsafe_file",
  "invalid_directory", "stale_revision", "busy", "persistence_uncertain", "invalid_server_list"]);
export function safeFailure(error: unknown): BridgeFailure {
  return new BridgeFailure({code: typeof error === "string" && nativeCodes.has(error)
    ? error as BridgeFailure["code"] : "transport"});
}
/** A scaffold command's rejection, or `"transport"` for anything else it could be. */
export function scaffoldFailure(error: unknown): ScaffoldError | "transport" {
  return error === "unavailable" ? error : safeFailure(error).code as StorageCode | "transport";
}

const logUploadCodes = new Set(["collect_failed", "logs_too_large", "network", "rejected"]);
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
export type SeedArchiveCommand = {command: "inspect" | "cancel" | "forget"; schema_version: 1};
export const SeedArchiveErrorCode = Schema.Union([
  Schema.Literal("not_a_file"), Schema.Literal("size_mismatch"), Schema.Literal("hash_mismatch"),
  Schema.Literal("changed"), Schema.Literal("cancelled"), Schema.Literal("busy"), Schema.Literal("io"),
  Schema.Literal("release_unavailable"),
]);
export type SeedArchiveError = typeof SeedArchiveErrorCode.Type;
export const SeedArchiveStatus = Schema.Struct({
  schema_version: Schema.Literal(1),
  native: NativeSnapshot,
  chosen: Schema.NullOr(Schema.Struct({file_name: Schema.String, size: Schema.Int})),
  // Present while a check runs and has reported progress.
  verifying: Schema.NullOr(Schema.Struct({
    phase: Schema.Union([Schema.Literal("download"), Schema.Literal("extraction")]),
    current: Schema.Int, total: Schema.Int,
  })),
  last_error: Schema.NullOr(SeedArchiveErrorCode),
});
export type SeedArchiveStatus = typeof SeedArchiveStatus.Type;
export const decodeSeedArchiveStatus = (value: unknown) =>
  Schema.decodeUnknownEffect(SeedArchiveStatus, {onExcessProperty: "error"})(value);
