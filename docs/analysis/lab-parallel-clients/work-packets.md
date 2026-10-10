# Lab parallel clients: work packets

> Type: work packets. Audience: `packet-coder` workers (Haiku) and
> `packet-reviewer` reviewers (Sonnet). Ledger and design: [README.md](README.md).
>
> Each packet names every file it touches and the checks it runs. Crate:
> `cimmeria-lab` (`crates/lab/`). All checks run through the build lane from
> PowerShell, in this order:
>
> ```powershell
> bash tools/build-lane/lane.sh cargo fmt -p cimmeria-lab
> bash tools/build-lane/lane.sh cargo clippy -p cimmeria-lab --all-targets -- -D warnings
> bash tools/build-lane/lane.sh cargo test -p cimmeria-lab
> ```
>
> The commit message is the packet's subject line, a blank line, a short
> body, and the attribution lines the coordinator's brief gives.

## Contents

- [LP-01 Per-instance user folder and client cap](#lp-01-per-instance-user-folder-and-client-cap)
- [LP-02 One lease book per supervisor](#lp-02-one-lease-book-per-supervisor)
- [LP-03 Lab tooling](#lp-03-lab-tooling)
- [LP-05a Instance registry and routing](#lp-05a-instance-registry-and-routing)
- [LP-05b Lease tools and UAT p2 across instances](#lp-05b-lease-tools-and-uat-p2-across-instances)
- [LP-06 Watchdog boot grace](#lp-06-watchdog-boot-grace)
- [LP-04 Docs and memory](#lp-04-docs-and-memory)
- [LP-07 Live UAT](#lp-07-live-uat)

## LP-01 Per-instance user folder and client cap

**Worktree:** `.claude/worktrees/lab-parallel` (branch
`feat/lab-parallel-instances`; the module below is already written there).
**Subject:** `feat(lab): LP-01 per-instance Firesky folder for every lab client (#1312)`

Files:

1. `crates/lab/Cargo.toml`: in the `windows-sys` features list add, with a
   comment line above it:

   ```toml
       # supervisor/instance_profile.rs: read the unexpanded Personal shell folder.
       "Win32_System_Registry",
   ```

2. `crates/lab/src/supervisor/mod.rs`:
   - After `pub mod instance;` add `pub mod instance_profile;`.
   - In `launch_client`, replace
     `let envs = instance::launch_env(&install_dir, inst);` with:

     ```rust
     let mut envs = instance::launch_env(&install_dir, inst);
     // Every lab client gets its own Firesky folder, so two clients never
     // lock each other's cooked-data cache (#1312).
     if let Some(profile) = instance_profile::prepare(&install_dir, inst.unwrap_or("default")) {
         envs.push(profile);
     }
     ```

     (`prepare` copies files on its first call for an instance; that is a
     one-off of about 30 MB, acceptable on the launch path.)

3. `crates/lab/src/supervisor/instance.rs`:
   - `CEILING_MAX_CLIENTS`: `4` to `5`; its doc comment becomes "Ceiling for
     `CIMMERIA_LAB_MAX_CLIENTS`: the five seeded lab accounts (`lab`, `lab2`
     to `lab5`). Each client is a 32-bit process with a D3D9 device and its
     own Mercury session."
   - In the module doc's table add the row
     `| the shared `Documents\My Games\Firesky` folder | `sessions/instances/<label>/profile` as the game's `USERPROFILE` ([`super::instance_profile`]) |`
     and change the first paragraph's "two `SGW.exe` clients" to "up to five
     `SGW.exe` clients".
   - Keep `DEFAULT_MAX_CLIENTS = 2` (LP-05a changes it).

4. `crates/lab/src/supervisor/instance_profile.rs`: no change unless a check
   fails. It already has its tests.

Checks: the three lane commands. Expect the `instance_profile::tests` and
`instance::tests` to pass.

Reviewer focus: `prepare` falls back to the shared folder on every error
path; the default instance is redirected too; `RegGetValueW` buffer sizing;
the seed's temp-then-rename.

## LP-02 One lease book per supervisor

**Worktree:** new, `bash tools/build-lane/mk-worktree.sh feat/lab-lease-per-instance lp02`.
**Subject:** `feat(lab): LP-02 one lease book per supervisor`

Files:

1. `crates/lab/src/supervisor/mod.rs`, `Supervisor::new`: replace
   `leases: crate::lease::global(),` with
   `leases: Arc::new(crate::lease::LeaseBook::default()),`. Update the field
   doc on `leases` to: "This instance's lab lease ([`crate::lease`]): one book
   per supervisor, so each lab instance (each lab account) is leased on its
   own. The watchdog relaunches a dead client only while it is held."
2. `crates/lab/src/lease/mod.rs`:
   - Delete `static GLOBAL`, `pub fn global()` and the now-unused `OnceLock`
     import.
   - Module doc: replace the paragraph starting "One book per process" with:
     "One book per supervisor: each lab instance (one lab account, one
     client) has its own lease, so several agents can drive several clients
     at once. The in-process second-player supervisor of a UAT run has its
     own book too; the run's permit (from the first player's book) covers its
     actions ([`permit::ensure`] checks the permit's book)."
3. `crates/lab/src/server/uat.rs`, the doc comment above `static P2_LAB`: add
   the sentence "Its watchdog consults its own lease book, which a UAT run
   does not hold, so it does not relaunch p2 after a crash."
4. A new test in `crates/lab/src/lease/tests.rs` (append):

   ```rust
   /// Regression guard (LP-02): two supervisors never share a lease.
   #[tokio::test]
   async fn each_supervisor_has_its_own_lease_book() {
       use crate::client::BridgeClient;
       use crate::supervisor::{Supervisor, SupervisorConfig};
       let mk = || {
           Supervisor::new(
               std::sync::Arc::new(BridgeClient::new("127.0.0.1:9".into(), String::new())),
               SupervisorConfig::from_env(),
           )
       };
       let (a, b) = (mk(), mk());
       assert!(!std::sync::Arc::ptr_eq(a.leases(), b.leases()));
       let req = AcquireRequest { owner: "a".into(), purpose: "p".into(), ..Default::default() };
       a.leases().acquire(req).unwrap();
       assert!(a.leases().is_held());
       assert!(!b.leases().is_held(), "a lease on one instance must not hold another");
   }
   ```

   Check the existing imports at the top of `lease/tests.rs` (`AcquireRequest`
   may already be in scope through `use super::*;`), and the real
   `BridgeClient::new` signature in `crates/lab/src/client.rs`; adapt the
   two constructor lines to it.

Checks: the three lane commands. Then `git grep -n "lease::global"` in the
worktree must print nothing.

Reviewer focus: any remaining caller of `global()`; the UAT run's p2 actions
still pass the permit check; the daemon's single supervisor behaves as
before.

## LP-03 Lab tooling

**Worktree:** new, `bash tools/build-lane/mk-worktree.sh feat/lab-instances-tooling lp03`.
**Subject:** `feat(lab): LP-03 instances.ps1 and multi-instance labd.env`
No Rust. PowerShell and docs only.

Files:

1. New `tools/lab/instances.ps1` (CRLF, same header style as
   `tools/lab/daemon.ps1`: a `<# .SYNOPSIS .DESCRIPTION .PARAMETER .EXAMPLE #>`
   block, `[CmdletBinding()]`, `Set-StrictMode -Version Latest`,
   `$ErrorActionPreference = 'Stop'`). Parameters:
   `[ValidateSet('init','status')] [string]$Command`, `[string]$InstallDir`
   (default: `$env:CIMMERIA_LAB_INSTALL_DIR`, else the value of
   `CIMMERIA_LAB_INSTALL_DIR` in `%LOCALAPPDATA%\cimmeria-lab\labd.env`),
   `[int]$Count = 5`, `[switch]$Force`.
   - `init`: read `<InstallDir>\Binaries\sessions\lab-account.json` (it must
     exist; else stop with a message naming it). For `n` in `2..$Count`, write
     `lab-account.p<n>.json` beside it, a copy with `username` set to
     `lab<n>` and `character` set to `Lab` + the English ordinal word
     (`Labtwo`, `Labthree`, `Labfour`, `Labfive`). Skip an existing file
     unless `-Force`. Never print the password. Print one line per file:
     written or kept.
   - `status`: for the default instance and `p2..p<Count>`, print the label,
     the account file's `username` and `character` (or "missing"), whether
     `Binaries\sessions\instances\<label>\profile\Documents\My Games\Firesky\SGWGame`
     exists ("seeded" / "not seeded"), and whether
     `Binaries\sessions\instances\<label>\lab-instance.json` names a live
     process ("client running pid N" / "no client").
2. `tools/lab/daemon.ps1`: in the `.DESCRIPTION` block, after the `labd.env`
   line, add a paragraph: "To host several lab clients in this one daemon,
   set `CIMMERIA_LAB_INSTANCES=default,p2,p3,p4,p5` in labd.env (each gets
   bridge port `CIMMERIA_LAB_BRIDGE_PORT` + its position, so 8770..8774)
   and run `tools/lab/instances.ps1 init` once to write the account files."
   No code change.
3. `.mcp.json.example`: next to the `cimmeria-lab` (HTTP daemon) entry, add a
   comment line (the file is JSON with `//` comments only if it already has
   them; if it is strict JSON, add the same text to the nearest description
   string instead) saying that one daemon serves every lab account and that
   tools take an optional `instance` (`default`, `p2`... or `lab`, `lab2`...).

Checks: `pwsh -NoProfile -File tools/lab/instances.ps1 status` runs without
error (it may print "missing"); `pwsh -NoProfile -Command "Get-Command -Syntax tools/lab/instances.ps1"`
parses. `bash tools/lint-md.sh` on changed Markdown is warn-only.

Reviewer focus: the password never printed; `-Force` respected; strict mode
errors on a missing property; CRLF.

## LP-05a Instance registry and routing

**Depends on:** LP-02 merged. **Worktree:** new off `origin/main`,
`bash tools/build-lane/mk-worktree.sh feat/lab-multi-instance lp05a`.
**Subject:** `feat(lab): LP-05a one daemon hosts every lab instance, routed per call`

Files:

1. New `crates/lab/src/server/instances.rs`:

   ```rust
   //! The lab instances one `cimmeria-lab` process hosts, and which one a
   //! tool call is for (#1312, LP-05a). Each instance is one lab account
   //! and one `SGW.exe` with its own supervisor, watchdog and lease book.

   use std::sync::Arc;
   use crate::supervisor::{instance, Supervisor};

   /// Comma-separated instance labels to host: `default,p2,p3,p4,p5`.
   /// Unset or empty: the one instance `CIMMERIA_LAB_INSTANCE` names.
   pub const INSTANCES_ENV: &str = "CIMMERIA_LAB_INSTANCES";
   /// The optional argument every tool takes when more than one instance
   /// is hosted.
   pub const INSTANCE_ARG: &str = "instance";

   /// One hosted instance.
   #[derive(Clone)]
   pub struct Hosted {
       /// `default` or the instance name (`p2`).
       pub label: String,
       pub supervisor: Arc<Supervisor>,
   }

   /// Every hosted instance; the first is the target when a call names none.
   #[derive(Clone)]
   pub struct Instances { list: Vec<Hosted> }

   /// Parse `INSTANCES_ENV`: `None` entries are the default instance.
   /// Refuses duplicates and bad names (`instance::validate_name`).
   pub fn parse_list(raw: &str) -> Result<Vec<Option<String>>, String> { ... }
   ```

   `parse_list`: split on `,`, trim, skip empty items; `default` (any case)
   becomes `None`, anything else goes through `instance::validate_name`;
   error on a duplicate label (case-insensitive) or an empty list.
   `Instances` methods:
   - `pub fn new(list: Vec<Hosted>) -> Self` (panics on an empty list: a
     programming error).
   - `pub fn first(&self) -> &Hosted`, `pub fn len(&self) -> usize`,
     `pub fn iter(&self) -> impl Iterator<Item = &Hosted>`.
   - `pub fn by_label(&self, name: &str) -> Option<&Hosted>`: case-insensitive
     match on `label`.
   - `pub fn by_lease(&self, lease_id: &str) -> Option<&Hosted>`: the instance
     whose `supervisor.leases().holds(lease_id)` is true.
   - `pub fn labels(&self) -> Vec<String>`.

   Unit tests: `parse_list` (default + names, case, duplicates, bad name,
   empty); `by_label` case-insensitivity. (`by_lease` is covered in LP-05b.)

2. `crates/lab/src/lease/mod.rs`: add to `impl LeaseBook`

   ```rust
   /// Whether `id` is the current, unexpired lease (no renewal).
   pub fn holds(&self, id: &str) -> bool {
       let mut st = self.lock();
       st.expire_if_due(now_ms());
       st.current.as_ref().is_some_and(|l| l.lease_id == id)
   }
   ```

3. `crates/lab/src/server/mod.rs`:
   - `mod instances;` beside the other `mod` lines; `pub use instances::{Hosted, Instances, INSTANCES_ENV};`
   - `LabServer` gains `instances: Arc<Instances>`. `LabServer::new(supervisor)`
     builds a one-entry `Instances` (label from
     `supervisor`'s config instance, else `default`; add a
     `pub fn instance(&self) -> Option<&str>` accessor on `Supervisor` in
     `supervisor/mod.rs` returning `self.config.instance.as_deref()` if none
     exists). Add `pub fn new_multi(instances: Instances) -> Self` that sets
     `supervisor` to `instances.first().supervisor.clone()` and otherwise
     matches `new`.
   - Add
     ```rust
     /// The server a call runs on: a clone whose supervisor is the target
     /// instance's. Takes the `instance` argument out of the call.
     fn route(&self, request: &mut CallToolRequestParams) -> Result<LabServer, McpError>
     ```
     Order: (1) `instance` argument (a string): remove it from the
     arguments; match `by_label`, else match the account name (LP-05b
     extends this; for now label only); unknown → `McpError::invalid_params`
     naming the hosted labels. (2) else a string `lease_id` argument (do not
     remove it; `gate_call` does) → `by_lease`; no match → fall through.
     (3) else `first()`. Return `LabServer { supervisor: hosted.supervisor.clone(), ..self.clone() }`.
   - `call_tool`: `let target = self.route(&mut request)?;` then use `target`
     for `gate_call`, `ToolCallContext::new(&target, request, context)` and
     `target.tool_router.call(tcc)`.
   - `list_tools`: when `self.instances.len() > 1`, add to every tool's
     schema an optional property `instance`:
     `{"type":"string","description":"Which lab client: <labels joined by ', '> or its account (lab, lab2, ...). Default: the instance your lease_id belongs to, else <first label>."}`.
     Put this in a new `fn advertise_instance(tools: Vec<Tool>, labels: &[String]) -> Vec<Tool>`
     in `server/instances.rs`, called after `lease::advertise_lease`.
4. `crates/lab/src/main.rs`, `build_server`: read `INSTANCES_ENV`. Unset or
   empty: today's code path. Set: `parse_list`; for each entry `i`,
   `let mut config = SupervisorConfig::from_env(); config.instance = entry.clone(); config.port = base_port + i as u16;`
   where `base_port` is the first `from_env()` port; the bridge address is
   `format!("127.0.0.1:{}", config.port)`; the bridge token is
   `CIMMERIA_LAB_TOKEN` for `i == 0` only, empty otherwise; spawn
   `lease::spawn_sweeper` per supervisor; log one INFO line per instance
   (`instance`, `port`); build `LabServer::new_multi`. Refuse to start
   (`anyhow::bail!`) on a parse error.
5. `crates/lab/src/supervisor/instance.rs`: `DEFAULT_MAX_CLIENTS` stays `2`,
   but `max_clients_from_env` becomes
   `max_clients_from_env(hosted: usize) -> usize` using `hosted.max(DEFAULT_MAX_CLIENTS)`
   as the default (still clamped to the ceiling); add
   `pub fn hosted_count_from_env() -> usize` (the number of `INSTANCES_ENV`
   entries, else 1; parse errors count as 1) and call
   `instance::max_clients_from_env(instance::hosted_count_from_env())` in
   `Supervisor::start`. Keep `INSTANCES_ENV`'s single definition in
   `server/instances.rs` and import it, or move the constant to
   `supervisor/instance.rs` and re-export from `server/instances.rs`:
   whichever compiles without a cycle (supervisor must not depend on server;
   so move it to `supervisor/instance.rs`). Update `clamp_max`'s tests.

Tests (in `server/instances.rs` `mod tests`): the `parse_list` and
`by_label` tests above; `advertise_instance` adds the optional property and
leaves `required` alone; with a one-entry `Instances`, `list_tools` output
is unchanged (assert on `advertise_instance` not being applied when
`len() == 1`, by testing a small helper `fn instance_arg_wanted(len) -> bool`).

Checks: the three lane commands.

Reviewer focus: a lease id from instance A used with `instance: "p2"` (must
be refused by p2's book, not silently accepted); the `instance` argument
reaching a tool's argument struct (it must be stripped); routing of
`lab_lease_acquire` (no lease id: goes to the named or first instance);
port collisions with `CIMMERIA_LAB_BRIDGE_PORT` overrides; the stdio mode
path unchanged.

## LP-05b Lease tools and UAT p2 across instances

**Depends on:** LP-05a merged. **Worktree:** new off `origin/main`,
`bash tools/build-lane/mk-worktree.sh feat/lab-lease-tools-multi lp05b`.
**Subject:** `feat(lab): LP-05b per-account lease tools, account-name routing, UAT p2 from the registry`

Files:

1. `crates/lab/src/server/instances.rs`: `Hosted` gains
   `pub account: Option<String>`, read once at startup from the instance's
   account file (`instance::account_path(install_dir, label)`, its `username`
   field; `None` if unreadable). `by_label` also matches `account`
   case-insensitively. Add `pub fn by_name(&self, name) -> Option<&Hosted>`
   if clearer, and use it in `route`.
2. `crates/lab/src/server/lease.rs`:
   - `lab_lease_acquire`: the grant JSON gains `"instance": <label>` and
     `"account": <account>` (add them to the `Value` after `grant_json`). The
     tool description: replace "One holder at a time across every session"
     with "One holder per lab instance (one lab account, one client) at a
     time; pass `instance` to pick which client, else the default".
   - `lab_lease_status`: when more than one instance is hosted and the call
     names none (the routed instance is then the first; detect "named none"
     by having `route` record it: add a `routed_explicitly: bool` field on
     `LabServer`, default `false`, set to `true` in `route` when the call
     carried `instance` or a matching `lease_id`), return
     `{"instances":[{"instance":label,"account":acct,"lease":<status>,"client_pid":pid-or-null}, ...]}`.
     Else today's single status. `client_pid`: use the supervisor's existing
     status accessor (see `lab_client_status`'s implementation in
     `server/mod.rs` for the call).
   - Refusals from `gate_call` append " (instance <label>)" to the message.
3. `crates/lab/src/server/uat.rs`, `p2_lab`: if the server's `instances`
   hosts the p2 label, return a `LabServer` routed to it instead of building
   the in-process one. Change `p2_lab(name)` to take `&LabServer` (the
   caller has `self`) and return `LabServer` (clone); keep the
   `OnceLock` fallback for a single-instance daemon.

Tests: routing by account name; a lease acquired on `p2` is not accepted for
a call routed to `default` (two supervisors from `Supervisor::new` with
distinct configs; call `gate_call` on each routed clone, or test
`by_lease` plus `LeaseBook::check` directly); `lab_lease_status` multi
shape (unit-test the JSON builder as a pure function over a slice).

Checks: the three lane commands.

Reviewer focus: the lease id never appears in `lab_lease_status`; the UAT
runner's p2 still works on a single-instance daemon; two-player rows with a
registry p2 do not leave the p2 lease held after the run.

## LP-06 Watchdog boot grace

**Worktree:** new, `bash tools/build-lane/mk-worktree.sh feat/lab-watchdog-boot-grace lp06`.
**Subject:** `fix(lab): LP-06 watchdog boot grace for a client whose bridge is still coming up`

Files:

1. `crates/lab/src/supervisor/stall_grace.rs`: add

   ```rust
   /// How long after a launch a client that has never answered the bridge
   /// is left alone. With several clients booting at once a bridge took
   /// about 25 s to come up (2026-10-10), past the 5-failure rule.
   pub const BOOT_GRACE: Duration = Duration::from_secs(90);

   /// Whether a failed heartbeat should be ignored: the bridge has not
   /// answered once since this launch and the launch is younger than
   /// `grace`.
   pub fn in_boot_grace(answered_once: bool, since_launch: Duration, grace: Duration) -> bool {
       !answered_once && since_launch < grace
   }
   ```

   with unit tests in `stall_grace_tests.rs`: true before the first answer
   inside the grace; false after the grace; false once answered.
2. `crates/lab/src/supervisor/watchdog.rs`, `watchdog_loop`: add
   `let mut answered_once = false;` before the loop. In the `Ok(count)` arm
   set `answered_once = true;`. In the `Err(_)` arm, after the
   `is_alive` death check and before building `Heartbeat::Failed`, add:

   ```rust
   let since = std::time::Duration::from_millis((now_ms() - started_ms).max(0) as u64);
   if stall_grace::in_boot_grace(answered_once, since, stall_grace::BOOT_GRACE) {
       tracing::debug!(pid = my_pid, since_ms = since.as_millis() as u64,
           "bridge not up yet; boot grace in effect");
       continue;
   }
   ```

   Check `started_ms`'s type (it is `now_ms()`, an `i64`) and adapt the cast.
   `continue` skips the tracker for that poll only.

Checks: the three lane commands.

Reviewer focus: a client that dies during boot is still detected (the
`is_alive` check runs before the grace); a client that answered once and
then hangs gets no grace; the grace does not mask a client stuck before its
first answer forever (90 s cap).

## LP-04 Docs and memory

**Depends on:** LP-01 and LP-05b merged (write against their final names).
**Worker:** `documentation-writer` (not Haiku). **Worktree:** new.
**Subject:** `docs(lab): parallel lab clients, one daemon, a lease per account (LP-04)`

Update, each in the same voice as its file (docs are CRLF):

- `docs/guides/live-research-lab.md`: rewrite "Two clients: two-player
  scenarios" as "Parallel clients (up to five)": `CIMMERIA_LAB_INSTANCES`,
  `tools/lab/instances.ps1 init`, the per-instance profile and its seed,
  `CIMMERIA_LAB_SHARED_USER_DIR`, the `instance` argument, a lease per
  instance, the OneDrive caveat, window tiling is the operator's (no tool).
- `docs/architecture/live-research-lab.md`: an addendum: one daemon, many
  instances; the lease per instance; routing order.
- `docs/reverse-engineering/findings/multi-client-lab.md`: results of the
  live test plan (LT-1 to LT-7 as run on 2026-10-10), root cause, the
  measurements in [README.md](README.md), confidence raised to HIGH.
- `docs/guides/automated-uat.md`: two-player rows use the registry's p2
  when hosted.
- `docs/analysis/lab-automation/tooling-backlog.md`: note the camera gap
  (#1243, PR #1309) and the DLL `SHGetFolderPathW` hook follow-up.
- `.claude/agent-memory/main-session/`: one reference memory,
  `reference_lab_parallel_clients_2026_10_10.md`, and its `MEMORY.md` line.
- This ledger: statuses, close-out.

## LP-07 Live UAT

**Coordinator only** (with a `lab-driver` for the in-game steps). After
everything is merged and the lab binaries are installed from `main`
(`tools/lab/install.ps1`):

1. Set `CIMMERIA_LAB_INSTANCES=default,p2,p3,p4,p5` in `labd.env`, run
   `tools/lab/instances.ps1 init`, restart the daemon.
2. Five agents (or one, sequentially) each acquire a lease with
   `instance: lab`..`lab5`, `lab_ensure_in_world`, `.gotolocation DebugArea`.
3. Expect: five clients in world at once; five `lab.lease` holders in
   `lab_lease_status`; a lease id for `lab2` refused on `lab3`; zero
   `Error opening static cache archive` lines in `SGWDebugLog.log`; no
   `full_resync` beyond the routine category 11; no watchdog kill during
   boot.
4. Record the result in this ledger and on #1312, then close #1312.
