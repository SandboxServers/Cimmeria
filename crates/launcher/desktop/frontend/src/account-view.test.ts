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

/** Mounts the panel; `respond` answers each command after `inspect`, which `inspected` supplies. */
async function mounted(inspected: Record<string, unknown>, respond: (request: Request) => Promise<unknown> = async () => withAccount()) {
 const {document, window} = parseHTML(html);
 const calls: Request[] = [];
 const feed = makePanelContext();
 feed.publish({} as NativeSnapshot);
 const app = mountAccount(document as unknown as Document, async (command, args) => {
  assert.equal(command, 'sign_in_command');
  const request = (args as {request: Request}).request;
  calls.push(request);
  return request.command === 'inspect' ? inspected : respond(request);
 }, feed.context);
 await app.ready;
 await flush();
 const panel = document.getElementById('account-panel')!;
 const button = (label: string) => Array.from(panel.querySelectorAll('button')).find(b => b.textContent === label) as unknown as HTMLButtonElement;
 const press = async (label: string) => { button(label).dispatchEvent(new window.Event('click')); await app.settled(); };
 const text = () => panel.textContent!;
 const statusText = () => panel.querySelector('[role="status"]')!.textContent;
 return {calls, app, document, window, panel, button, press, text, statusText};
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
 const p = await mounted(signedIn(), async request => {
  if (request.command === 'account') return withAccount();
  if (request.command === 'begin') return signedIn({account: account([pilot])});
  if (request.command === 'unlink' && refused) { refused = false; throw {error: 'server', code: 'reauth_required'}; }
  return withAccount([]);
 });
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
 assert.equal(p.panel.querySelectorAll('li').length, 0);
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
 assert.equal(p.panel.querySelectorAll('input').length, 0);
});

test('panel hidden when signed out', async () => {
 const p = await mounted(signedIn());
 assert.equal(p.panel.hidden, false);
 p.document.dispatchEvent(new p.window.CustomEvent('cimmeria-sign-in', {detail: status()}));
 assert.equal(p.panel.hidden, true);
});
