# client-tls-oidc wave 9: launcher sessions, store failures, recovery controls (PR #2031 review)

> **Type:** Reference
> **Audience:** The orchestrator, ops, packet-coders and packet-reviewers
> **Last updated:** 2026-10-26
> **Companions:** [Dispatch rules, rules R1-R10 and checks](work-packets.md), [contract C17 and C18](contract.md#c17-launcher-host-sessions-wave-9-d-ct25), [wave 6](packets-wave6.md), [wave 7](packets-wave7.md), [wave 8](packets-wave8.md), [ledger](README.md), [review ruling](worknotes/pr2031-ruling.md)

Wave 9 fixes the launcher half of the review: sign-in and Play pick the same server (F07), one session state per auth host (F08), the offline server switch (F11), late completions after Sign out or Cancel (F06), credential-store failures (F17), the manual-browser fallback (F09), and the panel's offline and `needs_link` controls (F10, F12).

**Ships as:** every packet merges into `client-tls-oidc/campaign` (`cto-camp`) with `merge-noff.ps1`; still **one PR, #2031**. Wave 9 edits only `crates/launcher/desktop/engine/` and `crates/launcher/desktop/frontend/`, which waves 6, 7 and 8 do not touch, so it runs **in parallel with waves 6 and 8**, branching at the campaign head current at dispatch.

| Packet | Agent | Gate | Depends on |
|---|---|---|---|
| CT-90 Sign-in uses the server Play uses (F07, D-CT21 amended) | packet-coder | none | - |
| CT-86 One slot per auth host; offline switch (F08, F11) | rust-gameserver-dev | none | CT-90 merged |
| CT-87 Operation epoch (F06) | rust-gameserver-dev | none | CT-86 merged |
| CT-88 Fallible credential store (F17, D-CT25) | packet-coder | none | CT-87 merged |
| CT-89 Manual-browser fallback that finishes (F09) | rust-gameserver-dev | none | CT-88 merged |
| CT-91 Panel: offline Sign out, Back to sign in, resume, store warnings (F10, F12, F06 UI) | packet-coder | none | - (merges last) |

**Merge order:** CT-90, CT-86, CT-87, CT-88, CT-89, CT-91. Dispatch CT-90 and CT-91 at once. CT-86, CT-87, CT-88 and CT-89 form a chain on the engine's `sign_in/` files: each branches at the campaign head after the previous one merged. CT-91 builds against the contract (C18.1), not the engine code, and merges after CT-89.

**After the wave:** the wave review loop on `cto-camp`, then one read-only `network-security-auth` pass on the integration head (brief: "review the launcher session, epoch and store-failure handling of wave 9 against contract.md C17 and C18.1; numbered fix notes only"). No live-DB test, so no sentinel ids.

**Common to every engine packet:** `<pkg>` is `cimmeria-launcher-engine`, in the desktop workspace (checks D1, D2, D3; cargo test, not nextest). Desktop tests use `wiremock`, `MemoryStore` and the debug-only `CIMMERIA_LAUNCHER_AUTH_BASE` override, under `super::tests::ENV_LOCK` (the `Fixture` in `driver_tests.rs` takes it). `auth_base` and `host_of` drop the port, so every `wiremock` server keys the store as `127.0.0.1`. A test that needs a second, distinct host uses `http://localhost:<port>/` (`host_of` is `localhost`). `driver_tests.rs` is 617 lines and `session_tests.rs` 553: new tests go in new `#[path]` children of those files, added with one `mod` line each, like `#[path = "driver_handover_tests.rs"] mod handover;` at `driver_tests.rs` l.616-617.

## CT-90 Sign-in uses the server Play uses (F07, D-CT21 amended)

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-90-effective-server` · **Worktree:** `cto-90`
- **Commit:** `fix(launcher): sign-in offers the server Play launches against (CT-90)`
- **Rules:**
  - R1: PowerShell only, never the Bash tool. Start every command with `Set-Location <repo>\.claude\worktrees\cto-90;`. Use absolute paths for every file write.
  - R2: every cargo call goes through `pwsh tools/build-lane/lane.ps1 cargo ...` in the foreground. Never call cargo directly and never pass `--target-dir`. Read the lane log under `logs/cto-90/` before rerunning. At most 5 fix rounds per check.
  - R3: edit only the files below and `git add` explicit paths. Commit with the subject above and the Haiku attribution lines from work-packets.md. No `git stash`, rebase or push.
  - R4: `adopted_tests.rs` is 488 lines, so the new test goes in a new file.
  - R6: unit test in the storage tests.
  - R7: test server names and URLs use `example.org` or `.example`.
  - R8: names come from the excerpt.
- **The defect.** `DesktopState::sign_in_login_server` (`storage/sign_in_state.rs` l.15-25) reads only preferences or the defaults. `launch_login_servers` (`storage/launch/preparation.rs` l.9-26) gives a legacy-adopted installation its own reviewed list. With B in the adoption and A in preferences, the panel signs in to A while Play launches B, and the launch drops the session.
- **Contract excerpt (C18.2):** `DesktopState::sign_in_login_server(&self) -> Result<Option<LoginServer>, StorageError>` answers the first server of the **effective** list, the list Play writes: the adopted list of a legacy-adopted installation, else saved preferences, else the defaults. It and `launch_login_servers` both call one helper, `fn effective_login_servers(&self) -> Result<Vec<LoginServer>, StorageError>`. An error reading the installation makes the driver answer `unavailable`; it never falls back to another list.

Files:

1. `crates/launcher/desktop/engine/src/storage/launch/preparation.rs`, in the `impl DesktopState` block:

   ```rust
   /// The list Play writes: a legacy adoption's reviewed list, else the saved
   /// preference, else the defaults (C18.2).
   fn pick_login_servers(&self, installed: Option<(&InstalledContent, Option<&Provenance>)>) -> Vec<LoginServer> {
       match installed {
           Some((content, Some(p))) if p.legacy_install_id.is_some() => content.intent.login_servers.clone(),
           _ if self.preferences.login_servers.is_empty() => login_servers::default_servers(),
           _ => self.preferences.login_servers.clone(),
       }
   }

   /// The effective list without a launch intent: what sign-in offers (C18.2).
   pub(crate) fn effective_login_servers(&self) -> Result<Vec<LoginServer>, StorageError> {
       let installed = self.installed_for_launch()?;
       Ok(self.pick_login_servers(installed.as_ref().map(|(c, p)| (c, p.as_ref()))))
   }
   ```

   `launch_login_servers` keeps its `Corrupt` and intent checks (l.13-16) and replaces the `match` (l.17-23) with `let servers = self.pick_login_servers(Some((&installed, adoption.as_ref())));`. The intent check makes `installed.intent` equal to `installation`, so the list is unchanged. Keep the `validate` call. Import `InstalledContent` and `Provenance` from where `installed_for_launch` (`storage/installed_content/mod.rs` l.173) names them.
2. `crates/launcher/desktop/engine/src/storage/sign_in_state.rs` l.15-25: `pub fn sign_in_login_server(&self) -> Result<Option<LoginServer>, StorageError> { Ok(self.effective_login_servers()?.into_iter().next()) }`. Change its doc to say the list is the one Play uses, and drop the D-CT21 sentence.
3. `crates/launcher/desktop/engine/src/sign_in/driver.rs` l.109: `let server = guard(state)?.sign_in_login_server().map_err(|_| SignInFailure::Unavailable)?;`. Add a WARN-free `tracing::debug!(target: "launcher.sign_in", reason = "installation_unreadable", ...)` on the error.
4. `crates/launcher/desktop/engine/src/storage/effective_settings/adopted_sign_in_tests.rs` (new). It is a child of `adopted_tests.rs` (`#[path = "adopted_sign_in_tests.rs"] mod sign_in;` at its end) and uses `super::*`.

Tests (unit):

| Test | Fails on revert because |
|---|---|
| `sign_in_offers_the_adopted_server_not_the_saved_one`: the setup of `an_adopted_copy_plays_with_its_reviewed_login_servers_not_the_saved_list` (`adopted_tests.rs` l.103-118), then assert `state.sign_in_login_server().unwrap()` is `Some` of the first of `fixtures::SERVERS`, and equals the first of `launch_login_servers(&installed.intent)` | Today it answers the saved `http://saved.example:8081` server. |
| `sign_in_without_an_installation_uses_the_saved_list`: a fresh `DesktopState::open` in a temp dir with one saved server, then `sign_in_login_server()` is that server | Guards the not-installed path (no installation is `Ok(None)` from `installed_for_launch`). |

**Checks, in order:** D1; D2 `cimmeria-launcher-engine`; D3 `cimmeria-launcher-engine` (no filter; the whole engine suite, since `launch_login_servers` changed). **Docs rows:** none here; CT-92 records D-CT21 amended.

## CT-86 One slot per auth host; offline switch (F08, F11)

- **Host:** windows · **Agent:** rust-gameserver-dev · **Branch:** `client-tls-oidc/CT-86-host-slot` · **Worktree:** `cto-86` (branch at the campaign head after CT-90 merged)
- **Commit:** `fix(launcher): one session state per auth host; offline switch resolves its own host (CT-86)`
- **Rules:**
  - R1 and R2 as in CT-90, with worktree `cto-86` and lane logs under `logs/cto-86/`.
  - R3 as in CT-90, but with the **Sonnet** attribution lines.
  - R4: `driver.rs` is 566 lines and `session.rs` 502. Both must end **under 500**, along the seams named below: the browser-flow handle moves to a new `flow.rs`, and the store helpers move to the new `host_slot.rs`.
  - R6: driver tests in a new `driver_host_tests.rs`.
  - R7: test accounts are `Player_1234` and `Player_5678`; the IP host is `203.0.113.5`.
  - R8: names come from C17.1.
  - No `DesktopState` guard is held across an `.await`, as the driver's module doc says.
- **The defects.**
  - F08: each `SignIn` has its own `tokio::sync::Mutex<Inner>` (`session.rs` l.81-94), but every object for one host shares one store entry. A game's retained `Arc<SignIn>` and a re-restored one can therefore both spend the same refresh token, and the server revokes the family as reuse.
  - F11: `offline` (`driver.rs` l.218-222) restores only when the state holds no session. After a switch from A to an unreachable B, it keeps A's session under B's name, and Sign out deletes A's entry. A target with no auth base keeps the previous session too.
  - Sign out (`session.rs` l.241-254) posts `/logout` before it forgets, so a refresh can slip in between.
- **Contract excerpt (C17.1):**

  ```rust
  // sign_in/host_slot.rs (new; `pub use` from session.rs)
  pub struct HostSlot { state: tokio::sync::Mutex<HostState> }
  struct HostState { access: Option<(String, Instant)>, account: Option<AccountSummary> }  // CT-88 adds two fields
  impl SignIn {
      pub fn new(base: Url, http: reqwest::Client, store: Arc<dyn CredentialStore>) -> Self;            // private slot
      pub fn restore(base: Url, http: reqwest::Client, store: Arc<dyn CredentialStore>) -> Option<Self>; // private slot
      pub fn new_in(base: Url, http: reqwest::Client, store: Arc<dyn CredentialStore>, slot: Arc<HostSlot>) -> Self;
      pub async fn restore_in(base: Url, http: reqwest::Client, store: Arc<dyn CredentialStore>, slot: Arc<HostSlot>) -> Option<Self>;
  }
  // driver.rs: SignInDriver gains `hosts: Mutex<HashMap<String, Arc<HostSlot>>>` and
  // fn slot(&self, base: &Url) -> Arc<HostSlot>   // keyed by host_of(base); created on first use, never removed
  ```

  Every `SignIn` the driver makes for one host shares one slot, and every store read and write happens under the slot lock. `restore_in` locks the slot and reads the store. Nothing stored is `None`. A stored entry sets the slot's `account` when it is empty (an access token already in the slot is kept) and answers a `SignIn` on that slot. The driver calls only `new_in` and `restore_in`, so `session_for` becomes `async`. `sign_out` locks the slot, reads the stored refresh token, forgets (store and memory), releases the lock, then posts `/logout`.

Files:

1. `crates/launcher/desktop/engine/src/sign_in/host_slot.rs` (new; `mod host_slot;` in `sign_in/mod.rs`; `pub use super::host_slot::HostSlot;` in `session.rs`):
   - `HostSlot` and `HostState`, with `HostState` replacing `Inner` (`session.rs` l.79-85).
   - `impl HostSlot { pub fn new() -> Arc<Self> }` (an empty state).
   - Move `host_of` (l.456) here as `pub(super)`.
   - Move the three store helpers here as free functions that take what they use, so CT-88 changes them in one place:
     - `pub(super) fn adopt(store: &dyn CredentialStore, host: &str, state: &mut HostState, signed: SignedIn) -> AccountSummary` (today's `adopt`, l.318-335);
     - `pub(super) fn forget(store: &dyn CredentialStore, host: &str, state: &mut HostState)` (l.338-342);
     - `pub(super) fn forget_if_stored(store: &dyn CredentialStore, host: &str, state: &mut HostState, used: &str)` (l.347-358).
   - `SignedIn` must be reachable as `pub(super)` from `session.rs`.
2. `crates/launcher/desktop/engine/src/sign_in/session.rs`:
   - Change the field `state` to `slot: Arc<HostSlot>`, and every `self.state.lock()` to `self.slot.state.lock()`.
   - `new` is `new_in(.., HostSlot::new())`. `restore` keeps its sync body on a fresh slot whose state is built directly, not locked.
   - `restore_in` follows the excerpt.
   - `sign_out`:

     ```rust
     pub async fn sign_out(&self) {
         let stored = {
             let mut state = self.slot.state.lock().await;
             let stored = self.store.get(host_of(&self.base));
             host_slot::forget(&*self.store, host_of(&self.base), &mut state);
             stored
         };
         if let Some(stored) = stored { /* POST logout with stored.refresh_token, as today (l.243-250) */ }
     }
     ```

   - Change the doc of `sign_out` and of `SignIn` (one slot per host; a refresh and a sign-out serialize on it).
3. `crates/launcher/desktop/engine/src/sign_in/flow.rs` (new; `mod flow;` in `sign_in/mod.rs`). Move the browser-flow handle out of the driver unchanged:

   ```rust
   pub(super) struct FlowSlot { waiting: Mutex<Option<(u64, oneshot::Sender<()>)>>, generation: AtomicU64 }
   impl FlowSlot {
       pub(super) fn new() -> Self;
       pub(super) async fn run<F>(&self, browser: &dyn Browser, start: F) -> Result<CodeGrant, SignInFailure>  // today's sign_in_flow body, l.339-392
       where F: Future<Output = Result<BrowserFlow, SignInError>>;
       pub(super) fn cancel(&self) -> Result<(), SignInFailure>;   // today's cancel_flow, l.395-401
       pub(super) fn is_waiting(&self) -> bool;                     // what reply/panel_state read from `waiting` today
   }
   ```

   `guard` (driver.rs) becomes `pub(super)` so `flow.rs` can use it.
4. `crates/launcher/desktop/engine/src/sign_in/driver.rs`:
   - Fields: replace `waiting` and `generation` with `flow: FlowSlot`, and add `hosts`.
   - Add `fn slot(&self, base: &Url) -> Arc<HostSlot>` (lock `hosts`, `entry(host_of(base).to_owned()).or_insert_with(HostSlot::new).clone()`).
   - `session_for` (l.240-255) becomes `async`: read the current session under the guard and drop the guard; if its base equals `base`, return it. Otherwise `SignIn::restore_in(base.clone(), .., self.slot(base)).await.map(Arc::new)`, then `set_sign_in` under a new guard. Update its callers with `.await`.
   - `begin` (l.272) and `link` (l.310) use `SignIn::new_in(.., self.slot(&base))`.
   - `offline` (l.218-223), F11:

     ```rust
     let session = match discovery::auth_base(&server.url) {
         Some(base) => self.session_for(state, &base).await?,
         None => { guard(state)?.set_sign_in(None); None }
     };
     ```

   - In `run`, find any other place that hands a session to a target with no auth base, and resolve it the same way.
   - `sign_in_flow` callers become `self.flow.run(..)`, `cancel_flow` becomes `self.flow.cancel()`, and readers of `waiting` become `self.flow.is_waiting()`.
5. `crates/launcher/desktop/engine/src/sign_in/driver_host_tests.rs` (new; `#[path = "driver_host_tests.rs"] mod host;` in `driver_tests.rs` next to `mod handover;`).

Tests (unit, `#[tokio::test]`):

| Test | Fails on revert because |
|---|---|
| `a_retained_session_and_a_restored_one_refresh_once`. Steps: `fx.store_entry("r1")`; `inspect`; keep `let old = fx.state.lock().unwrap().sign_in().unwrap()`; `fx.state.lock().unwrap().set_sign_in(None)` (as a switch away and back does); `inspect` again and take `new`; assert `!Arc::ptr_eq(&old, &new)`. Mount `/auth/v1/token` with `set_delay(300 ms)`, `signed_in_body("r2")` and `.expect(1)`. Then `tokio::join!(old.access_token(), new.access_token())`: both `Ok`, equal; the store holds `r2`. | Today each object has its own mutex: two `/token` calls, and `expect(1)` fails when the mock server is verified. |
| `offline_switch_shows_and_signs_out_only_the_new_host`. Online at the fixture: `fx.store_entry("ra")`, `inspect` is signed in. Store `Player_5678` (refresh `rb`) under `"localhost"`. Set `AUTH_BASE_ENV` to `http://localhost:1/` (closed, so offline) and save the login server `https://other.example/login` (as `switch_to_second_server` does). Then: `inspect` is `available: false` with `state: signed_in` and account `Player_5678`; `sign_out`; `store.get(HOST)` is still `Some` and `store.get("localhost")` is `None`. | Today offline keeps A's session: the reply names A's account, and Sign out deletes `127.0.0.1`. |
| `a_target_without_an_auth_base_drops_the_previous_session`. Signed in at the fixture. Then `std::env::remove_var(AUTH_BASE_ENV)` and save the login server `http://203.0.113.5:8081/login` (an IP host: `auth_base` is `None`). `inspect`: state `signed_out`, `fx.has_session()` false; the store entry for `HOST` is kept. | Today the old session stays in the state. |
| `sign_out_forgets_before_it_tells_the_server`. Stored and signed in. Mount `/auth/v1/logout` with a responder that captures `fx.store.clone()` and records `store.get(HOST).is_none()` in an `Arc<AtomicBool>`, answering 204. `sign_out`; the flag is true. | Today `/logout` is posted while the entry is still stored. |

Run the whole engine suite: every existing driver and session test must pass unchanged. If one breaks only because two mock servers now share the `127.0.0.1` slot, give that test's second server a `localhost` base; never key slots by port. Report each such change.

**Checks, in order:** D1; D2 `cimmeria-launcher-engine`; D3 `cimmeria-launcher-engine`. Then report the line counts of `driver.rs`, `session.rs`, `flow.rs` and `host_slot.rs`. **Docs rows:** none here (CT-92).

## CT-87 Operation epoch (F06)

- **Host:** windows · **Agent:** rust-gameserver-dev · **Branch:** `client-tls-oidc/CT-87-epoch` · **Worktree:** `cto-87` (branch at the campaign head after CT-86 merged)
- **Commit:** `fix(launcher): Sign out and Cancel void every late sign-in completion (CT-87)`
- **Rules:**
  - R1, R2 and R3 (Sonnet) as in CT-86, with worktree `cto-87`.
  - R4: `driver.rs`, `session.rs` and `flow.rs` stay under 500. Put `Epoch` in `host_slot.rs`, and the tests in a new `driver_epoch_tests.rs`.
  - R6: driver tests that interleave with `tokio::join!` and `wiremock` delays; no sleeps longer than 500 ms.
  - R7 as in CT-86.
  - R8: names from C17.2.
- **The defect (F06).** Sign out clears the state and the pending choice, but leaves the browser flow, the token exchange and the link calls running. When they finish, `adopt` writes the store and `signed_in` installs the session again (`driver.rs` l.258-276, 432-440; `session.rs` l.311-315). The flow's generation counter only guards its own cancel handle.
- **Contract excerpt (C17.2):**
  - `SignInDriver.epoch: AtomicU64`. Each `run` captures `let epoch = self.epoch.load(SeqCst)` at entry.
  - `sign_out` and `cancel` bump it **while holding the `DesktopState` lock**; `sign_out` also cancels the flow.
  - `SignIn::complete`, `link_new`, `link_existing_password` and `link_existing_code` take `fence: &Epoch` (`pub struct Epoch<'a> { counter: &'a AtomicU64, expected: u64 }` with `is_current()`). `adopt_signed_in` checks it under the slot lock before writing the store.
  - A stale epoch skips the store write, posts `/logout` with the refresh token just received (best effort), and returns `SignInError::Superseded`. The driver maps that to `SignInFailure::Cancelled`.
  - `signed_in` and `finish` check the epoch again under the `DesktopState` lock, before `set_sign_in` and `keep_pending`.
  - Refreshes (`access_token`) take no epoch: `sign_out` waits for the slot lock.
  - A server switch does not bump the epoch. A completion for the old base is still stored under its own host (the player finished it there), but `signed_in` and `finish` install it, or keep its pending choice, only when the current target's auth base still equals the session's base.

Files:

1. `crates/launcher/desktop/engine/src/sign_in/mod.rs` `SignInError` (l.28-43): add the variant `Superseded` (doc: "Sign out or Cancel ran while this step was in flight"). Its `reason()` text is `"Sign-in was cancelled."`.
2. `crates/launcher/desktop/engine/src/sign_in/commands.rs` l.207: `SignInError::Cancelled | SignInError::Superseded => Self::Cancelled`.
3. `crates/launcher/desktop/engine/src/sign_in/host_slot.rs`: `pub struct Epoch<'a> { counter: &'a AtomicU64, expected: u64 }` with `pub fn new(counter: &'a AtomicU64, expected: u64) -> Self` and `pub fn is_current(&self) -> bool { self.counter.load(SeqCst) == self.expected }`.
4. `crates/launcher/desktop/engine/src/sign_in/session.rs`:
   - `complete`, `link_new`, `link_existing_password` and `link_existing_code` gain a last parameter, `fence: &Epoch<'_>`, and pass it through `link_call` to `adopt_signed_in`.
   - `adopt_signed_in` locks the slot. If `!fence.is_current()`, it drops the lock, posts `logout` with `signed.refresh_token` (log a debug line on failure, as `sign_out` does), and returns `Err(SignInError::Superseded)`. Otherwise it adopts, as today.
   - `record` already logs the failure.
5. `crates/launcher/desktop/engine/src/sign_in/driver.rs`:
   - Add the field `epoch: AtomicU64`. In `run`, capture `let epoch = self.epoch.load(Ordering::SeqCst);` after the schema check.
   - Cancel (l.106-108): `{ let _st = guard(state)?; self.epoch.fetch_add(1, SeqCst); } self.flow.cancel()?;`.
   - `sign_out` (l.453-464): before `session.sign_out()`, `{ let _st = guard(site.state)?; self.epoch.fetch_add(1, SeqCst); } self.flow.cancel()?;`.
   - Pass `&Epoch::new(&self.epoch, epoch)` into every `complete` and `link_*` call (`begin`, `add_method`, `link`). Thread `epoch: u64` through `begin`, `add_method`, `link`, `finish` and `signed_in` as a parameter.
   - `signed_in`: under one guard, `if self.epoch.load(SeqCst) != epoch || !self.same_target(&st, session.base()) { drop(st); return self.reply(site, None, None).await... }`. Make it the reply the current state gives (the session the state holds, if any), not `Err`, then `set_sign_in` and `clear_pending` as today.
   - `finish` checks the same before `keep_pending`.
   - `fn same_target(&self, st: &DesktopState, base: &Url) -> bool` is `st.sign_in_login_server().ok().flatten().and_then(|s| discovery::auth_base(&s.url)).as_ref() == Some(base)`.
   - When the epoch is stale, `signed_in` and `finish` answer `Err(SignInFailure::Cancelled)`. When only the target moved, they answer the current reply.
6. `crates/launcher/desktop/engine/src/sign_in/session_tests.rs` (and any other caller the compiler names): pass a current fence. Add a test helper `fn fresh() -> AtomicU64` and use `&Epoch::new(&counter, 0)`.
7. `crates/launcher/desktop/engine/src/sign_in/driver_epoch_tests.rs` (new; `#[path = "driver_epoch_tests.rs"] mod epoch;` in `driver_tests.rs`). Reuse `until_opened` and the `SilentBrowser` pattern from `driver_handover_tests.rs`; copy `until_opened` if it is private to that file.

Tests (unit, `#[tokio::test]`; each `tokio::join!`s the slow command with a future that waits for its trigger, then sends the interrupting command):

| Test | Fails on revert because |
|---|---|
| `sign_out_during_the_browser_wait_voids_the_callback`. `begin` with a `SilentBrowser`, joined with: `until_opened`, `sign_out`, then a GET of the captured `redirect_uri?code=c&state=<state>`. `begin` is `Err(Cancelled)`; `fx.has_session()` is false; `store.get(HOST)` is `None`; `requests_to("/auth/v1/token")` is empty. | Today Sign out leaves the flow waiting: the callback completes it, and the session is stored and installed. |
| `sign_out_during_the_token_exchange_discards_the_reply`. `/auth/v1/token` has `set_delay(400 ms)` and answers `signed_in_body("r9")`; `begin` with `CallbackBrowser`, joined with: poll `fx.server.received_requests()` until one `/auth/v1/token` request arrived, then `sign_out`. `begin` is `Err(Cancelled)`; the store and the state are empty; `requests_to("/auth/v1/logout")` contains `{"refresh_token": "r9"}`. | Today `adopt` stores `r9` after Sign out and `signed_in` installs it. |
| `sign_out_during_a_link_call_discards_the_reply`. Reach `needs_link` (as `pending_from_one_server_is_not_offered_on_another` does). `/auth/v1/link/new` has `set_delay(400 ms)` and answers `signed_in_body("r8")`; `link_new` joined with a wait-then-`sign_out`. `link_new` is `Err(Cancelled)`; nothing stored; `inspect` is `signed_out` with no pending choice; `requests_to("/auth/v1/link/new")` has exactly one entry. | Today the link reply is stored and installed after Sign out. |
| `cancel_after_the_callback_discards_the_reply`. The same delayed `/token` as above, then `cancel` instead of `sign_out`. `begin` is `Err(Cancelled)` and nothing is stored. | Today `cancel` finds no waiting handle once the callback arrived, and the exchange completes. |
| `a_server_switch_mid_flow_stores_but_does_not_install`. `begin` with a `SilentBrowser`; after `until_opened`, call `switch_to_second_server(&fx)` (keep the returned server alive), then GET the callback. `fx.has_session()` is false after `begin` returns. | Today `signed_in` installs the old server's session under the new server. |

**Checks, in order:** D1; D2 `cimmeria-launcher-engine`; D3 `cimmeria-launcher-engine`. **Docs rows:** none here (CT-92).

## CT-88 Fallible credential store (F17, D-CT25)

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-88-store-errors` · **Worktree:** `cto-88` (branch at the campaign head after CT-87 merged)
- **Commit:** `fix(launcher): credential-store failures keep the session honest and visible (CT-88)`
- **Rules:**
  - R1, R2 and R3 (Haiku) as in CT-90, with worktree `cto-88`.
  - R4: `session.rs`, `driver.rs` and `host_slot.rs` stay under 500. Tests go in a new `driver_store_tests.rs`.
  - R6: driver tests over a fault-injecting store.
  - R7: the store's errors and logs never carry a host, token or account name; the log names only the operation and the `StoreError` variant.
  - R8: names from C17.3.
  - The compiler finds every call site. Change each one to the new signature without changing what the test asserts.
- **The defect (F17).** Every `KeyringStore` error is logged at debug and dropped (`store.rs` l.87-120).
  - A failed `set` after a refresh loses the rotated token, so the next refresh spends a dead one and the server sees reuse.
  - A failed `delete` at Sign out leaves the entry, and the next launch restores the account the player signed out of.
  - A failed `get` looks the same as "not signed in".
- **Contract excerpt (C17.3 and C18.1, first bullet):**

  ```rust
  pub enum StoreError { Unavailable, Io }      // no secret, no host in Display
  pub trait CredentialStore: Send + Sync {
      fn get(&self, host: &str) -> Result<Option<Stored>, StoreError>;   // Ok(None) = nothing stored
      fn set(&self, host: &str, stored: &Stored) -> Result<(), StoreError>;
      fn delete(&self, host: &str) -> Result<(), StoreError>;            // deleting nothing is Ok
  }
  ```

  - `set` fails: the slot keeps the entry in `session_only` and the session works until the launcher exits; the status gets `storage_warning: "session_only"`.
  - `delete` fails: the slot sets `tombstone` (the process treats the host as signed out and never restores it), `session_only` is cleared, and the status gets `storage_warning: "sign_out_incomplete"`.
  - `get` fails: nothing is restored, and the status gets `storage_warning: "store_unreadable"`. Sign-in still works session-only.
  - Reads prefer `session_only`, then the store; a tombstone answers `None` before either.
  - `SignInStatus` gains `storage_warning: null | "session_only" | "sign_out_incomplete" | "store_unreadable"`.

Files:

1. `crates/launcher/desktop/engine/src/sign_in/store.rs`:
   - Add `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum StoreError { Unavailable, Io }` with a `Display` of `"credential store unavailable"` / `"credential store i/o error"`, and `as_str()` (`"unavailable"` / `"io"`) for logs.
   - Change the trait as in the excerpt and update its docs. `MemoryStore` answers `Ok` (a poisoned lock is `Err(Io)`).
   - `KeyringStore`: put the mapping in one function, `fn classify(e: &keyring::Error) -> StoreError`. `keyring::Error::PlatformFailure(_)` and `NoStorageAccess(_)` are `Unavailable`; anything else is `Io`.
     - `get`: `NoEntry` is `Ok(None)`. A value that is not valid `Stored` JSON is `Err(Io)`.
     - `delete`: `NoEntry` is `Ok(())`.
     - An `Entry::new` failure is `Err(classify(..))`.
   - Log each error at `warn` with `op` (`get`, `set` or `delete`) and `error = e.as_str()`.
   - Change the module doc (l.5-6): errors are returned, never dropped.
2. `crates/launcher/desktop/engine/src/sign_in/host_slot.rs`:
   - `HostState` gains `session_only: Option<Stored>`, `tombstone: bool` and `warning: Option<&'static str>`.
   - Add `pub(super) fn read(store: &dyn CredentialStore, host: &str, state: &mut HostState) -> Option<Stored>`: a tombstone gives `None`, else `session_only.clone()`, else `store.get`. On `Err`, it sets `warning = Some("store_unreadable")` and answers `None`.
   - `adopt`: on `store.set` `Ok`, clear `session_only`, `tombstone` and `warning`. On `Err`, set `session_only = Some(stored)`, `tombstone = false` and `warning = Some("session_only")`.
   - `forget` and `forget_if_stored`: read through `read`. On `delete` `Ok`, clear `session_only` and `warning` and set `tombstone = false`. On `Err`, set `tombstone = true`, `session_only = None` and `warning = Some("sign_out_incomplete")`.
   - Add `impl HostSlot { pub(super) async fn warning(&self) -> Option<&'static str> }`.
3. `crates/launcher/desktop/engine/src/sign_in/session.rs`:
   - `restore`, `restore_in`, `access_token` and `sign_out` read through `host_slot::read`, not `self.store.get`.
   - `restore` (the sync, private-slot constructor) maps `Err` to `None`.
   - Add `pub async fn storage_warning(&self) -> Option<&'static str>`.
4. `crates/launcher/desktop/engine/src/sign_in/commands.rs` `SignInStatus` (l.79-90): add `pub storage_warning: Option<&'static str>` (doc: the C18.1 values).
5. `crates/launcher/desktop/engine/src/sign_in/driver.rs`:
   - `Site` gains `base: Option<&'a Url>`: the auth base the target resolves, filled where `Site` is built (`run`, `offline`).
   - `reply` fills `storage_warning` from the session's slot when there is a session, else from `self.slot(base)` when `site.base` is `Some`, else `None`.
6. Every other `CredentialStore` caller or implementor. Grep `CredentialStore`, `.store.get(`, `.store.set(`, `store.delete(` and `store_entry` under `crates/launcher/desktop/`. They include:
   - `driver_tests.rs` l.133 (`store_entry` becomes `.unwrap()`), l.284, 312, 579 and 600;
   - `session_tests.rs` l.42, 101, 137, 145, 175, 194, 227, 246 and 474;
   - `storage/launch/sign_in_play_tests.rs` l.56;
   - the CT-86 and CT-87 test files.

   Change `assert_eq!(store.get(h), Some(x))` to `assert_eq!(store.get(h).unwrap(), Some(x))`; never weaken an assertion.
7. `crates/launcher/desktop/engine/src/sign_in/driver_store_tests.rs` (new; `#[path = "driver_store_tests.rs"] mod store_faults;` in `driver_tests.rs`). Add `struct FaultyStore { inner: MemoryStore, fail_get: AtomicBool, fail_set: AtomicBool, fail_delete: AtomicBool }`. Each method returns `Err(StoreError::Io)` while its flag is set, else delegates. Build the driver on it with a fixture variant, `Fixture::with_store(Arc<FaultyStore>)`: add it next to `Fixture::new` if it fits in two short lines plus a shared body, else copy the fixture into the new file.

Tests (unit, `#[tokio::test]`):

| Test | Fails on revert because |
|---|---|
| `a_sign_in_the_store_refuses_is_kept_for_the_session`. `fail_set`; `begin` with `CallbackBrowser` (`/token` answers `signed_in_body("r1")`): signed in, `storage_warning == Some("session_only")`. Then `fx.state.lock().unwrap().set_sign_in(None)` and `inspect`: still signed in (restored from the slot's `session_only`); the inner store is empty. | Today the failed write is dropped and nothing restores: `inspect` is `signed_out`. |
| `a_refresh_the_store_refuses_keeps_the_rotated_token`. Stored `r1`, signed in. Mount `/token`: refresh `r1` gives `r2`, and refresh `r2` gives `r3` (`body_partial_json`). `fail_set`. Expire the access token (as the existing refresh tests in `session_tests.rs` do), call `access_token` twice. Both succeed; the second `/token` body carries `r2`. | Today the second refresh sends `r1` again, which the server treats as reuse. |
| `a_sign_out_the_store_refuses_stays_signed_out`. Stored, signed in; `fail_delete`; `sign_out`: `signed_out`, `storage_warning == Some("sign_out_incomplete")`. Then `set_sign_in(None)` and `inspect`: still `signed_out`, although the inner store holds the entry. | Today the next restore signs the player back in. |
| `an_unreadable_store_is_reported_and_sign_in_still_works`. `fail_get`; `inspect`: `signed_out`, `storage_warning == Some("store_unreadable")`. Clear `fail_get`, then `begin`: signed in, warning `None`. | Today a read error looks like "not signed in", with no warning. |
| `deleting_nothing_is_ok` (in `store.rs` tests or the new file): `MemoryStore::default().delete("h")` is `Ok(())`. | Guards the contract's `Ok` for a missing entry. |

The `KeyringStore` mapping cannot run in CI without an OS store. Mark it with a one-line comment that CT-U6 covers it on a real host, and do not stub `keyring`.

**Checks, in order:** D1; D2 `cimmeria-launcher-engine`; D3 `cimmeria-launcher-engine`. **Docs rows:** none here (CT-92).

## CT-89 Manual-browser fallback that finishes (F09)

- **Host:** windows · **Agent:** rust-gameserver-dev · **Branch:** `client-tls-oidc/CT-89-resume` · **Worktree:** `cto-89` (branch at the campaign head after CT-88 merged)
- **Commit:** `fix(launcher): a browser that will not open parks the flow; resume finishes it (CT-89)`
- **Rules:**
  - R1, R2 and R3 (Sonnet) as in CT-86, with worktree `cto-89`.
  - R4: `flow.rs` stays under 300 and `driver.rs` under 500. Tests go in a new `driver_resume_tests.rs`.
  - R6: driver tests with `FailingBrowser` plus a manual callback.
  - R7 as in CT-86.
  - R8: names from C18.1.
- **The defect (F09).** When `browser.open` fails (`driver.rs` l.362-366), the driver returns `BrowserFailed { url }` and drops the `BrowserFlow`. That closes the loopback listener and loses the verifier, so the URL the panel tells the player to paste leads to a closed port.
- **Contract excerpt (C18.1, second bullet):** a new command, `resume` (no other fields). A `begin` or `add_method` whose browser would not open now **parks** its flow and keeps `waiting: true`. The parked flow holds the listener, the verifier, the cancel handle, the epoch, and what it continues: a plain sign-in for `provider` at `base`, or an Add method on the current session. The command fails with `browser_failed {url}` as today. `resume` awaits the parked flow and completes it as the original command would have. With nothing parked, it fails `no_pending_flow` (a new `SignInFailure` code). `cancel` and the flow's own 5-minute limit clear the parked flow.

Files:

1. `crates/launcher/desktop/engine/src/sign_in/oauth.rs`: add `pub async fn finish_within(self, limit: Duration) -> Result<CodeGrant, SignInError>` (today's `finish` body with `limit`), and make `finish` call `finish_within(SIGN_IN_TIMEOUT)`.
2. `crates/launcher/desktop/engine/src/sign_in/commands.rs`:
   - `SignInCommand::Resume { schema_version: u32 }`, added to `schema_version()`.
   - `SignInFailure::NoPendingFlow`.
3. `crates/launcher/desktop/engine/src/sign_in/flow.rs`:

   ```rust
   /// What a parked flow completes once its grant arrives.
   pub(super) enum Continue { Begin { base: Url, provider: String }, AddMethod { session: Arc<SignIn>, provider: String } }
   struct Parked { mine: u64, flow: BrowserFlow, cancel_rx: oneshot::Receiver<()>, epoch: u64, cont: Continue, since: Instant }
   ```

   - `FlowSlot` gains `parked: Mutex<Option<Parked>>`.
   - `run` takes `cont: Continue` and `epoch: u64`. When `browser.open` fails, it moves the flow, the receiver, the epoch and `cont` into `parked`, keeps its `waiting` entry, and returns `BrowserFailed { url }`. It never clears its own handle on that path.
   - `pub(super) async fn resume(&self) -> Result<(CodeGrant, Continue, u64), SignInFailure>`:
     - It takes `parked`. `None` is `NoPendingFlow`. When `since.elapsed() >= SIGN_IN_TIMEOUT`, it clears `waiting` for `mine` and answers `TimedOut`.
     - Otherwise it runs `select!` over `flow.finish_within(SIGN_IN_TIMEOUT - since.elapsed())` and `cancel_rx`. Map and log the result as `run` does, and clear `waiting` for `mine`.
   - `cancel` also drops `parked`; dropping the flow closes the listener.
   - `is_waiting` first drops a parked flow older than `SIGN_IN_TIMEOUT` (and its `waiting` entry for `mine`).
   - Add `#[cfg(test)] pub(super) fn backdate_parked(&self, by: Duration)`, which moves `since` back.
4. `crates/launcher/desktop/engine/src/sign_in/driver.rs`:
   - Split `begin` and `add_method` at the grant. Each starts its flow with its `Continue`, then calls one shared continuation:

     ```rust
     async fn continue_flow(&self, site: &Site<'_>, discovery: &Discovery, grant: CodeGrant, cont: Continue, epoch: u64) -> Result<SignInStatus, SignInFailure>
     ```

     `Begin` is today's l.272-275 (`new_in`, `complete`, `finish`). `AddMethod` is today's l.291-299 (`complete`, `finish`, then the account view).
   - `SignInCommand::Resume` is valid only online (like `begin`): `let (grant, cont, epoch) = self.flow.resume().await?; self.continue_flow(&site, discovery, grant, cont, epoch).await`.
   - Offline, `Resume` answers `Unavailable`, as other network commands do.
   - The parked epoch is the one the original command captured, so a Sign out in between voids it (CT-87).
5. `crates/launcher/desktop/engine/src/sign_in/driver_resume_tests.rs` (new; `#[path = "driver_resume_tests.rs"] mod resume;` in `driver_tests.rs`).

Tests (unit, `#[tokio::test]`):

| Test | Fails on revert because |
|---|---|
| `a_failed_browser_parks_the_sign_in_and_resume_finishes_it`. `begin` with `FailingBrowser` (with `/token` answering `signed_in_body("r1")`) is `Err(BrowserFailed { url })`; `inspect` shows `waiting: true`. Join `resume` with a GET of `url`'s `redirect_uri?code=c&state=<state>`: `resume` is `Ok` and signed in, `store.get(HOST)` has `r1`, and `waiting` is false. | Today the listener is gone: the GET is refused and `resume` does not exist. |
| `a_failed_browser_parks_add_method_and_resume_finishes_it`. The same for `add_method`, signed in first. `/link/start` answers an `authorize_url` under the mock base built from the request's values (as the existing Add-method test does), and `/token` answers `signed_in_body("r2")`. After `resume`, the reply carries `account`. | Same as above, for the Add-method path. |
| `resume_with_nothing_parked_is_no_pending_flow`. | The command is new. |
| `cancel_clears_a_parked_flow`. Park, then `cancel`; `resume` is `NoPendingFlow`, and a GET of the callback is refused (the listener is closed). | Guards that `cancel` drops the listener. |
| `a_parked_flow_expires`. Park, then `fx.driver` `backdate_parked(SIGN_IN_TIMEOUT)`; `inspect` shows `waiting: false`, and `resume` is `NoPendingFlow`. | Guards the 5-minute limit. |
| `sign_out_voids_a_parked_flow`. Park, then `sign_out`, then `resume` is `NoPendingFlow` (`sign_out` cancels), and nothing is stored. | Guards CT-87 across a park. |

**Checks, in order:** D1; D2 `cimmeria-launcher-engine`; D3 `cimmeria-launcher-engine`. **Docs rows:** none here (CT-92).

## CT-91 Panel: offline Sign out, Back to sign in, resume, store warnings (F10, F12, F06 UI)

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-91-panel-recovery` · **Worktree:** `cto-91` (branch at the campaign head at dispatch; it merges after CT-89)
- **Commit:** `fix(launcher-ui): offline Sign out, Back to sign in, browser fallback resume, store warnings (CT-91)`
- **Rules:**
  - R1 as in CT-90, with worktree `cto-91`. Run every check from the worktree; never from the main checkout, whose `frontend/ui/index.html` has someone else's uncommitted edits.
  - R3 (Haiku) as in CT-90. This packet does not touch `ui/index.html`: the panel builds its controls in TypeScript.
  - R4: the four `.ts` files stay under 400. If a test file passes 400, put the new tests in a new `*.test.ts` next to it.
  - R6: `node:test` with `linkedom`, as `sign-in-view.test.ts` does. Each test drives the real `mountSignIn` or `mountAccount` and asserts on the requests sent and the visible text.
  - Every button press gets visible feedback on the first press (repo rule): set the status text before the command goes out.
  - R7: test accounts are `Player_1234`, and URLs are under `example.org`.
  - R8: command and field names come from C18.1.
  - A REPL-style logic UAT (AGENTS.md) is part of this packet. In the report, list each new test as the user-visible step it proves.
- **The defects.**
  - F10: `render` (`sign-in-view.ts` l.67-72) hides every control when `available` is false, so a player signed in offline cannot sign out.
  - F12: `needs_link` has no way back to the provider list.
  - F09 UI: after `browser_failed`, nothing waits on the flow.
  - Add method in `account-view.ts` shows the "copy this address" text, but not the address.
  - Sign in again with no last-used provider (`account-view.ts` l.173-174) ends silently.
- **Contract excerpt (C18.1):**
  - New field: `storage_warning: null | "session_only" | "sign_out_incomplete" | "store_unreadable"`.
  - New command `resume`, and new failure `no_pending_flow`. A `begin` or `add_method` that fails `browser_failed {url}` has parked its flow. The frontend sends `resume` at once after showing the URL; `resume` answers what the original command would have.
  - `available: false` may carry `state: signed_in`. The panel then keeps the account summary and **Sign out** visible, and hides only network actions.
  - In `needs_link`, the panel offers **Back to sign in**, which sends `sign_out`.

Files:

1. `crates/launcher/desktop/frontend/src/sign-in-contract.ts`:
   - `SignInStatus` gains `storage_warning: Schema.optionalKey(Schema.NullOr(Schema.Literals(['session_only', 'sign_out_incomplete', 'store_unreadable'])))`. Make it optional so older fixtures still decode.
   - `SignInFailure` gains `failureKind('no_pending_flow')`.
2. `crates/launcher/desktop/frontend/src/sign-in-workflow.ts`:
   - `failureText` gains `case 'no_pending_flow': return 'That sign-in expired. Sign in again.';`.
   - New export `storageWarningText(w)`:
     - `session_only`: "Signed in for this session only: this computer's credential store would not save the sign-in.";
     - `sign_out_incomplete`: "Signed out here, but the saved sign-in could not be removed from this computer's credential store. It will not be used again in this session.";
     - `store_unreadable`: "This computer's credential store could not be read, so no saved sign-in was restored.".
3. `crates/launcher/desktop/frontend/src/sign-in-view.ts`:
   - `render` offline branch (l.67-72):
     - Hide the panel only when `!s || (!s.available && !s.play_refusal && !s.storage_warning && s.state.state !== 'signed_in')`. An offline store warning (for example `sign_out_incomplete` after an offline Sign out, or `store_unreadable` at start-up) shows in the status line with no controls, as a play refusal does (ruling CT-91-1).
     - When `!s.available` and the state is `signed_in`, show the heading ("Your account"), `stateLine` and `signOut`. Hide `providers`, `signupNote`, `linkBox` and `cancel`.
     - Otherwise, offline, keep today's behaviour.
   - A new `back = button('Back to sign in')` at the end of `linkBox`, shown only in `needs_link`. Its press runs: `linkKind = null; password.input.value = ''; code.input.value = ''; send({command: 'sign_out'}, 'Going back…')`.
   - Resume:
     - In `exchange`'s failure branch, when the request was `begin` and the failure is `browser_failed`, set a flag `resumeNext = true`.
     - In `send`'s `.then` chain (after `finally`), when `resumeNext` is set, clear it and run: `beginning = true; busy = true; render();` then `exchange({command: 'resume'})`, then `finally` clears `busy` and `beginning`.
     - Keep the copy-address status text and the URL box while it waits. A successful `resume` clears the URL box.
   - `settle` and `inspect`: when `next.storage_warning` is set and no refusal or failure text is showing, `setStatus(storageWarningText(next.storage_warning))`.
4. `crates/launcher/desktop/frontend/src/account-view.ts`:
   - Add a URL box like `sign-in-view.ts` l.44-45, shown on an `add_method` `browser_failed` with the failure's `url`. Then send `resume` with `waiting = true`, so Cancel shows, and clear the box when it settles.
   - `signAgainPress` l.173-174: `if (!providerId) { setStatus('Sign in again from the sign-in panel, then retry.'); return; }`.
   - A `cancelled` reply to the `begin` already returns `null` (no retry). Keep it, and add the test below.
   - `signAgainPress`: a `begin` that fails `browser_failed` shows the address in the same URL box, sends `resume` (Cancel shows), clears the box when it settles, and on success runs the retry as a direct `begin` success would. A failed `resume` keeps `reauth` as a failed `begin` does (ruling CT-91-1).
5. Tests: `sign-in-view.test.ts`, `account-view.test.ts` and `sign-in-workflow.test.ts`, or new sibling `*.test.ts` files where R4 says so. The existing tests at `sign-in-view.test.ts` l.39-55 use `state: signed_out` and stay unchanged. Any other existing test that has to change: say which, and why, in the report.

Tests (unit, `node:test`):

| Test | Fails on revert because |
|---|---|
| `offline with a restored sign-in shows the account and Sign out`. Inspected `status({available: false, providers: [], state: signed_in Player_1234})`: the panel is visible, Sign out is visible, and no provider button exists. Press Sign out: the status text is "Signing out…" before the reply; `calls` ends with `sign_out`; the reply `status({available: false, state: signed_out})` hides Sign out. | Today the panel is hidden and Sign out is unreachable. |
| `ticket_unavailable while signed in offline offers Sign out`. The same with `play_refusal: 'ticket_unavailable'`: the refusal text shows, and Sign out is visible and sends `sign_out`. | Today Sign out is hidden there (l.70). |
| `offline Sign out with sign_out_incomplete keeps the warning visible`. Offline `signed_in`; Sign out answers `available: false, signed_out, storage_warning: 'sign_out_incomplete'`: the panel stays visible with `storageWarningText('sign_out_incomplete')`, Sign out is hidden and no provider button exists. | Without `!s.storage_warning` the panel hides and the warning is never seen (ruling CT-91-1). |
| `needs_link offers Back to sign in`. Begin, then `needs_link`. Type a password, press Link (it is now confirming), press Back to sign in: `calls` ends with `sign_out`, the password and code fields are empty, and Remove password and link is hidden. The reply `signed_out` shows the provider buttons. Begin again with a second provider: `calls` has no `link_*` entry. | There is no such button today. |
| `browser_failed sends resume and shows the address while it waits`. `begin` throws `{error: 'browser_failed', url: 'https://example.org/auth'}`, and `resume` answers signed in. `calls` is `begin`, then `resume`. While `resume` is pending, the URL field holds the address and Cancel is visible. After it, the panel shows the account and the URL box is hidden. | Today nothing is sent after `browser_failed`. |
| `resume no_pending_flow says the sign-in expired`. | The text is new. |
| `a store warning is shown`. Inspected `status({storage_warning: 'sign_out_incomplete'})`: the status text is `storageWarningText('sign_out_incomplete')`. | The field is new. |
| `add method browser_failed shows the address and resumes` (account-view). | Today account-view shows the text without the address, and nothing is sent. |
| `sign in again browser_failed resumes, then retries` (account-view). `calls` ends `begin`, `resume`, `unlink`; the address shows while `resume` waits. | Today the parked flow is never resumed and the retry never runs (ruling CT-91-1). |
| `sign in again with no last-used provider says what to do` (account-view). The status text is the new line, not empty. | Today the press ends silently. |
| `sign in again whose begin is cancelled does not retry` (account-view). `begin` throws `{error: 'cancelled'}`; `calls` has no second `unlink` or `add_method`. | Guards F06 on the UI side; it passes today and must keep passing. |
| `workflow decodes storage_warning and no_pending_flow` (`sign-in-workflow.test.ts`). | The decoders are new. |

**Checks, in order:** F1. **Docs rows:** none here; CT-92 updates `docs/client/launcher-sign-in-guide.md` for the new controls.
