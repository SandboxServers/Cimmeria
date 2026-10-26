import test from 'node:test';
import assert from 'node:assert/strict';
import {Effect} from 'effect';
import {decodeSignInStatus, signInFailure} from './sign-in-contract';
import {failureText, needsLinkText, playRefusalText, signedInText, storageWarningText} from './sign-in-workflow';

const status = {schema_version: 1, available: true, server_name: 'Cimmeria', signup_open: true, providers: [],
 waiting: false, state: {state: 'signed_out'}, account: null, play_refusal: null};

test('failure texts cover every C13 error', () => {
 const texts: [unknown, string][] = [
  [{error: 'cancelled'}, 'Sign-in cancelled.'],
  [{error: 'denied'}, 'Discord did not allow the sign-in. Try again, or pick another method.'],
  [{error: 'timed_out'}, 'Sign-in timed out after 5 minutes. Try again.'],
  [{error: 'offline'}, 'The server could not be reached. Check your connection and try again.'],
  [{error: 'unavailable'}, 'This server does not offer launcher sign-in. Press Play and log in with your account name and password.'],
  [{error: 'busy'}, 'A sign-in is already waiting in your browser. Finish it there, or press Cancel.'],
  [{error: 'browser_failed', url: 'https://example.org/auth'}, 'Your browser could not be opened. Copy this address into your browser:'],
  [{error: 'no_pending_link'}, 'That sign-in expired. Sign in again.'],
  [{error: 'password_removal_not_acknowledged'}, "Confirm that linking removes this account's password, then press Link again."],
  [{error: 'protocol'}, 'The server answered in an unexpected way. Try again later.'],
  [{error: 'io'}, 'The launcher answered in an unexpected format. Restart it.'],
  [{error: 'unsupported_schema'}, 'The launcher answered in an unexpected format. Restart it.'],
  [{error: 'server', code: 'invalid_credentials'}, 'That account name and password do not match.'],
  [{error: 'server', code: 'not_linkable'}, 'That account cannot be linked to a sign-in method. Ask the server staff.'],
  [{error: 'server', code: 'signup_closed'}, 'This server is not accepting new accounts right now. Link an existing account instead.'],
  [{error: 'server', code: 'rate_limited', retry_after: 30}, 'Too many attempts. Try again in 30 seconds.'],
  [{error: 'server', code: 'rate_limited', retry_after: null}, 'Too many attempts. Try again in a few minutes.'],
  [{error: 'server', code: 'account_disabled'}, 'This account is suspended. Contact the server staff.'],
  [{error: 'server', code: 'signed_out'}, 'Your sign-in has ended. Sign in again.'],
  [{error: 'server', code: 'unknown_provider'}, 'That sign-in method is no longer offered. Pick another.'],
  [{error: 'server', code: 'reauth_required'}, 'Sign-in failed (reauth_required). Try again.'],
  [{error: 'nonsense'}, 'The launcher answered in an unexpected format. Restart it.'],
 ];
 for (const [value, text] of texts) {
  const f = signInFailure(value);
  assert.equal(failureText(f, 'Discord'), text, JSON.stringify(value));
 }
 assert.equal(failureText('schema'), 'The launcher answered in an unexpected format. Restart it.');
});

test('decoders reject a status missing state', async () => {
 const {state: _state, ...missing} = status;
 await assert.rejects(Effect.runPromise(decodeSignInStatus(missing)));
 assert.equal(signInFailure({error: 'browser_failed'}), 'schema');
 await Effect.runPromise(decodeSignInStatus({...status, extra_field: true}));
});

test('workflow decodes storage_warning and no_pending_flow', async () => {
 const warned = await Effect.runPromise(decodeSignInStatus({...status, storage_warning: 'store_unreadable'}));
 assert.equal(warned.storage_warning, 'store_unreadable');
 await assert.rejects(Effect.runPromise(decodeSignInStatus({...status, storage_warning: 'nonsense'})));
 const expired = signInFailure({error: 'no_pending_flow'});
 assert.deepEqual(expired, {error: 'no_pending_flow'});
 assert.equal(failureText(expired), 'That sign-in expired. Sign in again.');
 assert.equal(storageWarningText('session_only'), "Signed in for this session only: this computer's credential store would not save the sign-in.");
 assert.equal(storageWarningText('sign_out_incomplete'), "Signed out here, but the saved sign-in could not be removed from this computer's credential store. It will not be used again in this session.");
 assert.equal(storageWarningText('store_unreadable'), "This computer's credential store could not be read, so no saved sign-in was restored.");
});

test('play refusal, signed-in and needs-link texts', () => {
 assert.equal(playRefusalText('ticket_unavailable'), 'Play stopped: could not get a sign-in for this launch. Check your connection and press Play again. If it keeps failing, sign out and sign in again.');
 assert.equal(playRefusalText('sign_in_ended'), 'Play stopped: your sign-in has ended. Sign in again, then press Play.');
 assert.equal(signedInText({state: 'signed_in', account_name: 'Player_1234', display_name: 'Player'}), 'Signed in as Player (Player_1234). Play uses this account.');
 const link = {state: 'needs_link' as const, provider_display_name: 'Discord', identity_display_name: 'Pilot', hint: 'none' as const, signup_open: true};
 assert.equal(needsLinkText(link), 'Signed in with Discord as Pilot. This sign-in is not linked to a game account yet.');
 assert.equal(needsLinkText({...link, hint: 'email_match'}), 'Signed in with Discord as Pilot. This sign-in is not linked to a game account yet. An account with the same email address exists. If it is yours, link it below.');
});
