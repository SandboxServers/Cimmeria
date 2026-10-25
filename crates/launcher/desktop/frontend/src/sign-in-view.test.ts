import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {parseHTML} from 'linkedom';
import type {NativeSnapshot} from './contract';
import {makePanelContext} from './panel-context';
import {mountSignIn} from './sign-in-view';
import {playRefusalText} from './sign-in-workflow';

const html = readFileSync('ui/index.html', 'utf8');
type Request = Record<string, unknown>;
const status = (over: Record<string, unknown> = {}) => ({schema_version: 1, available: true, server_name: 'Cimmeria', signup_open: true,
 providers: [{id: 'discord', display_name: 'Discord'}], waiting: false, state: {state: 'signed_out'}, account: null, play_refusal: null, ...over});
const needsLink = (over: Record<string, unknown> = {}) => status({state: {state: 'needs_link', provider_display_name: 'Discord',
 identity_display_name: 'Pilot', hint: 'none', signup_open: true}, ...over});
const flush = () => new Promise<void>(resolve => setImmediate(resolve));

/** Mounts the panel; `respond` answers each request after `inspect` (which `inspected` supplies). */
async function mounted(inspected: Record<string, unknown>, respond: (request: Request) => Promise<unknown> = async () => status()) {
 const {document, window} = parseHTML(html);
 const calls: Request[] = [];
 const feed = makePanelContext();
 feed.publish({} as NativeSnapshot);
 const app = mountSignIn(document as unknown as Document, async (command, args) => {
  assert.equal(command, 'sign_in_command');
  const request = (args as {request: Request}).request;
  calls.push(request);
  return request.command === 'inspect' ? inspected : respond(request);
 }, feed.context);
 await app.ready;
 await flush();
 const button = (label: string) => Array.from(document.querySelectorAll('button')).find(b => b.textContent === label) as unknown as HTMLButtonElement;
 const field = (id: string) => document.querySelector(`#sign-in-panel ${id}`) as unknown as HTMLInputElement;
 const press = (el: HTMLElement) => el.dispatchEvent(new window.Event('click'));
 const statusText = () => document.querySelector('#sign-in-panel [role="status"]')!.textContent;
 return {calls, app, document, button, field, press, statusText, feed};
}

test('unavailable hides the panel', async () => {
 const p = await mounted(status({available: false, providers: []}));
 try {
  assert.equal((p.document.getElementById('sign-in-panel') as unknown as HTMLElement).hidden, true);
  assert.deepEqual(p.calls.map(c => c.command), ['inspect']);
 } finally { await p.app.dispose(); }
});

test('unavailable with a play_refusal shows the refusal and no controls', async () => {
 const p = await mounted(status({available: false, play_refusal: 'ticket_unavailable'}));
 try {
  assert.equal((p.document.getElementById('sign-in-panel') as unknown as HTMLElement).hidden, false);
  assert.equal(p.statusText(), playRefusalText('ticket_unavailable'));
  assert.equal((p.document.getElementById('sign-in-title') as unknown as HTMLElement).hidden, true);
  for (const label of ['Cancel', 'Sign out']) assert.equal((p.button(label) as unknown as HTMLElement).hidden, true, label);
 } finally { await p.app.dispose(); }
});

test('the panel comes back after an offline snapshot', async () => {
 const reply = status({available: false, play_refusal: 'ticket_unavailable'});
 const p = await mounted(reply);
 try {
  Object.assign(reply, status({state: {state: 'signed_in', account_name: 'pilot', display_name: 'Pilot'}}));
  p.feed.publish({} as NativeSnapshot);
  await flush();
  assert.equal((p.document.getElementById('sign-in-title') as unknown as HTMLElement).hidden, false);
  const line = Array.from(p.document.querySelectorAll('#sign-in-panel p')).find(el => !el.getAttribute('role')) as unknown as HTMLElement;
  assert.equal(line.hidden, false);
  assert.notEqual(line.textContent, '');
 } finally { await p.app.dispose(); }
});

test('a status with no login server name still decodes', async () => {
 const p = await mounted(status({server_name: null, providers: [{id: 'discord', display_name: 'Discord'}]}));
 try {
  assert.equal((p.button('Sign in with Discord').parentNode as unknown as HTMLElement).hidden, false);
 } finally { await p.app.dispose(); }
});

test('provider press shows feedback before invoke', async () => {
 let statusAtBegin = '';
 let release!: (value: unknown) => void;
 const p = await mounted(status(), request => {
  if (request.command !== 'begin') return Promise.resolve(status());
  statusAtBegin = p.statusText() ?? '';
  return new Promise(resolve => { release = resolve; });
 });
 try {
  p.press(p.button('Sign in with Discord'));
  await flush();
  assert.equal(statusAtBegin, 'Opening your browser. Finish signing in there, then come back here.');
  assert.equal(p.button('Sign in with Discord').disabled, true);
  assert.equal(p.button('Sign in with Discord').getAttribute('aria-busy'), 'true');
  assert.equal(p.button('Cancel').hidden, false);
  release(status({waiting: false}));
  await p.app.settled();
 } finally { await p.app.dispose(); }
});

test('needs_link shows create only when signup is open', async () => {
 const open = await mounted(needsLink());
 try {
  assert.equal(open.button('Create a new account').hidden, false);
 } finally { await open.app.dispose(); }
 const closed = await mounted(needsLink({signup_open: false, state: {state: 'needs_link', provider_display_name: 'Discord',
  identity_display_name: 'Pilot', hint: 'none', signup_open: false}}));
 try {
  assert.equal(closed.button('Create a new account').hidden, true);
 } finally { await closed.app.dispose(); }
});

test('password box is cleared after a failed link', async () => {
 const p = await mounted(needsLink(), async request => {
  if (request.command === 'link_existing_password') throw {error: 'server', code: 'invalid_credentials'};
  return status();
 });
 try {
  p.field('input[type="password"]').value = 'hunter2';
  p.press(p.button('Link'));
  p.press(p.button('Remove password and link'));
  await p.app.settled();
  assert.equal(p.field('input[type="password"]').value, '');
  assert.equal(p.statusText(), 'That account name and password do not match.');
 } finally { await p.app.dispose(); }
});

test('link warns and needs a confirm press', async () => {
 const p = await mounted(needsLink(), async () => needsLink());
 try {
  assert.match(p.document.querySelector('#sign-in-panel')!.textContent!, /Linking removes this account's password for good/);
  p.field('input[autocomplete="username"]').value = 'Player_1234';
  p.field('input[type="password"]').value = 'hunter2';
  p.press(p.button('Link'));
  await flush();
  assert.equal(p.calls.length, 1, 'Link sends nothing');
  p.press(p.button('Keep password'));
  await flush();
  assert.equal(p.calls.length, 1, 'Keep password sends nothing');
  assert.equal(p.field('input[type="password"]').value, '');
  p.field('input[type="password"]').value = 'hunter2';
  p.press(p.button('Link'));
  p.press(p.button('Remove password and link'));
  await flush();
  assert.deepEqual(p.calls[1], {schema_version: 1, command: 'link_existing_password', account_name: 'Player_1234',
   password: 'hunter2', acknowledge_password_removal: true});
  await p.app.settled();
  p.field('input[maxlength="10"]').value = 'abc123';
  p.press(p.button('Link with a code'));
  await flush();
  assert.equal(p.calls.length, 2, 'Link with a code sends nothing');
  p.press(p.button('Keep password'));
  await flush();
  assert.equal(p.calls.length, 2, 'Keep password sends nothing');
  p.press(p.button('Link with a code'));
  p.press(p.button('Remove password and link'));
  await p.app.settled();
  assert.deepEqual(p.calls[2], {schema_version: 1, command: 'link_existing_code', link_code: 'ABC123', acknowledge_password_removal: true});
 } finally { await p.app.dispose(); }
});

test('play refusal is shown', async () => {
 const p = await mounted(status({play_refusal: 'ticket_unavailable'}));
 try {
  assert.equal(p.statusText(), 'Play stopped: could not get a sign-in for this launch. Check your connection and press Play again. If it keeps failing, sign out and sign in again.');
 } finally { await p.app.dispose(); }
});

test('browser_failed shows the address', async () => {
 const p = await mounted(status(), async request => {
  if (request.command === 'begin') throw {error: 'browser_failed', url: 'https://example.org/auth'};
  return status();
 });
 try {
  p.press(p.button('Sign in with Discord'));
  await p.app.settled();
  const field = p.field('[readonly]');
  assert.equal(field.value, 'https://example.org/auth');
  assert.equal(p.statusText(), 'Your browser could not be opened. Copy this address into your browser:');
 } finally { await p.app.dispose(); }
});

test('a final link failure re-inspects and offers sign-in again', async () => {
 const seen = needsLink();
 const p = await mounted(seen, async request => {
  if (request.command === 'link_new') {
   Object.assign(seen, status());
   throw {error: 'server', code: 'signup_closed'};
  }
  return status();
 });
 try {
  p.press(p.button('Create a new account'));
  await p.app.settled();
  assert.equal(p.calls.filter(c => c.command === 'inspect').length, 2);
  assert.equal(p.button('Sign in with Discord').parentElement!.hidden, false);
  assert.equal(p.statusText(), 'This server is not accepting new accounts right now. Link an existing account instead.');
 } finally { await p.app.dispose(); }
});
