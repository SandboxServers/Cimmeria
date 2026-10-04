import { Data, Effect, Schema } from "effect";

const Revision = Schema.Int.check(Schema.isBetween({ minimum: 0, maximum: Number.MAX_SAFE_INTEGER }));
const Operation = Schema.Struct({
  id: Schema.String,
  kind: Schema.Literals(["install", "prepare_runtime", "repair", "uninstall", "launch", "adopt", "update"]),
  intent_digest: Schema.Array(Schema.Int.check(Schema.isBetween({minimum: 0, maximum: 255})))
    .check(Schema.isBetweenLength(32, 32)),
  state: Schema.Literals(["starting", "running", "cancel_requested", "succeeded", "failed", "cancelled", "reconciliation_required"]),
});
export const NativeSnapshot = Schema.Struct({
  schema_version: Schema.Literal(1),
  operation: Schema.Struct({schema_version: Schema.Literal(1), revision: Revision, operation: Schema.NullOr(Operation)}),
  preferences: Schema.Struct({
    schema_version: Schema.Literal(1), revision: Revision,
    install_directory: Schema.NullOr(Schema.String), launcher_summary_consent: Schema.Boolean,
  }),
  requires_reopen: Schema.Boolean,
});
export type NativeSnapshot = typeof NativeSnapshot.Type;
export type Preferences = NativeSnapshot["preferences"];
export type Command = {command: "inspect"; schema_version: 1} | {
  command: "save_preferences"; schema_version: 1; expected_revision: number;
  install_directory: string | null; launcher_summary_consent: boolean;
};

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
