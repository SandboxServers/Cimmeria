import type {NeedsLinkState, PlayRefusal, SignedInState, SignInFailure} from './sign-in-contract';
/**
 * Pure sign-in texts (CT-42). The panel in `sign-in-view.ts` shows these; nothing here
 * touches the document or the bridge.
 */
const unexpected = 'The launcher answered in an unexpected format. Restart it.';

/** The status line for a failed `sign_in_command`; `provider` is the display name of the provider pressed. */
export function failureText(f: SignInFailure | 'schema', provider?: string): string {
  if (f === 'schema') return unexpected;
  switch (f.error) {
    case 'cancelled': return 'Sign-in cancelled.';
    case 'denied': return `${provider ?? 'The provider'} did not allow the sign-in. Try again, or pick another method.`;
    case 'timed_out': return 'Sign-in timed out after 5 minutes. Try again.';
    case 'offline': return 'The server could not be reached. Check your connection and try again.';
    case 'unavailable': return 'This server does not offer launcher sign-in. Press Play and log in with your account name and password.';
    case 'busy': return 'A sign-in is already waiting in your browser. Finish it there, or press Cancel.';
    case 'browser_failed': return 'Your browser could not be opened. Copy this address into your browser:';
    case 'no_pending_link': return 'That sign-in expired. Sign in again.';
    case 'password_removal_not_acknowledged': return "Confirm that linking removes this account's password, then press Link again.";
    case 'protocol': return 'The server answered in an unexpected way. Try again later.';
    case 'io':
    case 'unsupported_schema': return unexpected;
    case 'server': return serverText(f.code, f.retry_after);
  }
}

function serverText(code: string, retryAfter: number | null | undefined): string {
  switch (code) {
    case 'invalid_credentials': return 'That account name and password do not match.';
    case 'not_linkable': return 'That account cannot be linked to a sign-in method. Ask the server staff.';
    case 'signup_closed': return 'This server is not accepting new accounts right now. Link an existing account instead.';
    case 'rate_limited': return retryAfter != null ? `Too many attempts. Try again in ${retryAfter} seconds.` : 'Too many attempts. Try again in a few minutes.';
    case 'account_disabled': return 'This account is suspended. Contact the server staff.';
    case 'signed_out': return 'Your sign-in has ended. Sign in again.';
    case 'unknown_provider': return 'That sign-in method is no longer offered. Pick another.';
    default: return `Sign-in failed (${code}). Try again.`;
  }
}

/** The line shown when Play stopped for a reason (C13 `play_refusal`). */
export function playRefusalText(r: PlayRefusal): string {
  return r === 'ticket_unavailable'
    ? 'Play stopped: could not get a sign-in for this launch. Check your connection and press Play again. If it keeps failing, sign out and sign in again.'
    : 'Play stopped: your sign-in has ended. Sign in again, then press Play.';
}

export function signedInText(s: SignedInState): string {
  return `Signed in as ${s.display_name} (${s.account_name}). Play uses this account.`;
}

export function needsLinkText(s: NeedsLinkState): string {
  const base = `Signed in with ${s.provider_display_name} as ${s.identity_display_name}. This sign-in is not linked to a game account yet.`;
  return s.hint === 'email_match' ? `${base} An account with the same email address exists. If it is yours, link it below.` : base;
}
