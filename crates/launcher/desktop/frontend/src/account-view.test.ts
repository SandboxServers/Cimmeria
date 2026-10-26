import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parseHTML} from 'linkedom';
import type {NativeSnapshot} from './contract';
import {makePanelContext} from './panel-context';
import {mountAccount} from './account-view';

const html = readFileSync('ui/index.html', 'utf8');
type Request = Record<string, unknown>;
const status = (over: Record<string, unknown> = {}) => ({schema_version: 1, available: true, server_name: 'Cimmeria', signup_open: true,
 providers: [{id: 'discord', display_name: 'Discord'}, {id: 'google', display_name: 'Google'}], waiting: false,
 state: {state: 'signed_out'}, account: null, play_refusal: null, ...over});
const signedIn = (over: Record<string, unknown> = {}) => status({state: {state: 'signed_in', account_name: 'Player_1234', display_name: 'Player'}, ...over});
const account = (identities: Record<string, unknown>[]) => ({account_name: 'Player_1234', display_name: 'Player', identities});
const pilot = {identity_id: 7, provider: 'discord', display_name: 'Pilot', created_at: 1700000000000, last_used_at: 1700000100000};
const withAccount = (identities: Record<string, unknown>[] = [pilot]) => signedIn({account: account(identities)});
const flush = () => new Promise<void>(resolve => setImmediate(resolve));
const reauthRemove = 'To remove a sign-in method, sign in again first. Removing is allowed for 10 minutes after a fresh sign-in.';
const reauthAdd = 'To add a sign-in method, sign in again first. Adding is allowed for 10 minutes after a fresh sign-in.';
const lastMethod = 'You cannot remove your last sign-in method. Add another method first. If you lose access to every method, ask the server staff for a link code.';

/**
 * Mounts the panel with no event: the account panel inspects on mount, as it does at startup. `respond` answers
 * each command after `inspect`, which answers `inspected.value` (a holder, so a test can change it later).
 */
async function mounted(first: Record<string, unknown>, respond: (request: Request) => Promise<unknown> = async () => withAccount()) {
 const {document, window} = parseHTML(html);
 const calls: Request[] = [];
 const inspected = {value: first};
 const feed = makePanelContext();
 feed.publish({} as NativeSnapshot);
 const app = mountAccount(document as unknown as Document, async (command, args) => {
  assert.equal(command, 'sign_in_command');
  const request = (args as {request: Request}).request;
  calls.push(request);
  return request.command === 'inspect' ? inspected.value : respond(request);
 }, feed.context);
 await app.ready;
 await flush();
 const panel = document.getElementById('account-panel')!;
 const button = (label: string) => Array.from(panel.querySelectorAll('button')).find(b => b.textContent === label) as unknown as HTMLButtonElement;
 const press = async (label: string) => { button(label).dispatchEvent(new window.Event('click')); await app.settled(); };
 const text = () => panel.textContent!;
 const statusText = () => panel.querySelector('[role="status"]')!.textContent;
 return {calls, app, feed, inspected, document, window, panel, button, press, text, statusText};
}

test('remove needs a confirm press', async () => {
 const p = await mounted(signedIn());
 await p.press('Show linked sign-in methods');
 await p.press('Remove');
 assert.equal(p.calls.some(r => r.command === 'unlink'), false, 'the first press must send nothing');
 assert.match(p.text(), /Remove discord\?/);
 await p.press('Confirm remove');
 const unlink = p.calls.find(r => r.command === 'unlink');
 assert.deepEqual(unlink, {schema_version: 1, command: 'unlink', identity_id: 7});
});

test('reauth_required offers sign in again and retries the removal', async () => {
 let refused = true;
 let statusNow: () => string | null = () => null; // reads the status line, set once the panel is mounted
 const retryStatus: (string | null)[] = [];
 const p = await mounted(signedIn(), async request => {
  if (request.command === 'account') return withAccount();
  if (request.command === 'begin') return signedIn({account: account([pilot])});
  if (request.command === 'unlink' && refused) { refused = false; throw {error: 'server', code: 'reauth_required'}; }
  if (request.command === 'unlink') retryStatus.push(statusNow());
  return withAccount([]);
 });
 statusNow = p.statusText;
 await p.press('Show linked sign-in methods');
 await p.press('Remove');
 await p.press('Confirm remove');
 assert.ok(p.text().includes(reauthRemove));
 assert.equal(p.button('Sign in again').hidden, false);
 await p.press('Sign in again');
 const begin = p.calls.find(r => r.command === 'begin');
 assert.deepEqual(begin, {schema_version: 1, command: 'begin', provider: 'discord'});
 const unlinks = p.calls.filter(r => r.command === 'unlink');
 assert.equal(unlinks.length, 2, 'the refused removal is retried once after the sign-in');
 assert.deepEqual(retryStatus, ['Removing…'], 'the retried removal shows its status');
 assert.equal(p.panel.querySelectorAll('li').length, 0);
});

test('sign in again as another account signs that account out and retries nothing', async () => {
 const otherAccount = 'That is a different account, so you are signed out. Sign in again with the same account.';
 const other = signedIn({state: {state: 'signed_in', account_name: 'Other_9999', display_name: 'Other'}});
 const p = await mounted(signedIn(), async request => {
  if (request.command === 'account') return withAccount();
  if (request.command === 'begin') return other;
  if (request.command === 'unlink') throw {error: 'server', code: 'reauth_required'};
  return status();
 });
 await p.press('Show linked sign-in methods');
 await p.press('Remove');
 await p.press('Confirm remove');
 await p.press('Sign in again');
 assert.deepEqual(p.calls.find(r => r.command === 'sign_out'), {schema_version: 1, command: 'sign_out'});
 assert.equal(p.calls.filter(r => r.command === 'unlink').length, 1, 'the refused removal is not retried as another account');
 assert.equal(p.statusText(), otherAccount);
});

test('reauth_required on add offers sign in again', async () => {
 const p = await mounted(signedIn(), async request => {
  if (request.command === 'add_method') throw {error: 'server', code: 'reauth_required'};
  return withAccount();
 });
 await p.press('Add Google');
 assert.ok(p.text().includes(reauthAdd));
 assert.ok(p.button('Sign in again'));
 assert.equal(p.calls.some(r => r.command === 'add_method' && r.provider === 'google'), true);
});

test('sign in again starts with the provider of the most recently used identity', async () => {
 const p = await mounted(signedIn(), async request => {
  if (request.command === 'account') return withAccount([{...pilot, provider: 'google'}]);
  if (request.command === 'add_method') throw {error: 'server', code: 'reauth_required'};
  return withAccount();
 });
 await p.press('Add Google');
 await p.press('Sign in again');
 const names = p.calls.map(r => r.command);
 const begin = p.calls.find(r => r.command === 'begin');
 assert.deepEqual(begin, {schema_version: 1, command: 'begin', provider: 'google'});
 assert.ok(names.indexOf('account') > -1 && names.indexOf('account') < names.indexOf('begin'), 'the account loads before the provider is chosen');
});

test('every press writes status before the command', async () => {
 let statusNow: () => string | null = () => null;
 const seen: (string | null)[] = [];
 const p = await mounted(signedIn(), async request => {
  if (request.command === 'account' || request.command === 'add_method' || request.command === 'unlink') seen.push(statusNow());
  return withAccount();
 });
 statusNow = p.statusText;
 await p.press('Show linked sign-in methods');
 await p.press('Add Google');
 await p.press('Remove');
 await p.press('Confirm remove');
 assert.deepEqual(seen, ['Loading your account…', 'Opening your browser. Finish signing in there, then come back here.', 'Removing…']);
});

test('the pressed button is aria-busy while its command is pending', async () => {
 const p = await mounted(signedIn(), () => new Promise(() => {}));
 p.button('Show linked sign-in methods').dispatchEvent(new p.window.Event('click'));
 assert.equal(p.button('Show linked sign-in methods').getAttribute('aria-busy'), 'true');
});

test('restored sign-in shows the panel after a snapshot', async () => {
 const p = await mounted(signedIn());
 assert.equal(p.panel.hidden, false, 'a player already signed in sees the panel at startup, with no sign-in event');
 p.inspected.value = status();
 p.feed.publish({} as NativeSnapshot);
 await p.app.settled();
 assert.equal(p.panel.hidden, true);
});

test('last_method text', async () => {
 const p = await mounted(signedIn(), async request => {
  if (request.command === 'account') return withAccount();
  throw {error: 'server', code: 'last_method'};
 });
 await p.press('Show linked sign-in methods');
 await p.press('Remove');
 await p.press('Confirm remove');
 assert.equal(p.statusText(), lastMethod);
});

test('no password line and no password field with the account view', async () => {
 const p = await mounted(signedIn());
 await p.press('Show linked sign-in methods');
 assert.ok(p.text().includes('This account has no password: it plays only through this launcher.'));
 assert.equal(p.panel.querySelectorAll('input[type="password"]').length, 0);
});

test('panel hidden when signed out', async () => {
 const p = await mounted(signedIn());
 assert.equal(p.panel.hidden, false);
 p.document.dispatchEvent(new p.window.CustomEvent('cimmeria-sign-in', {detail: status()}));
 assert.equal(p.panel.hidden, true);
});

test('add method browser_failed shows the address and resumes', async () => {
 let addressWhileWaiting = '';
 let cancelWhileWaiting = false;
 const urlBox = () => (p.panel.querySelector('[readonly]')!.parentNode) as unknown as HTMLElement;
 const p = await mounted(signedIn(), async request => {
  if (request.command === 'add_method') throw {error: 'browser_failed', url: 'https://example.org/auth'};
  if (request.command === 'resume') {
   addressWhileWaiting = (p.panel.querySelector('[readonly]') as unknown as HTMLInputElement).value;
   cancelWhileWaiting = p.button('Cancel').hidden === false;
   return withAccount();
  }
  return withAccount();
 });
 await p.press('Add Google');
 assert.deepEqual(p.calls.slice(-2).map(c => c.command), ['add_method', 'resume']);
 assert.equal(addressWhileWaiting, 'https://example.org/auth');
 assert.equal(cancelWhileWaiting, true, 'Cancel shows while the resume waits');
 assert.equal(urlBox().hidden, true, 'the address box clears once it settles');
});

test('sign in again with no last-used provider says what to do', async () => {
 const p = await mounted(signedIn(), async request => {
  if (request.command === 'add_method') throw {error: 'server', code: 'reauth_required'};
  return withAccount([]);
 });
 await p.press('Add Google');
 await p.press('Sign in again');
 assert.equal(p.statusText(), 'Sign in again from the sign-in panel, then retry.');
 assert.equal(p.calls.some(r => r.command === 'begin'), false, 'nothing is started without a provider');
});

test('sign in again whose begin is cancelled does not retry', async () => {
 const p = await mounted(signedIn(), async request => {
  if (request.command === 'account') return withAccount();
  if (request.command === 'begin') throw {error: 'cancelled'};
  if (request.command === 'unlink') throw {error: 'server', code: 'reauth_required'};
  return withAccount();
 });
 await p.press('Show linked sign-in methods');
 await p.press('Remove');
 await p.press('Confirm remove');
 await p.press('Sign in again');
 assert.equal(p.calls.filter(r => r.command === 'unlink').length, 1, 'the refused removal is not retried');
 assert.equal(p.calls.some(r => r.command === 'add_method'), false);
 assert.equal(p.statusText(), 'Sign-in cancelled.');
});
