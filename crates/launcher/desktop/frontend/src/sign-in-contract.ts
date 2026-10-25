import {Schema} from 'effect';
/**
 * Decoders for the `sign_in_command` bridge (C13). The engine owns the types; these
 * only check the shape the panel reads. Unknown extra fields are allowed.
 */
export const SignInProvider = Schema.Struct({id: Schema.String, display_name: Schema.String});
export const SignInIdentity = Schema.Struct({
  identity_id: Schema.Number, provider: Schema.String, display_name: Schema.String,
  created_at: Schema.Number, last_used_at: Schema.Number,
});
export const SignInAccount = Schema.Struct({
  account_name: Schema.String, display_name: Schema.String, identities: Schema.Array(SignInIdentity),
});
const SignedOut = Schema.Struct({state: Schema.Literal('signed_out')});
const NeedsLink = Schema.Struct({
  state: Schema.Literal('needs_link'), provider_display_name: Schema.String, identity_display_name: Schema.String,
  hint: Schema.Literals(['none', 'email_match']), signup_open: Schema.Boolean,
});
const SignedIn = Schema.Struct({state: Schema.Literal('signed_in'), account_name: Schema.String, display_name: Schema.String});
export const SignInState = Schema.Union([SignedOut, NeedsLink, SignedIn]);
export const PlayRefusal = Schema.Literals(['ticket_unavailable', 'sign_in_ended']);
export const SignInStatus = Schema.Struct({
  schema_version: Schema.Literal(1),
  available: Schema.Boolean,
  server_name: Schema.NullOr(Schema.String),
  signup_open: Schema.Boolean,
  providers: Schema.Array(SignInProvider),
  waiting: Schema.Boolean,
  state: SignInState,
  account: Schema.NullOr(SignInAccount),
  play_refusal: Schema.NullOr(PlayRefusal),
});
export type SignInProvider = typeof SignInProvider.Type;
export type SignInAccount = typeof SignInAccount.Type;
export type SignInState = typeof SignInState.Type;
export type NeedsLinkState = typeof NeedsLink.Type;
export type SignedInState = typeof SignedIn.Type;
export type PlayRefusal = typeof PlayRefusal.Type;
export type SignInStatus = typeof SignInStatus.Type;

/** A `SignInFailure` (C13): one member per `error`, so each branch narrows its own fields. */
const failureKind = <E extends string>(error: E) =>Schema.Struct({error: Schema.Literal(error)});
export const SignInFailure = Schema.Union([
  failureKind('unsupported_schema'), failureKind('unavailable'), failureKind('busy'),
  Schema.Struct({error: Schema.Literal('browser_failed'), url: Schema.String}),
  failureKind('offline'), failureKind('cancelled'), failureKind('denied'), failureKind('timed_out'),
  failureKind('no_pending_link'), failureKind('password_removal_not_acknowledged'),
  Schema.Struct({error: Schema.Literal('server'), code: Schema.String, retry_after: Schema.optionalKey(Schema.NullOr(Schema.Number))}),
  failureKind('protocol'), failureKind('io'),
]);
export type SignInFailure = typeof SignInFailure.Type;

/** Decodes a `sign_in_command` success; rejects with the decode error when a field is missing. */
export const decodeSignInStatus = (value: unknown) => Schema.decodeUnknownEffect(SignInStatus)(value);

/** A `sign_in_command` rejection as a `SignInFailure`, or `'schema'` when it is not one. */
export function signInFailure(value: unknown): SignInFailure | 'schema' {
  try {
    return Schema.decodeUnknownSync(SignInFailure)(value);
  } catch {
    return 'schema';
  }
}
