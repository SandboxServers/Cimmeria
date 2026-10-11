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
  "invalid_directory" | "stale_revision" | "busy" | "persistence_uncertain";
/** The error shape the three scaffold commands share (LX-02). */
export type ScaffoldError = "unsupported_schema" | "unavailable" | {storage: StorageCode};

// `log_upload_command` (engine `storage/log_upload/`, filled by LX-05).
export type LogUploadCommand = {command: "inspect"; schema_version: 1} | {command: "upload"; schema_version: 1};
export const LogUploadStatus = Schema.Struct({schema_version: Schema.Literal(1)});
export type LogUploadStatus = typeof LogUploadStatus.Type;
export type LogUploadError = ScaffoldError;

// `client_maintenance_command` (engine `storage/client_maintenance/`, filled by LX-06).
export type ClientMaintenanceCommand = {command: "inspect"; schema_version: 1} |
  {command: "reset_cache"; schema_version: 1} |
  {command: "reset_all_client_state"; schema_version: 1; confirmed: boolean};
export const ClientMaintenanceStatus = Schema.Struct({schema_version: Schema.Literal(1)});
export type ClientMaintenanceStatus = typeof ClientMaintenanceStatus.Type;
export type ClientMaintenanceError = ScaffoldError;

// `client_changes_command` (engine `storage/client_changes/`, filled by LX-07).
export type ClientChangesCommand = {command: "inspect"; schema_version: 1};
export const ClientChangesStatus = Schema.Struct({schema_version: Schema.Literal(1)});
export type ClientChangesStatus = typeof ClientChangesStatus.Type;
export type ClientChangesError = ScaffoldError;

export class BridgeFailure extends Data.TaggedError("BridgeFailure")<{
  readonly code: "transport" | "schema" | "in_use" | "io" | "corrupt" | "unsupported_schema" |
    "too_large" | "unsafe_file" | "invalid_directory" | "stale_revision" | "busy" | "persistence_uncertain";
}> {}

const nativeCodes = new Set(["in_use", "io", "corrupt", "unsupported_schema", "too_large", "unsafe_file",
  "invalid_directory", "stale_revision", "busy", "persistence_uncertain"]);
export function safeFailure(error: unknown): BridgeFailure {
  return new BridgeFailure({code: typeof error === "string" && nativeCodes.has(error)
    ? error as BridgeFailure["code"] : "transport"});
}

export const decodeSnapshot = (value: unknown) => Schema.decodeUnknownEffect(NativeSnapshot,
  {onExcessProperty: "error"})(value).pipe(Effect.mapError(() => new BridgeFailure({code: "schema"})));
