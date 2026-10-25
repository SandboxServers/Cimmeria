import {Effect} from 'effect';
import type {Invoke} from './view';
import type {PanelContext} from './panel-context';
import {decodeSignInStatus, signInFailure, type SignInFailure, type SignInProvider, type SignInStatus} from './sign-in-contract';
import {failureText, needsLinkText, playRefusalText, signedInText} from './sign-in-workflow';
import {reportUiEvent} from './ui-event';
/**
 * Main-page panel (CT-42, C13): sign in with a provider before Play. It stays hidden on servers
 * without launcher sign-in, so those look unchanged. Every command goes through `sign_in_command`;
 * the password is sent once and never kept in the DOM after a send.
 */
export const linkWarning = "Linking removes this account's password for good. After linking you can play it only through this launcher; a game started without the launcher can no longer log in to it.";
const opening = 'Opening your browser. Finish signing in there, then come back here.';
const decodeReply = (value: unknown) => Effect.runPromise(decodeSignInStatus(value)).catch(() => { throw 'schema'; });

export function mountSignIn(document: Document, invoke: Invoke, context: PanelContext) {
 const panel = document.getElementById('sign-in-panel');
 if (!panel) throw new Error('Missing launcher element sign-in-panel');
 const make = <K extends keyof HTMLElementTagNameMap>(tag: K, text = '') => { const e = document.createElement(tag); e.textContent = text; return e; };
 const button = (text: string) => { const b = make('button', text); b.type = 'button'; return b; };
 const field = (label: string, attrs: Record<string, string>) => {
  const wrap = make('label', label); const input = make('input');
  for (const [name, value] of Object.entries(attrs)) input.setAttribute(name, value);
  wrap.append(input); return {wrap, input};
 };
 const heading = make('h3', 'Sign in'); heading.id = 'sign-in-title';
 const status = make('p'); status.setAttribute('role', 'status'); status.setAttribute('aria-live', 'polite');
 const stateLine = make('p');
 const signupNote = make('p', 'New accounts are closed on this server. You can still sign in and link an existing account.');
 const providers = make('div');
 const cancel = button('Cancel');
 const signOut = button('Sign out');
 const linkBox = make('div');
 const warning = make('p', linkWarning);
 const createNew = button('Create a new account');
 const account = field('Account name', {type: 'text', autocomplete: 'username', maxlength: '20'});
 const password = field('Password', {type: 'password', autocomplete: 'current-password'});
 const linkPassword = button('Link');
 const code = field('Link code', {type: 'text', autocomplete: 'off', maxlength: '10'});
 const codeHint = make('p', 'No password? Ask the server staff for a link code.');
 const linkCode = button('Link with a code');
 const confirmRemove = button('Remove password and link');
 const keepPassword = button('Keep password');
 const urlBox = make('label', 'Copy this address into your browser:');
 const urlField = make('input'); urlField.readOnly = true; urlBox.append(urlField);
 linkBox.append(warning, createNew, account.wrap, password.wrap, linkPassword, code.wrap, codeHint, linkCode, confirmRemove, keepPassword);
 panel.textContent = '';
 panel.append(heading, status, stateLine, signupNote, providers, cancel, linkBox, signOut, urlBox);
 panel.hidden = true;

 let current: SignInStatus | null = null;
 let busy = false; // a command other than cancel is in flight
 let beginning = false; // a provider press waits for the browser
 let linkKind: 'password' | 'code' | null = null; // set once Link is pressed, until confirmed or kept
 let providerKey = '';
 let providerButtons: HTMLButtonElement[] = [];
 let disposed = false;
 let mutation: Promise<void> = Promise.resolve();
 const setStatus = (text: string) => { status.textContent = text; };
 // Never throws synchronously into the caller, whatever the bridge does.
 const call = (request: Record<string, unknown>) => Promise.resolve().then(() => invoke('sign_in_command', {request: {schema_version: 1, ...request}}));

 const render = () => {
  if (disposed) return;
  const s = current;
  panel.hidden = !s?.available;
  if (!s || !s.available) return;
  const st = s.state;
  const out = st.state === 'signed_out';
  heading.textContent = out ? 'Sign in' : st.state === 'needs_link' ? 'Link your account' : 'Your account';
  stateLine.textContent = st.state === 'needs_link' ? needsLinkText(st) : st.state === 'signed_in' ? signedInText(st) : '';
  providers.hidden = !out;
  signupNote.hidden = !out || s.signup_open;
  const key = s.providers.map(p => `${p.id}\n${p.display_name}`).join('\n');
  if (key !== providerKey) {
   providerKey = key;
   providers.textContent = '';
   providerButtons = s.providers.map(p => {
    const b = button(`Sign in with ${p.display_name}`);
    b.addEventListener('click', () => beginPress(p));
    providers.append(b);
    return b;
   });
  }
  for (const b of providerButtons) { b.disabled = busy || beginning; b.setAttribute('aria-busy', String(beginning)); }
  cancel.hidden = !beginning;
  linkBox.hidden = st.state !== 'needs_link';
  createNew.hidden = !s.signup_open;
  const confirming = linkKind !== null;
  linkPassword.hidden = linkCode.hidden = confirming;
  confirmRemove.hidden = keepPassword.hidden = !confirming;
  for (const b of [createNew, linkPassword, linkCode, confirmRemove, keepPassword, signOut]) b.disabled = busy;
  signOut.hidden = st.state !== 'signed_in';
 };

 // Applies a reply that carries the whole sign-in state.
 const settle = (next: SignInStatus) => {
  current = next;
  setStatus(next.play_refusal ? playRefusalText(next.play_refusal) : '');
  render();
  document.dispatchEvent(new CustomEvent('cimmeria-sign-in', {detail: next}));
 };
 // A failed command: the text, and the address to copy for a browser that did not open. Returns the ui code.
 const fail = (failure: unknown, provider?: string): string => {
  const f: SignInFailure | 'schema' = signInFailure(failure);
  setStatus(failureText(f, provider));
  const url = f !== 'schema' && f.error === 'browser_failed' ? f.url : '';
  urlField.value = url;
  urlBox.hidden = !url;
  return f === 'schema' ? 'schema' : f.error;
 };
 // One command and its reply. Reports the outcome by name only; no request field is ever reported.
 const exchange = (request: Record<string, unknown>, provider?: string) => call(request).then(decodeReply).then(
  next => { if (!disposed) { reportUiEvent('sign_in', String(request.command), 'ok', 'info'); settle(next); } },
  failure => { if (!disposed) reportUiEvent('sign_in', String(request.command), fail(failure, provider)); },
 );
 // A press that starts a command: the status text goes first, then the command.
 const send = (request: Record<string, unknown>, pending: string, provider?: string) => {
  if (disposed || busy) return;
  busy = true; setStatus(pending); render();
  mutation = exchange(request, provider).finally(() => { busy = false; beginning = false; render(); });
 };
 const beginPress = (p: SignInProvider) => {
  if (disposed || busy) return;
  beginning = true;
  send({command: 'begin', provider: p.id}, opening, p.display_name);
 };
 const cancelPress = () => {
  if (disposed) return;
  setStatus('Cancelling…');
  mutation = exchange({command: 'cancel'});
 };
 // Link asks first: the warning is only a text until the player confirms the password removal.
 const linkPress = (kind: 'password' | 'code') => {
  if (disposed || busy) return;
  linkKind = kind;
  reportUiEvent('sign_in', 'link_prompt', 'prompted', 'info');
  setStatus('Linking removes the password. Confirm or keep it.');
  render();
 };
 const confirmPress = () => {
  if (disposed || busy || linkKind === null) return;
  const kind = linkKind;
  linkKind = null;
  const request = kind === 'password'
   ? {command: 'link_existing_password', account_name: account.input.value, password: password.input.value, acknowledge_password_removal: true}
   : {command: 'link_existing_code', link_code: code.input.value.toUpperCase(), acknowledge_password_removal: true};
  password.input.value = '';
  send(request, 'Linking your account…');
 };
 const keepPress = () => {
  if (disposed) return;
  linkKind = null;
  password.input.value = '';
  setStatus('');
  render();
 };

 cancel.addEventListener('click', cancelPress);
 signOut.addEventListener('click', () => send({command: 'sign_out'}, 'Signing out…'));
 createNew.addEventListener('click', () => send({command: 'link_new'}, 'Creating your account…'));
 linkPassword.addEventListener('click', () => linkPress('password'));
 linkCode.addEventListener('click', () => linkPress('code'));
 confirmRemove.addEventListener('click', confirmPress);
 keepPassword.addEventListener('click', keepPress);

 // Each snapshot re-inspects, so a sign-in finished elsewhere (or a server switch) shows up.
 const inspect = () => {
  if (disposed || busy) return mutation;
  mutation = call({command: 'inspect'}).then(decodeReply).then(next => {
   if (disposed || busy) return;
   current = next;
   if (next.play_refusal && !status.textContent) setStatus(playRefusalText(next.play_refusal));
   render();
  }, failure => {
   if (!disposed && !busy && current) setStatus(failureText(signInFailure(failure)));
  });
  return mutation;
 };
 const unsubscribe = context.onSnapshot(() => { void inspect(); });
 const ready = context.snapshot() ? inspect() : Promise.resolve();
 return {ready, refresh: inspect, settled: () => mutation, dispose: async () => {
  disposed = true; unsubscribe(); await mutation; panel.textContent = ''; panel.hidden = true;
 }};
}
