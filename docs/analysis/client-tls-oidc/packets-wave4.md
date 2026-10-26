# client-tls-oidc wave 4: launcher sign-in UI, GM `.account`, docs

> **Type:** Reference
> **Audience:** Packet-coders and packet-reviewers of wave 4
> **Last updated:** 2026-10-25
> **Companions:** [Dispatch rules, rules R1-R10 and checks](work-packets.md), [contract](contract.md), [wave 3](packets-wave3.md)

Seven small packets, one PR from `client-tls-oidc/wave4` (worktree `cto-w4`), based on main after wave 3 merged. Merge order: CT-44, CT-45, CT-40, CT-41, CT-42, CT-43, CT-46.

Start CT-44, CT-40 and CT-42 at once from the wave base. CT-45 branches at CT-44's reviewed head; CT-41 at CT-40's; CT-43 at CT-42's. CT-46 (docs) starts at the integration head after the other six merged in. Every packet carries R1-R9. The desktop-workspace notes at the top of [wave 3](packets-wave3.md) (Cargo.lock, `deny.toml`) apply to CT-40 and CT-41.

Frontend packets (CT-42, CT-43) follow the panel pattern of `crates/launcher/desktop/frontend/src/physx-view.ts`: `import {Effect} from 'effect'`, `type Invoke` from `./view`, `type PanelContext` from `./panel-context`, `reportUiEvent` from `./ui-event`; a never-throwing `call` wrapper (`Promise.resolve().then(() => invoke(...))`); `Effect.runPromise(decode(value))` with a `'schema'` failure; `aria-busy` and `disabled` on a pressed button; a `role="status"` `aria-live="polite"` line; `context.onSnapshot` re-inspects; the mount returns `{ready, refresh, settled, dispose}`. Every press writes its status text **before** the `invoke` (first-press feedback). Add each new `*.test.ts` to the explicit file list in the `test` script of `frontend/package.json`. Write `index.html` changes with the Edit tool; the main checkout's copy may carry someone's uncommitted edits, so re-grep anchors in your worktree.

## CT-40 Engine sign-in driver

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-40-engine-driver` · **Worktree:** `cto-40`
- **Commit:** `feat(launcher): sign-in command driver for the shell (CT-40)`
- **Rules:** R1-R9. Contract: C6, C9, C11, [C13](contract.md#c13-launcher-sign-in-bridge-sign_in_command), C15 (a, d), D-CT21.

Files (all under `crates/launcher/desktop/engine/src/`):

1. `sign_in/commands.rs`: C13 `SignInCommand` (`Deserialize`), `SignInStatus`, `SignInStateView`, `AccountPanelView`, `SignInFailure` (`Serialize`; `#[serde(tag = "error", rename_all = "snake_case")]`), exactly as C13. `impl From<SignInError> for SignInFailure`: `Offline`, `Cancelled`, `Denied`, `TimedOut`, `Protocol` map by name; `Unsupported` to `Unavailable`; `Server(text)` to `Server { code, retry_after }`. Read how CT-34 put `retry_after` into the text (`session.rs`, the 429 branch) and parse exactly that format; add `pub fn retry_after(&self) -> Option<u64>` and `pub fn code(&self) -> &str` to `SignInError` in `sign_in/mod.rs` so the parsing lives next to the formatting.
2. `sign_in/lease.rs`: `pub static ACTIVE_RELAYS: AtomicUsize`; `pub fn active_relays() -> usize`; `pub struct RelayLease(&'static AtomicUsize)` with `pub fn on(counter: &'static AtomicUsize) -> RelayLease` (increments) and `Drop` (decrements). `storage/launch/sign_in_play.rs`: add a `lease: RelayLease` field to the `RelayPassword` and `Ticket` variants, created with `RelayLease::on(&ACTIVE_RELAYS)` right after `relay::start` succeeds; fix any pattern in `sign_in_play.rs` or its tests that names every field (use `..`).
3. `storage/sign_in_state.rs`: `pub fn sign_in_login_server(&self) -> Option<LoginServer>`: the first entry of the same list `launch_login_servers` (`storage/launch/preparation.rs`) uses when no installation is adopted: preferences' `login_servers`, else `login_servers::default_servers()`; `None` when that list is empty.
4. `sign_in/driver.rs`:

```rust
pub trait Browser: Send + Sync { fn open(&self, url: &str) -> Result<(), ()>; }
pub struct SignInDriver { /* store, http, discovery: Mutex<DiscoveryCache>, pending: Mutex<Option<PendingLink>>, waiting: Mutex<Option<oneshot::Sender<()>>> */ }
impl SignInDriver {
    pub fn new(store: Arc<dyn CredentialStore>) -> Self;          // http: discovery::client() (CT-33: rustls, TLS 1.2, no redirects, 10 s)
    pub async fn run(&self, state: &Mutex<DesktopState>, request: SignInCommand, browser: &dyn Browser)
        -> Result<SignInStatus, SignInFailure>;
}
pub fn default_store() -> Arc<dyn CredentialStore>; // KeyringStore on Windows and macOS, else MemoryStore
```

`run`, in this order. Every `DesktopState` lock is its own statement; never hold a `std::sync::Mutex` guard across an `.await`.

1. `schema_version != 1`: `UnsupportedSchema`.
2. `let server = state.lock().map_err(|_| SignInFailure::Io)?.sign_in_login_server();`. `None`: `inspect` and `cancel` return a status with `available: false`; every other command `Unavailable`.
3. Discovery through the cache (`Instant::now()` read here). `None`: as step 2, but first, when `state.lock()?.sign_in()` is `None` and `discovery::auth_base(&server.url)` is `Some(base)`, `SignIn::restore(base, http, store)` and, when `Some`, `set_sign_in(Some(arc))`; no network call (ruling CT-35-1: Play then refuses a signed-in player with `ticket_unavailable` instead of showing the password screen when the launcher started offline).
4. Session: `state.lock()?.sign_in()` when its base equals `discovery.auth_base`; else `SignIn::restore(base, http, store)` and, when `Some`, `set_sign_in(Some(arc))`. If CT-34 gave `SignIn` no signed-out constructor, add `pub fn new(base: Url, http: reqwest::Client, store: Arc<dyn CredentialStore>) -> SignIn` to `sign_in/session.rs` (state empty, nothing stored).
5. Commands:
   - Every status carries `play_refusal` from `state.lock()?.play_refusal()` (C13).
   - `inspect`: status. `state` is `signed_in` when the session has an account summary or a stored entry, `needs_link` when `pending` is live (10 min), else `signed_out`.
   - `begin { provider }`: provider not in `discovery.providers`: `Server { code: "unknown_provider" }`. `waiting` already `Some`: `Busy`. `oauth::begin(&base, &provider)`; `browser.open(&flow.authorize_url)` error: `BrowserFailed { url }`; info event `launcher.sign_in.started` (`outcome`, `reason` only, C11); store the cancel sender; `tokio::select!` over `flow.finish()` and the cancel receiver (cancel gives `Cancelled`); clear `waiting` on every path; `session.complete(grant)`: `SignedIn` sets the session into state and clears `pending`; `NeedsLink(view)` stores `pending`.
   - `cancel`: take `waiting` and send; return the status.
   - `link_existing_password`, `link_existing_code` with `acknowledge_password_removal: false`: `PasswordRemovalNotAcknowledged` before anything else (`pending` kept, no request sent; C15 a).
   - `link_new`, `link_existing_password`, `link_existing_code`: take `pending` (missing or older than 10 min: `NoPendingLink`); call the session method; on success set the session and clear `pending`; on `Server` codes `invalid_credentials`, `rate_limited`, `not_linkable` put `pending` back so the player can retry; drop it on any other failure.
   - `account`: no signed-in session: `Server { code: "signed_out" }`; else `session.account()`, status with `account` filled.
   - `add_method { provider }`: signed-in session required; `session.add_method(&provider)` gives a `BrowserFlow`; then the same open, wait, cancel and `complete` steps as `begin`; then `account()`.
   - `unlink { identity_id }`: `session.unlink(id)`, then `account()`.
   - `sign_out`: `session.sign_out()` (works offline), `set_sign_in(None)`, clear `pending`; status `signed_out`.
6. `sign_in/mod.rs`: `pub mod commands; pub mod driver; pub mod lease;`.

Tests (`sign_in/driver_tests.rs`; `wiremock` stands in for `/auth/v1`; `MemoryStore`; a fake `Browser` that records the URL and, on a spawned task, GETs the callback `redirect_uri?code=c&state=<state from the URL>`; a `DesktopState` opened on a temp dir with preferences whose first login server is the wiremock host, base overridden through the debug-only `CIMMERIA_LAUNCHER_AUTH_BASE` path CT-33 made, or a test seam on the cache if CT-33 added one):

| Test | Asserts | Fails on revert because |
|---|---|---|
| `commands_deserialize_and_refuse_unknown_fields` | each C13 command parses; an extra field and `schema_version: 2` fail | C13 is the bridge contract |
| `inspect_without_sign_in_server_is_unavailable` | `/providers` 404: `available: false`, empty providers | Play keeps working on servers without sign-in |
| `offline_inspect_still_restores_the_session` | `/providers` 404 and a stored entry for the server's host: `available: false`, and `DesktopState::sign_in()` is `Some` | C15 (d): Play knows the player is signed in when discovery failed (ruling CT-35-1) |
| `begin_signs_in_and_stores_the_session` | `state.signed_in`; `DesktopState::sign_in()` is `Some`; store holds the refresh token | new |
| `begin_needs_link_then_link_new` | first `needs_link` with `hint`; then `link_new` gives `signed_in` | new |
| `wrong_password_keeps_the_pending_link` | `link_existing_password` 401 gives `Server { code: "invalid_credentials" }`; a second call with the right body succeeds | the player can retry without a new browser trip |
| `link_without_ack_sends_nothing` | `acknowledge_password_removal: false`: `PasswordRemovalNotAcknowledged`, wiremock saw no `/link/existing` request, `pending` still live; with `true` the request body carries `"password_removal_ack": true` | C15 (a): the warning cannot be skipped |
| `status_carries_play_refusal` | after `set_play_refusal(Some("ticket_unavailable"))`, `inspect` reports it | C15 (d): the panel names why Play stopped |
| `second_begin_is_busy_and_cancel_ends_the_first` | a browser that never calls back: second `begin` is `Busy`; `cancel` makes the first return `Cancelled` | one browser flow at a time; Cancel works |
| `browser_failure_returns_the_url` | `BrowserFailed { url }` starting with the auth base | Wine and locked-down hosts can copy the address |
| `rate_limited_carries_retry_after` | 429 `retry_after: 30`: `Server { code: "rate_limited", retry_after: Some(30) }` | the UI can say how long to wait |
| `sign_out_clears_state_offline` | server down: `signed_out`, `sign_in()` is `None`, store empty | the player can always sign out |
| `relay_lease_counts` | `RelayLease::on(&LOCAL)` with a test-local static: 1 while held, 0 after drop | the close guard reads this count |

Checks: D1, D2 `cimmeria-launcher-engine`, D3 `cimmeria-launcher-engine sign_in`, D3 `cimmeria-launcher-engine sign_in_play`.

## CT-41 Shell: `sign_in_command` and the close guard

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-41-shell` · **Worktree:** `cto-41` (at CT-40's reviewed head)
- **Commit:** `feat(launcher): sign-in command and keep-running prompt (CT-41)`
- **Rules:** R1-R9. Contract: C13, D-CT8.

Files (under `crates/launcher/desktop/shell/src/`):

1. `host.rs` (385 lines): `mod sign_in;` next to the other `mod` lines (make it `pub(crate) mod sign_in;` like `ui_event`); one field `sign_in: cimmeria_launcher_engine::sign_in::driver::SignInDriver` in `NativeHost`, set in `new()` to `SignInDriver::new(driver::default_store())`. Building the store touches no keyring until a command runs.
2. New `host/sign_in.rs`:
   - `struct SystemBrowser;` implementing `Browser` with `tauri_plugin_opener::open_url(url, None::<&str>)` (`tauri_plugin_opener::reveal_item_in_dir` is already used in `main.rs`; no plugin init is needed for these free functions). Map the error to `Err(())`; log `warn!(reason = "browser_open_failed")` with no URL.
   - `#[tauri::command] pub(crate) async fn sign_in_command(request: SignInCommand, state: tauri::State<'_, Arc<NativeHost>>) -> Result<SignInStatus, SignInFailure>`: `let host = state.inner().clone();` get the `Arc<Mutex<DesktopState>>` with `host.store()` inside `tauri::async_runtime::spawn_blocking` (it may wait up to 30 s on first open), mapping errors to `SignInFailure::Io`; then `host.sign_in.run(&desktop, request, &SystemBrowser).await`. `store()` is private to `host.rs`; call it from the child module (`super::NativeHost` methods are visible to child modules) or add a `pub(super)` wrapper if the compiler refuses.
   - `pub(crate) fn hold_close(active_relays: usize) -> bool { active_relays > 0 }`.
   - `pub(crate) fn close_guard(window: &tauri::Window, event: &tauri::WindowEvent)`: on `WindowEvent::CloseRequested { api, .. }` when `hold_close(lease::active_relays())`: `api.prevent_close()`, then a warning message dialog through `tauri_plugin_dialog::DialogExt` (copy the builder calls from `adoption_picker.rs`): title `Game still signed in`, text `The game is signed in through this launcher. If you close it now, the game cannot change server or log in again until you press Play.`, buttons `OkCancelCustom("Keep running", "Close anyway")`; on `Close anyway` call `window.destroy()`. Log `info!(event = "launcher.relay.close_prompt", outcome = "kept"|"closed")`.
3. `main.rs` (634 lines): one handler line `host::sign_in::sign_in_command,` in `generate_handler!` (l.609-631), and `.on_window_event(host::sign_in::close_guard)` on the builder before `.invoke_handler(...)`. At most 3 new lines; never past 700.

Tests (`#[cfg(test)] mod tests` in `host/sign_in.rs`):

| Test | Asserts | Fails on revert because |
|---|---|---|
| `hold_close_only_with_active_relays` | `false` at 0, `true` at 1 | the guard must not nag when no game is relayed |
| `unsupported_schema_needs_no_state` | `NativeHost::new(tempdir).sign_in.run(..)` with `schema_version: 2` gives `UnsupportedSchema` before any store or network use | bridge contract |
| `failure_serializes_with_error_tag` | `serde_json::to_value(SignInFailure::Server { code: "rate_limited", retry_after: Some(5) })` equals the C13 JSON | the frontend decodes this shape |

Checks: D1, D2 `cimmeria-launcher-desktop`, D3 `cimmeria-launcher-desktop sign_in`, then the shell's full suite once: D3 `cimmeria-launcher-desktop` (no filter).

## CT-42 Frontend: the sign-in panel

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-42-frontend-sign-in` · **Worktree:** `cto-42`
- **Commit:** `feat(launcher-ui): sign in with a provider before Play (CT-42)`
- **Rules:** R1-R9. Contract: C13 (only the JSON; this packet compiles no Rust), C15 (a, d), D-CT11.

Files (under `crates/launcher/desktop/frontend/`):

1. `src/sign-in-contract.ts`: `effect` `Schema` decoders for C13 `SignInStatus` and `SignInFailure` (`decodeSignInStatus`, `signInFailure(value): SignInFailure | 'schema'`), in their own file (`contract.ts` is near 200 lines). Unknown extra fields are allowed; a missing required field is `'schema'`.
2. `src/sign-in-workflow.ts`, pure functions only:
   - `failureText(f: SignInFailure, provider?: string): string` with exactly these texts: `cancelled` "Sign-in cancelled."; `denied` "{Provider} did not allow the sign-in. Try again, or pick another method."; `timed_out` "Sign-in timed out after 5 minutes. Try again."; `offline` "The server could not be reached. Check your connection and try again."; `unavailable` "This server does not offer launcher sign-in. Press Play and log in with your account name and password."; `busy` "A sign-in is already waiting in your browser. Finish it there, or press Cancel."; `browser_failed` "Your browser could not be opened. Copy this address into your browser:"; `no_pending_link` "That sign-in expired. Sign in again."; `password_removal_not_acknowledged` "Confirm that linking removes this account's password, then press Link again."; `protocol` "The server answered in an unexpected way. Try again later."; `io`, `unsupported_schema` and `'schema'` "The launcher answered in an unexpected format. Restart it."; `server` by code: `invalid_credentials` "That account name and password do not match."; `not_linkable` "That account cannot be linked to a sign-in method. Ask the server staff."; `signup_closed` "This server is not accepting new accounts right now. Link an existing account instead."; `rate_limited` "Too many attempts. Try again in {n} seconds." (or "Try again in a few minutes." without `retry_after`); `account_disabled` "This account is suspended. Contact the server staff."; `signed_out` "Your sign-in has ended. Sign in again."; `unknown_provider` "That sign-in method is no longer offered. Pick another."; any other code "Sign-in failed ({code}). Try again."
   - `playRefusalText(r)`: the two C13 `play_refusal` texts, exactly.
   - `signedInText(s)`: "Signed in as {display_name} ({account_name}). Play uses this account."; `needsLinkText(s)`: "Signed in with {provider_display_name} as {identity_display_name}. This sign-in is not linked to a game account yet." plus, for `hint: "email_match"`, " An account with the same email address exists. If it is yours, link it below."
3. `src/sign-in-view.ts`: `mountSignIn(document, invoke, context)` filling `#sign-in-panel`:
   - `available: false`: panel hidden; nothing else changes (plain Play still works).
   - `signed_out`: heading "Sign in"; one button per provider "Sign in with {display_name}"; when `signup_open` is false the line "New accounts are closed on this server. You can still sign in and link an existing account."
   - A provider press: status "Opening your browser. Finish signing in there, then come back here." before `invoke`; provider buttons `disabled` and `aria-busy`; a "Cancel" button appears and sends `cancel` (its own status "Cancelling…").
   - `needs_link`: `needsLinkText`; "Create a new account" (only when `signup_open`) sends `link_new`; a form "Link an existing account" with account name (`autocomplete="username"`, max 20) and password (`type="password"`, `autocomplete="current-password"`) and a "Link" button; a "Link with a code" input (max 10, upper-cased on send) and button, with the line "No password? Ask the server staff for a link code." Above both forms, always visible, the warning (D-CT11): "Linking removes this account's password for good. After linking you can play it only through this launcher; a game started without the launcher can no longer log in to it." Pressing "Link" or the code button sends nothing yet: it swaps the buttons for "Remove password and link" and "Keep password" (status "Linking removes the password. Confirm or keep it." first). "Remove password and link" sends `link_existing_password` or `link_existing_code` with `acknowledge_password_removal: true`; "Keep password" restores the form and sends nothing. The password box is cleared after every send, success or not, and on "Keep password".
   - `play_refusal` set (any state): `playRefusalText` in the panel's `role="status"` line, above the state text.
   - `signed_in`: `signedInText` and a "Sign out" button (status "Signing out…"; sends `sign_out`).
   - Failures: `failureText`; for `browser_failed` also a read-only `<input>` holding `url` for copying.
   - After each successful reply, `document.dispatchEvent(new CustomEvent('cimmeria-sign-in', {detail: status}))` (CT-43 listens).
   - `reportUiEvent('sign_in', <action>, <outcome>)` per press, never with field values.
4. `ui/index.html`: `<section id="sign-in-panel" class="preference-card" aria-labelledby="sign-in-title" hidden></section>` inside `<main id="play">`, directly before the `<footer aria-label="Game actions">`.
5. `src/view.ts`: `import {mountSignIn} from './sign-in-view'` and `mountSignIn(document, invoke, panelFeed.context),` next to `mountServerList(...)`.
6. `package.json`: add `src/sign-in-workflow.test.ts src/sign-in-view.test.ts` to the `test` list.

Tests (`node:test`, `linkedom`, a fake `invoke` that records requests; copy the harness of `src/physx-view.test.ts`):

| Test | Asserts | Fails on revert because |
|---|---|---|
| `failure texts cover every C13 error` | each `error` and each listed `server` code gives its exact text; `rate_limited` with 30 names 30 seconds | the player always learns why |
| `decoders reject a status missing state` | `'schema'` | bridge contract |
| `provider press shows feedback before invoke` | status text set when the fake `invoke` is first called | first-press feedback rule |
| `needs_link shows create only when signup is open` | button present/absent | D-CT5 kill switch |
| `password box is cleared after a failed link` | input value `''` after an `invalid_credentials` reply | the password never lingers in the DOM |
| `link warns and needs a confirm press` | the warning text is present in `needs_link`; "Link" sends nothing; "Remove password and link" sends `link_existing_password` with `acknowledge_password_removal: true`; "Keep password" sends nothing; same for the code form | D-CT11 (a): the password removal is never a surprise |
| `play refusal is shown` | a status with `play_refusal: "ticket_unavailable"` shows its exact text | D-CT11 (d): Play stops with a reason, not a password prompt |
| `unavailable hides the panel` | `hidden` | servers without sign-in look unchanged |
| `browser_failed shows the address` | a read-only input holds the URL | Wine and locked-down hosts |

Checks: F1.

## CT-43 Frontend: account panel and privacy disclosure

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-43-frontend-account` · **Worktree:** `cto-43` (at CT-42's reviewed head)
- **Commit:** `feat(launcher-ui): linked sign-in methods and sign-in privacy notes (CT-43)`
- **Rules:** R1-R9. Contract: C5 (what the server stores), C6 (`account`, `identities`), C13, C15 (e), D-CT9, D-CT11.

Files (under `crates/launcher/desktop/` unless named):

1. `frontend/src/account-view.ts`: `mountAccount(document, invoke, context)` filling `#account-panel`. Hidden unless the last `cimmeria-sign-in` event (or its own `inspect` at mount) says `signed_in`.
   - A "Show linked sign-in methods" button sends `account` (status "Loading your account…" first).
   - The list: one row per identity, "{provider} · {display_name} · added {created_at date} · last used {last_used_at date}", each with "Remove". "Remove" swaps the row's buttons for "Remove {provider}?" with "Confirm remove" and "Keep"; "Confirm remove" sends `unlink` ("Removing…").
   - The fixed line (D-CT11): "This account has no password: it plays only through this launcher."
   - `reauth_required`: "To remove a sign-in method, sign in again first. Removing is allowed for 10 minutes after a fresh sign-in." plus a "Sign in again" button sending `begin` with the provider of the most recently used identity, with the CT-42 browser feedback text.
   - `last_method`: "You cannot remove your last sign-in method. Add another method first. If you lose access to every method, ask the server staff for a link code."
   - "Add a sign-in method": one button per provider from the last status ("Add {display_name}") sending `add_method` with the CT-42 browser feedback text and the Cancel button behaviour.
   - `reauth_required` from `add_method`: "To add a sign-in method, sign in again first. Adding is allowed for 10 minutes after a fresh sign-in." with the same "Sign in again" button as Remove (it sends `begin` with the provider of the most recently used identity).
   - Other failures: CT-42 `failureText`.
2. `frontend/ui/index.html`:
   - `<section id="account-panel" class="preference-card" aria-labelledby="account-title" hidden></section>` directly after `<section id="server-list-panel" ...>`.
   - Inside `<details id="diagnostics-privacy">`, before `<details id="privacy-fields">`: `<details id="privacy-sign-in"><summary>Launcher sign-in and linked accounts</summary>` with one or two `<p>` (no `<a>`, `<button>` or `<input>`: the existing test allows exactly one link in the whole disclosure) saying: when you sign in, the launcher keeps a refresh token, your account name and display name in your operating system's credential store (Windows Credential Manager or the macOS Keychain), never in launcher files or logs; sign-in requests go only to the first login server's HTTPS address; the server stores which provider you used, that provider's account id, a display name, and, only for providers the server operator marks as verifying email, your verified email address, used to match accounts with the same email; linking with a password sends it once over HTTPS and removes it from the account; signing out deletes the stored token; none of this goes to diagnostics.
   - Inside `privacy-fields`: `<p data-privacy-schema="Stored">` listing `refresh_token`, `account_name`, `display_name` as `<code data-field="x">x</code>`, and one `<p data-privacy-schema="<Name>">` for every `pub struct` in `engine/src/sign_in/api.rs` whose name ends in `Request`, listing its fields the same way.
3. `frontend/src/privacy-disclosure.test.ts`: add `Stored: 'crates/launcher/desktop/engine/src/sign_in/store.rs'` and each `*Request` struct (path `.../sign_in/api.rs`) to `schemas`; add `'sign-in'` to the `sections` list; add to the identity test: `privacy-sign-in` matches `/operating system's credential store/`, `/only for providers the server operator marks as verifying email/` and `/none of this goes to diagnostics/`.
4. `frontend/src/view.ts`: `mountAccount(document, invoke, panelFeed.context),` after `mountSignIn`.
5. `docs/privacy-inventory.md` (`crates/launcher/desktop/docs/`, CRLF, R5): rows in its existing table format for the credential-store entry (what, where, when deleted) and for the sign-in requests (what is sent, to which host, when).
6. `frontend/package.json`: add `src/account-view.test.ts`.

Tests (`frontend/src/account-view.test.ts`, plus the privacy test changes):

| Test | Asserts | Fails on revert because |
|---|---|---|
| `remove needs a confirm press` | first press sends nothing; "Confirm remove" sends `unlink` with the id | an unlink cannot be undone |
| `reauth_required offers sign in again` | text and a button that sends `begin` with the last-used provider | the 10-minute rule must be explainable |
| `reauth_required on add offers sign in again` | `add_method` answering `reauth_required` shows the Add text and the button | adding needs a fresh sign-in (W3-1) |
| `last_method text` | exact text | recovery rule (C6) |
| `no password line` | the fixed D-CT11 line is shown with the account view | the player learns they depend on the launcher |
| `panel hidden when signed out` | `hidden` after a `cimmeria-sign-in` event with `signed_out` | new |
| privacy schema tests for `Stored` and each `*Request` | every field disclosed | exhaustive disclosure rule |

Checks: F1.

## CT-44 GM `.account`: wire message and cell console

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-44-gm-cell` · **Worktree:** `cto-44`
- **Commit:** `feat(console): .account identities, unlink and linkcode on the cell (CT-44)`
- **Rules:** R1-R9. Contract: C10.

Copy the `.bankdump` pattern exactly: `crates/cell-console/src/cell/console/bank.rs` `dump` (l.63-134), `crates/wire/src/cell/messages/bank_cell_to_base.rs`, `registry/commands/bank.rs`, the `"bankdump"` arm in `dispatch.rs` (l.401).

Files:

1. New `crates/wire/src/cell/messages/account_cell_to_base.rs`:

```rust
/// The GM `.account` commands (client-tls-oidc C10). The cell has no DB pool, so it forwards these to the base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountCellToBase {
    Identities { entity_id: u32, account_id: Option<u32>, account_name: String },
    Unlink { entity_id: u32, account_id: Option<u32>, account_name: String, identity_id: i64 },
    LinkCode { entity_id: u32, account_id: Option<u32>, account_name: String },
}
```

   `entity_id` and `account_id` are the GM's (from `space_mgr.player_identity(caller_id)`); `account_name` is the target account the GM typed.
2. `crates/wire/src/cell/messages/mod.rs`: `mod account_cell_to_base;` and `pub use account_cell_to_base::AccountCellToBase;` beside the bank lines (l.81-82); one doc bullet beside l.15. `cell_to_base.rs`: `Account(AccountCellToBase),` beside `Bank(BankCellToBase)` (l.591) with a one-line doc; `cell_to_base_kind.rs`: `Self::Account { .. } => "account",` beside l.77. `cell_to_base.rs` is 622 lines; add at most 4.
3. New `crates/cell-console/src/cell/console/account.rs`: module doc (what `.account` does, C10, GM-only, codes are printed once and never logged); `pub(super) async fn run(caller_id: u32, args: &[&str], tx: &mpsc::Sender<CellToBaseMsg>, space_mgr: &SpaceManager)`:
   - Parse `identities <name>`, `unlink <name> <identity_id>`, `linkcode <name>` (sub-command ASCII case-insensitive). Anything else: feedback `usage: .account identities <account> | unlink <account> <identity_id> | linkcode <account>`.
   - `<name>` must match `^[A-Za-z0-9_-]{3,20}$` (check by hand, no regex crate): else `account: not a valid account name`. `<identity_id>` must parse as `i64 > 0`: else `account: identity id must be a positive number`.
   - Send `CellToBaseMsg::Account(..)`. A send error logs `warn!(target: "account_admin", event = "gm_action", action = "account_<sub>", result = "refused", reason = "base_channel_closed", account_id = id.account_id, entity_id = caller_id, ..)` like `bank.rs` l.115-131. Never log the typed name of the target or anything the base answers.
   - `pub(crate) fn is_account_command(text: &str) -> bool` and `pub(crate) fn log_non_gm_account(entity_id, space_mgr)` (`info!`, `reason = "not_gm"`), as `is_bankdump_command` and `log_non_gm_bankdump` (l.137-160).
4. `crates/cell-console/src/cell/console/mod.rs`: `mod account;` (beside `mod bank;`, l.89) and one doc bullet. `dispatch.rs`: `"account" => account::run(caller_id, args, tx, space_mgr).await,` beside l.401, and `account` in the `use` list at l.14. `chat/mod.rs`: where it calls `log_non_gm_bankdump` (near l.121), add the same call shape for `is_account_command` / `log_non_gm_account`.
5. New `registry/commands/account.rs`: `SPECS = &[spec("account", 2, 3, Target::None, "Launcher sign-in methods of an account: list, remove one, or issue a one-time link code (identities <account> | unlink <account> <identity_id> | linkcode <account>)")]`; `registry/commands/mod.rs`: `mod account;` and `account::SPECS,` in the list. If a registry test counts commands or checks help text width, update that number or fit the text.
6. New `crates/cell-console/src/cell/console/tests/ct44_account.rs`, registered in `tests/mod.rs` (622 lines; one line), copying `bv04_bankdump.rs`'s harness.

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `account_identities_forwards_to_base` | the base channel receives `Account(Identities { entity_id, account_id, account_name: "Player_1234" })` | the cell has no pool; C10 |
| `account_unlink_parses_identity_id` | `Unlink { identity_id: 42, .. }` | C10 |
| `account_linkcode_forwards` | `LinkCode { .. }` | C10 |
| `account_bad_name_and_usage_stay_on_cell` | `a`, `x y`, a 21-char name and a bad sub-command answer feedback and send nothing to the base | GM typos never reach the DB |
| `account_kind_label` | `CellToBaseMsg::Account(..).kind()` (or the kind function `cell_to_base_kind.rs` exposes) is `"account"` | the base logs the kind |
| `is_account_command_matches_word_only` | `.account x` true, `.accountx` false | non-GM refusal logging |

Checks: S1 `cimmeria-wire`, S1 `cimmeria-cell-console`, S2 `cimmeria-cell-console`, S2 `cimmeria-wire`. If the new `CellToBaseMsg` variant breaks an exhaustive `match` in another crate (the base dispatcher), add a temporary arm there only if CT-45 has not merged: `CellToBaseMsg::Account(_) => {}` with a `// CT-45 routes this` comment, and run S1 on that crate too. Name the crate in the report.

## CT-45 GM `.account`: base handler

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-45-gm-base` · **Worktree:** `cto-45` (at CT-44's reviewed head)
- **Commit:** `feat(base): answer .account from the identity tables (CT-45)`
- **Rules:** R1-R9. Contract: C5, C6 (`last_method`), C7 (`linkable`), C10, C11, C15 (b, e).

The base and the auth service run in one `cimmeria-server` process (`crates/services/src/orchestrator.rs` builds `AuthService`, `BaseService` and `CellService`), so `cimmeria_identity::link_code::shared()` is the same store the `link/existing` route reads.

Files:

1. `crates/base-session/Cargo.toml`: `cimmeria-identity = { workspace = true }` (add the `[workspace.dependencies]` row in the root `Cargo.toml` if CT-11 did not). Run S4 afterwards (hakari).
2. New `crates/base-session/src/base/account_admin/mod.rs` (`pub mod account_admin;` in `base/mod.rs`, 411 lines):

```rust
pub struct AdminCaller { pub entity_id: u32, pub account_id: Option<u32> }
pub enum AdminAction { Identities, Unlink { identity_id: i64 }, LinkCode }
pub async fn run(caller: &AdminCaller, target: &str, action: AdminAction,
                 pool: Option<&PgPool>, codes: &LinkCodeStore, now: std::time::Instant) -> Vec<String>;
pub async fn handle(caller: AdminCaller, target: String, action: AdminAction, pool: Option<&PgPool>,
                    transport: &Arc<dyn Transport>, connected: &.., entity_to_addr: &..);
```

   `handle` copies `bank_dump::handle_gm_dump` (l.307): `run(.., link_code::shared(), Instant::now())`, then each line through `send_gm_feedback_to_client`. `run`:
   - No pool: `["account: the database is not available"]`, `warn!` `reason = "db_unavailable"`.
   - `repo::account::facts_by_name(pool, target)` `None`: `["account: no account named <target>"]`.
   - `Identities`: header `account <name> (id <account_id>): <n> sign-in methods; password <set|none>` (`repo::account::has_password`), then per `repo::identity::list_for_account` row `  #<identity_id> <provider> "<display_name>" added <YYYY-MM-DD> last used <YYYY-MM-DD>`, or `  (no linked sign-in methods)`. Display names pass a local `sanitize` (printable ASCII 0x20-0x7E, `"` dropped, max 32 chars).
   - `Unlink`: `repo::identity::count_methods <= 1`: `account: refusing to remove the last sign-in method of <name>`; `repo::identity::delete(pool, account_id, identity_id)` false: `account: identity #<id> is not linked to <name>`; success: `repo::event::record(.., Some(identity_id), EventKind::Unlink, Some(provider))`, `repo::refresh::revoke_account(pool, account_id, now_utc)` (a wrongly linked identity must lose its stored sign-ins), line `account: removed identity #<id> (<provider>) from <name>; its launcher sign-ins are revoked`. Read the provider from the `list_for_account` row before deleting.
   - `LinkCode`: `link::linkable(&facts)` false: `account: <name> is a staff, test or disabled account and cannot be linked (not_linkable)`; else `codes.issue(account_id, now)`, `repo::event::record(.., None, EventKind::LinkCodeIssued, None)`, lines `account: link code for <name>: <CODE>` and `account: valid 15 minutes, single use, replaces any older code. Give it to the player privately; they enter it in the launcher under Link with a code.`, plus, when `repo::account::has_password` is true, `account: <name> has a password; linking with this code removes it for good (D-CT11).` A code is also the recovery step for a player who lost every sign-in method (C15 e).
   - Every outcome logs one `info!` (refusals `warn!` only for `db_unavailable`) with `target: "account_admin"`, `event = "gm_action"`, `action = "account_identities"|"account_unlink"|"account_linkcode"`, `result = "ok"|"refused"`, `reason`, `gm_account_id`, `entity_id`, `target_account_id`, `identity_id` where known. Never the code, a subject, an email or a display name (C11).
3. New `crates/base-world-entry/src/base/world_entry/cell_dispatch/account_dispatch.rs`: `pub(super) async fn route(msg: AccountCellToBase, ctx: &DispatchCtx<'_>)` mapping the three variants to `account_admin::handle(AdminCaller { entity_id, account_id }, account_name, action, ctx.db_pool, ctx.transport, ctx.connected, ctx.entity_to_addr)`, imported the way `bank_dispatch.rs` imports `bank_dump`. `cell_dispatch/mod.rs`: `mod account_dispatch;` (beside l.54) and `CellToBaseMsg::Account(msg) => account_dispatch::route(msg, &ctx).await,` beside l.315; remove CT-44's temporary arm if it added one.
4. `crates/server/src/logging/filters.rs`: if it lists `"bank"` (or other GM targets) explicitly, add `"account_admin"` the same way; otherwise leave it.
5. Tests: `crates/base-session/src/base/account_admin/tests_live_db.rs` (`#[cfg(test)] #[path]`), sentinel accounts `2_000_701..=2_000_712` inserted and cleaned up the way CT-15's `repo` live-DB tests do, subjects `ct45-<n>`, emails `p<n>@example.org`, each test with its own `LinkCodeStore::default()`. Dispatch arm: `cell_dispatch/tests_dispatch_arms/account_arm.rs` (registered in that folder's `mod.rs`), copying `bank_arm.rs` l.12-54 with no pool.

| Test | Asserts | Fails on revert because |
|---|---|---|
| `live_db_identities_lists_without_subjects_or_emails` | lines name provider and display name; no line contains `ct45-` or `example.org` | C10, R7 |
| `live_db_unlink_removes_and_revokes` | two identities; unlink one: row gone, `unlink` event row, every refresh row of the account revoked | a wrong link loses its sign-ins |
| `live_db_unlink_refuses_the_last_method` | one identity: refusal line, row kept | C15 (e) |
| `live_db_linkcode_warns_when_the_account_has_a_password` | an account with a password: the removal line is present; without one: absent | C15 (b) |
| `live_db_unlink_other_accounts_identity_refused` | identity of account B named with account A: refusal, row kept | GM typo safety |
| `live_db_linkcode_refuses_staff_and_test_accounts` | `accesslevel 1` and `test_account true`: `not_linkable`; the store redeems nothing | D-CT6 |
| `live_db_linkcode_issues_a_redeemable_code` | the code line holds 10 chars of the C6 alphabet; `codes.redeem(code, now)` is the account id; `link_code_issued` event | C10 |
| `live_db_unknown_account_line` | `account: no account named Player_9999` | new |
| `no_pool_answers_unavailable` | one line, no DB | new |
| `account_arm_answers_the_gm` | one feedback packet to the GM; `gm_action` row with `reason = "db_unavailable"` | the arm reaches `account_admin::handle` |

Checks: S1 `cimmeria-base-session`, S1 `cimmeria-base-world-entry`, S2 `cimmeria-base-world-entry`, S3 `account_admin`, S4, and S1 `cimmeria-server` if step 4 changed it.

## CT-46 Docs: launcher sign-in, GM `.account`, deployment pointers

- **Host:** windows · **Agent:** documentation-writer · **Branch:** `client-tls-oidc/CT-46-docs` · **Worktree:** `cto-46` (at the wave integration head after CT-40..CT-45 merged in)
- **Commit:** `docs: launcher sign-in, linked accounts and .account (CT-46)`
- **Rules:** R1, R3, R4, R5, R7, R9. Every touched file under `docs/` and `crates/launcher/desktop/docs/` stays CRLF. Never touch `docs/client/sgw-launcher.md` (the retired egui launcher's archived page).

Read first: [design.md](design.md), [contract.md](contract.md) C1, C4, C6-C10, C13, and the merged code of waves 1-4. State what works and what is untested: V1-V3 (the client overlay in the live game), V5 (the relay under Wine) and the packaged Windows/macOS keyring are not verified until wave 5's UATs.

Files:

1. New `docs/technical/launcher-sign-in.md` (Explanation plus Reference; at most 300 lines): the flow (launcher browser sign-in, PKCE handoff, `needs_link`, Play through the loopback relay with a launch ticket, the scrub on exit); linking rules and residual risks (C7, the ledger's linking section); the D-CT11 rule (C15: a linked account has no password, linking removes it for good, Play never falls back to a password, recovery is a GM link code); what stays compatible (unmodified clients, plain HTTP, legacy SHA-1, D-CT7; for accounts without a linked identity only); the `/auth/v1` route table in short (link to contract C6 for detail); kill switches and env (C4); telemetry spans and events (C11) with the `account_admin` GM target; failure modes (provider outage, server restart clears tickets, rate limits).
2. `docs/protocol/login-handshake.md` (646 lines): at most 3 added lines: a pointer that the launcher may relay `UserAuth` over HTTPS and that a password field may hold a launch ticket, linking the new page.
3. `docs/technical/login-auth-flow.md` and `docs/connection-flow.md`: a short section each (at most 12 lines) on the ticket path and the TLS-only rule (`ticket_requires_tls`).
4. `crates/launcher/desktop/docs/launch.md` (612 lines): at most 15 lines on the relay, the ticket file and the scrub, linking the new page; if that would pass 640, write `crates/launcher/desktop/docs/sign-in.md` instead and add one link line.
5. `crates/launcher/desktop/README.md`, `docs/client/launcher-guide.md` (player-facing: how to sign in, link (and that linking removes the account's password), add or remove a method, sign out, the keep-running prompt, what to do when Play says the sign-in could not be obtained) and `docs/guides/macos.md` (the Keychain prompt; the relay under Wine is unverified).
6. `crates/README.md`: rows for `crates/identity` and `crates/launcher/desktop/relay` in the existing table format.
7. `docs/operations/colo-deploy.md` and `docs/operations/container.md`: one pointer each to the env names (C4) and to `docs/operations/auth-https.md` (written in CT-51; link it now, the file lands in wave 5, so say "added in a later PR" in the link text until then).
8. `docs/commands.md`: the `.account` entry in the same format as `.bankdump` (around l.622), outside any `<!-- gen: -->` block.
9. `docs/architecture/observability-target-catalog.md` (689 lines): rows for `cimmeria_identity` and `account_admin` only if the file stays at or under 700 lines; otherwise list them in `launcher-sign-in.md` § Telemetry and say so in the report.
10. `docs/readme.md`: index line for the new page.

Walk [doc-update-map.md](../../agents/doc-update-map.md) and list the rows touched in the report. Checks: every touched file's CRLF test (R5) and line count; `python tools/docs-gen/regen.py --check` if the script has a check mode (else skip and say so).
