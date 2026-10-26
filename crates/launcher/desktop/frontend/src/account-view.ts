import {Effect} from 'effect';
import type {Invoke} from './view';
import type {PanelContext} from './panel-context';
import {decodeSignInStatus, signInFailure, type SignInAccount, type SignInFailure, type SignInProvider, type SignInStatus} from './sign-in-contract';
import {failureText} from './sign-in-workflow';
import {reportUiEvent} from './ui-event';
import {opening} from './sign-in-view';
/**
 * Account panel (CT-43, C6, C13, C15): the linked sign-in methods of a signed-in account. Removing or
 * adding a method needs a fresh sign-in (`reauth_required`); a linked account never shows a password.
 * Every command goes through `sign_in_command`.
 */
export const noPasswordLine = 'This account has no password: it plays only through this launcher.';
const reauthRemove = 'To remove a sign-in method, sign in again first. Removing is allowed for 10 minutes after a fresh sign-in.';
const reauthAdd = 'To add a sign-in method, sign in again first. Adding is allowed for 10 minutes after a fresh sign-in.';
const lastMethod = 'You cannot remove your last sign-in method. Add another method first. If you lose access to every method, ask the server staff for a link code.';
const otherAccount = 'That is a different account, so you are signed out. Sign in again with the same account.';
const decodeReply = (value: unknown) => Effect.runPromise(decodeSignInStatus(value)).catch(() => { throw 'schema'; });
const isServer = (f: SignInFailure | 'schema', code: string) => f !== 'schema' && f.error === 'server' && f.code === code;
const day = (ms: number) => new Date(ms).toISOString().slice(0, 10);

/** A command the engine refused for want of a fresh sign-in, and the request to retry after the sign-in. */
type Reauth = {retry: Record<string, unknown>; add: boolean};

export function mountAccount(document: Document, invoke: Invoke, context: PanelContext) {
 const panel = document.getElementById('account-panel');
 if (!panel) throw new Error('Missing launcher element account-panel');
 const make = <K extends keyof HTMLElementTagNameMap>(tag: K, text = '') => { const e = document.createElement(tag); e.textContent = text; return e; };
 const button = (text: string) => { const b = make('button', text); b.type = 'button'; return b; };
 const heading = make('h3', 'Your sign-in methods'); heading.id = 'account-title';
 const status = make('p'); status.setAttribute('role', 'status'); status.setAttribute('aria-live', 'polite');
 const noPassword = make('p', noPasswordLine);
 const show = button('Show linked sign-in methods');
 const list = make('ul');
 const reauthBox = make('div');
 const reauthText = make('p');
 const signAgain = button('Sign in again');
 const addHeading = make('h4', 'Add a sign-in method');
 const addButtons = make('div');
 const cancel = button('Cancel');
 const urlBox = make('label', 'Copy this address into your browser:');
 const urlField = make('input'); urlField.setAttribute('readonly', ''); urlBox.append(urlField); urlBox.hidden = true;
 reauthBox.append(reauthText, signAgain);
 panel.textContent = '';
 panel.append(heading, status, noPassword, show, list, reauthBox, addHeading, addButtons, cancel, urlBox);
 panel.hidden = true;

 let current: SignInStatus | null = null;
 let account: SignInAccount | null = null; // loaded by `account`, `add_method` and `unlink`
 let accountName = ''; // the signed-in name the loaded account belongs to
 let busy = false; // a command other than cancel is in flight
 let waiting = false; // an add or a sign-in again waits for the browser
 let confirming: number | null = null; // the identity whose row asks for a removal
 let reauth: Reauth | null = null;
 let resumeNext = false; // an add parked its browser flow (browser_failed): send `resume` once the address shows
 let providerKey = '';
 let providerButtons: HTMLButtonElement[] = [];
 let disposed = false;
 let mutation: Promise<void> = Promise.resolve();
 const setStatus = (text: string) => { status.textContent = text; };
 // Never throws synchronously into the caller, whatever the bridge does.
 const call = (request: Record<string, unknown>) => Promise.resolve().then(() => invoke('sign_in_command', {request: {schema_version: 1, ...request}}));
 const signedIn = () => !!current?.available && current.state.state === 'signed_in';
 // The provider of the identity used most recently (the account must be loaded); undefined when there is none.
 const lastUsedProvider = (): string | undefined => {
  const latest = (account?.identities ?? []).reduce<SignInAccount['identities'][number] | undefined>(
   (a, b) => (!a || b.last_used_at > a.last_used_at ? b : a), undefined);
  return latest?.provider;
 };

 const drawList = () => {
  list.textContent = '';
  for (const id of account?.identities ?? []) {
   const row = make('li');
   if (confirming === id.identity_id) {
    const confirmBtn = button('Confirm remove');
    const keep = button('Keep');
    confirmBtn.disabled = busy;
    confirmBtn.addEventListener('click', () => unlinkPress(id.identity_id, id.provider));
    keep.addEventListener('click', () => { confirming = null; drawList(); });
    row.append(make('span', `Remove ${id.provider}?`), confirmBtn, keep);
   } else {
    const remove = button('Remove');
    remove.disabled = busy;
    remove.addEventListener('click', () => { confirming = id.identity_id; drawList(); });
    row.append(make('span', `${id.provider} · ${id.display_name} · added ${day(id.created_at)} · last used ${day(id.last_used_at)}`), remove);
   }
   list.append(row);
  }
 };
 const render = () => {
  if (disposed) return;
  panel.hidden = !signedIn();
  if (!signedIn()) return;
  show.disabled = busy;
  show.setAttribute('aria-busy', String(busy));
  drawList();
  reauthBox.hidden = !reauth;
  if (reauth) reauthText.textContent = reauth.add ? reauthAdd : reauthRemove;
  signAgain.disabled = busy;
  signAgain.setAttribute('aria-busy', String(busy));
  const providers: readonly SignInProvider[] = current?.providers ?? [];
  const key = providers.map(p => `${p.id}\n${p.display_name}`).join('\n');
  if (key !== providerKey) {
   providerKey = key;
   addButtons.textContent = '';
   providerButtons = providers.map(p => {
    const b = button(`Add ${p.display_name}`);
    b.addEventListener('click', () => addPress(p));
    addButtons.append(b);
    return b;
   });
  }
  for (const b of providerButtons) { b.disabled = busy; b.setAttribute('aria-busy', String(waiting)); }
  cancel.hidden = !waiting;
 };

 // Takes a reply that carries the sign-in state; a different account drops the loaded list.
 const adopt = (next: SignInStatus) => {
  const name = next.state.state === 'signed_in' ? next.state.account_name : '';
  if (name !== accountName) { accountName = name; account = null; confirming = null; reauth = null; }
  if (next.account) account = next.account;
  current = next;
  render();
  return next;
 };
 const notify = (next: SignInStatus) => {
  document.dispatchEvent(new (document.defaultView?.CustomEvent ?? CustomEvent)('cimmeria-sign-in', {detail: next}));
 };
 // A failed command: reauth_required offers Sign in again; other failures show their text.
 const fail = (request: Record<string, unknown>, failure: unknown, provider?: string) => {
  const f = signInFailure(failure);
  const command = String(request.command);
  reportUiEvent('sign_in', command, f === 'schema' ? 'schema' : f.error);
  if (isServer(f, 'reauth_required') && command !== 'begin' && command !== 'resume') {
   reauth = {retry: request, add: command === 'add_method'};
   setStatus('');
  } else if (command !== 'begin' && command !== 'resume') {
   reauth = null;
   setStatus(isServer(f, 'last_method') ? lastMethod : failureText(f, provider));
  } else {
   setStatus(failureText(f, provider));
  }
  if ((command === 'add_method' || command === 'begin') && f !== 'schema' && f.error === 'browser_failed') {
   resumeNext = true;
   urlField.value = f.url;
   urlBox.hidden = false;
  }
  render();
 };
 // One command and its reply. Returns the new state, or null when the command failed (already shown).
 const exchange = (request: Record<string, unknown>, provider?: string): Promise<SignInStatus | null> => call(request).then(decodeReply).then(
  next => {
   if (disposed) return null;
   reportUiEvent('sign_in', String(request.command), 'ok', 'info');
   if (request.command !== 'begin') reauth = null;
   if (request.command === 'begin') notify(next);
   setStatus('');
   return adopt(next);
  },
  failure => { if (!disposed) fail(request, failure, provider); return null; },
 );
 // A press that starts a command: the status text goes first, then the command. One at a time.
 const guard = (pending: string, task: () => Promise<unknown>, wait = false) => {
  if (disposed || busy) return;
  busy = true; waiting = wait; setStatus(pending); render();
  mutation = task().finally(() => { busy = false; waiting = false; render(); }).then(() => undefined);
 };
 // An add whose browser did not open waits on the parked flow: `resume` completes it, and the address box clears after.
 const addPress = (p: SignInProvider) => guard(opening, async () => {
  await exchange({command: 'add_method', provider: p.id}, p.display_name);
  if (!resumeNext) return;
  resumeNext = false;
  await exchange({command: 'resume'});
  urlField.value = '';
  urlBox.hidden = true;
 }, true);
 const unlinkPress = (id: number, provider: string) => {
  confirming = null;
  guard('Removing…', () => exchange({command: 'unlink', identity_id: id}, provider));
 };
 // Sign in again with the most recently used provider, then retry the refused command once.
 const signAgainPress = () => {
  if (!reauth || disposed || busy) return;
  const {retry} = reauth;
  const who = accountName;
  guard(opening, async () => {
   if (!account && !(await exchange({command: 'account'}))) return;
   const providerId = lastUsedProvider();
   if (!providerId) { setStatus('Sign in again from the sign-in panel, then retry.'); return; }
   const name = current?.providers.find(p => p.id === providerId)?.display_name ?? providerId;
   let next = await exchange({command: 'begin', provider: providerId}, name);
   if (!next && resumeNext) {
    resumeNext = false;
    next = await exchange({command: 'resume'});
    urlField.value = '';
    urlBox.hidden = true;
    if (next) notify(next);
   }
   if (!next) return;
   if (next.state.state !== 'signed_in' || next.state.account_name !== who) {
    // The engine already holds the other account: sign it out so Play cannot run as it.
    reauth = null;
    if (next.state.state === 'signed_in') await exchange({command: 'sign_out'});
    setStatus(otherAccount);
    return;
   }
   reauth = null;
   setStatus(retry.command === 'unlink' ? 'Removing…' : opening);
   await exchange(retry);
  }, true);
 };
 const cancelPress = () => {
  if (disposed) return;
  setStatus('Cancelling…');
  mutation = exchange({command: 'cancel'}).then(() => undefined);
 };

 show.addEventListener('click', () => guard('Loading your account…', () => exchange({command: 'account'})));
 signAgain.addEventListener('click', signAgainPress);
 cancel.addEventListener('click', cancelPress);

 const inspect = () => {
  if (disposed || busy) return mutation;
  mutation = call({command: 'inspect'}).then(decodeReply).then(next => { if (!disposed && !busy) adopt(next); }, () => undefined);
  return mutation;
 };
 const onSignIn = (event: Event) => { if (!disposed) adopt((event as CustomEvent<SignInStatus>).detail); };
 document.addEventListener('cimmeria-sign-in', onSignIn);
 // Each snapshot re-inspects (as the sign-in panel does), so a restored sign-in or a server switch shows up; the sign-in panel's own events keep it in step between inspects.
 const unsubscribe = context.onSnapshot(() => { void inspect(); });
 const ready = context.snapshot() ? inspect() : Promise.resolve();
 return {ready, refresh: inspect, settled: () => mutation, dispose: async () => {
  disposed = true; unsubscribe(); document.removeEventListener('cimmeria-sign-in', onSignIn); await mutation;
  panel.textContent = ''; panel.hidden = true;
 }};
}
