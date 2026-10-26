# client-tls-oidc work packets

> **Type:** Reference
> **Audience:** The orchestrator, ops, packet-coders and packet-reviewers
> **Last updated:** 2026-10-26
> **Companions:** [Ledger](README.md), [contract](contract.md), [resume card](RESUME.md), waves [1](packets-wave1.md), [2](packets-wave2.md), [3](packets-wave3.md), [4](packets-wave4.md), [5](packets-wave5.md), [6](packets-wave6.md), [7](packets-wave7.md), [8](packets-wave8.md), [9](packets-wave9.md), [10](packets-wave10.md), [PR #2031 review ruling](worknotes/pr2031-ruling.md)

Eleven waves and a tail. The design, findings and decisions (D-CT1..D-CT29) are in the [ledger](README.md); the shared types, wire shapes, DDL and Lua contract are in [contract.md](contract.md). This file holds the dispatch rules, the rules every packet carries, the checks, and wave 0. Line numbers are from `main` `2019922fe`; re-grep every anchor before editing.

## Dispatch rules

- **Worktrees.** Ask `ops` for one worktree per packet: `pwsh tools/build-lane/mk-worktree.ps1 client-tls-oidc/CT-NN-<slug> cto-NN`, at the wave's base (main for wave 0 and 1; the previous wave's merge commit on main after that). Worktrees are `<repo>\.claude\worktrees\cto-NN`. A packet that depends on another packet of the same wave gets its branch at that packet's reviewed head (`branch-at.ps1`).
- **Integration (amended 2026-10-24, owner: one PR at the end of the campaign).** The whole campaign ships as **one PR** from a single integration worktree `client-tls-oidc/campaign` (`cto-camp`, created by `ops` from main when the first packet is reviewed `ship`). Packets merge into it with `merge-noff.ps1` in each wave's listed order, wave after wave; later waves branch from its head. Wave and campaign review loops run on that branch. No wave PRs and no per-packet PRs; wave 5's gated owner/colo/lab steps that need no code change run against the integration branch or main as their gates open, and any wave 5 code packet merges into the same branch before the one PR opens. Where a packet or wave section below says "ships as one PR", read "merges into the campaign branch".
- **Agents.** Code packets go to `packet-coder`; doc packets to `documentation-writer`; lab packets follow the `lab-uat` skill (lab use approved in principle 2026-10-24; first confirm the lab is free and the lab MCP is up; a Haiku `lab-driver` does the driving); ops checks go to `ops`. Each packet gets a packet-review loop. Waves 2 and 3 also get one read-only `network-security-auth` pass on the integration head before the wave PR opens (the brief: "review the auth surface of this wave against contract.md C1-C7 and the design's § 5.6; numbered fix notes only").
- **Owner gates.** Lab use (CT-04, CT-58, CT-U1..U6) is approved in principle (2026-10-24); at execution time confirm the lab is free and the lab MCP is up. CT-52's DNS record is an `az network dns` command a `hands-on` agent runs (ops cannot run az); the 443/tcp edge forward in UniFi stays an owner-confirmed change. Provider registrations (CT-53) are owner actions; the orchestrator asks, never an agent. CT-56 waits on D-CT16. CT-55 and CT-59 are Cancelled (D-CT11).
- **Report path:** `<worktree>\.claude\reports\client-tls-oidc\<packet>-<role>.md`. **Return line:** `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`.
- **Retire** each packet worktree as soon as it is merged into the integration branch (`rm-worktree.ps1 --merged-into <PR>` after the wave PR merges, or at once when merged into the integration branch). Never `git worktree prune`.
- Commit attribution, at the end of every commit message: a `Co-Authored-By` line naming the model that wrote the commit, then the session line. Sonnet implementers (fix rounds, ruling fixes, `rust-gameserver-dev`) and every other Sonnet session:

```text
Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01WRynanEMGjFbBzPn7s7aWe
```

  A `packet-coder` session (Haiku, per its agent file) writes `Co-Authored-By: Claude Haiku 5.5 <noreply@anthropic.com>` instead, with the same session line. Never name a model that did not write the commit; never reword a pushed or reviewed commit to change it (CT-33-ruling-1 D4).

## Rules every packet carries

- **R1 Shell.** `Host: windows`: PowerShell only, never the Bash tool. Start every command with `Set-Location <your worktree>;`. Use absolute paths for every file write (`[IO.File]` relative paths land in the main checkout).
- **R2 Lane.** Every compiling cargo call goes through `pwsh tools/build-lane/lane.ps1 cargo ...` in the foreground. Never call cargo directly; never pass `--target-dir`. Read the lane log under `logs/<worktree>/` before rerunning; at most 5 fix rounds per check.
- **R3 Scope.** Edit only the files your packet names. No `git stash`, no `git worktree prune`, no rebase, no push. When every check passes, `git add <explicit paths>` and commit at once with the packet's subject and the attribution lines.
- **R4 Caps.** Soft cap 500 lines, hard cap 700 per file. A file the packet names that would pass 700: split it along the seam the packet names, or stop and report `blocked`. Never grow a file that is already over 700.
- **R5 Line endings.** `docs/**/*.md` and `crates/launcher/desktop/docs/*.md` are CRLF; check `([IO.File]::ReadAllText('<abs>') -match "[^\r]\n")` is `False`. The 017 patch files and the golden fixture are ASCII CRLF and `-text`; never let an editor convert them.
- **R6 Tests.** Every behaviour change has a test that fails when the change is reverted; name it and say why in the report. Unit tests sit in the module (`#[cfg(test)] mod tests`, or a sibling `<file>_tests.rs` via `#[path]`). Live-DB tests: `live_db` in the test or module name, `require_db_or_skip!`, `test_support::database_url()`, sentinel ids that fit `i32`, cleanup by exact sentinel ([TESTING.md](../../../TESTING.md)).
- **R7 Secrets.** No IPs, real client ids, secrets, tokens, emails or account names in code, tests, fixtures or reports. Test values: `example.org`, `203.0.113.x`, `Player_1234`. Never log a code, token, ticket, subject, email or password (contract C11).
- **R8 Contract.** Names, routes, columns and error codes come from [contract.md](contract.md). A gap: report `blocked - contract gap: <what>`; never invent one.
- **R9 Generated blocks.** Never hand-edit a `<!-- gen:NAME -->` block.
- **R10 New crate.** A packet that adds a crate does the whole checklist it names (workspace members, `cimmeria-workspace-hack` dependency, then `cargo hakari generate` and `cargo hakari manage-deps --yes`, which run directly because hakari does not compile, per [pre-pr-checks.md](../../agents/pre-pr-checks.md)).

### Checks

| ID | Command (prefix every one with `Set-Location <worktree>;`) |
|---|---|
| S1 `<pkg>` | `pwsh tools/build-lane/lane.ps1 cargo fmt -p <pkg>` then `pwsh tools/build-lane/lane.ps1 cargo clippy -p <pkg> --all-targets -- -D warnings` |
| S2 `<pkg>` | `pwsh tools/build-lane/lane.ps1 cargo nextest run -p <pkg>` |
| S3 `<filter>` | `pwsh tools/build-lane/live-db-test.ps1 <filter>` |
| S4 | `cargo hakari generate --diff` and `cargo hakari manage-deps --dry-run` (direct; hakari does not compile) |
| D1 | `pwsh tools/build-lane/lane.ps1 cargo fmt --manifest-path crates/launcher/desktop/Cargo.toml --all` |
| D2 `<pkg>` | `pwsh tools/build-lane/lane.ps1 cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p <pkg> --all-targets -- -D warnings` |
| D3 `<pkg>` `<f>` | `pwsh tools/build-lane/lane.ps1 cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p <pkg> <f>` (cargo test, not nextest, in the desktop workspace) |
| F1 | `npm ci --ignore-scripts --prefix crates/launcher/desktop/frontend; npm run check --prefix crates/launcher/desktop/frontend; npm test --prefix crates/launcher/desktop/frontend` |
| L1 | `python -m pip install lupa` once, then `python data/client-patches/017-launcher-sign-in/test/run_lupa.py` |

`<pkg>` for the main workspace: `cimmeria-auth`, `cimmeria-identity`, `cimmeria-server`, `cimmeria-launcher-core`. Desktop workspace: `cimmeria-launcher-engine`, `cimmeria-launcher-relay`, `cimmeria-launcher-desktop`.

## Waves

| Wave | Packets | Ships as | Merge order |
|---|---|---|---|
| 0 Client login screen | CT-01, CT-02, CT-03, CT-05 (CT-04 lab spike, own docs PR) | one PR `client-tls-oidc/wave0` | CT-01, CT-03, CT-02, CT-05 |
| 1 [Server foundations](packets-wave1.md) | CT-10..CT-16 | one PR `client-tls-oidc/wave1` | CT-10, CT-11, CT-14, CT-12, CT-13, CT-15, CT-16 |
| 2 [Server routes](packets-wave2.md) | CT-20..CT-26 | one PR `client-tls-oidc/wave2` | CT-20, CT-21, CT-22, CT-23, CT-24, CT-25, CT-26 |
| 3 [Server wiring, relay, launcher engine](packets-wave3.md) | CT-30..CT-35 | one PR `client-tls-oidc/wave3` | CT-30, CT-31, CT-32, CT-33, CT-34, CT-35 |
| 4 [Launcher UI, GM, docs](packets-wave4.md) | CT-40..CT-46 | one PR `client-tls-oidc/wave4` | CT-44, CT-45, CT-40, CT-41, CT-42, CT-43, CT-46 |
| 5 [Ops, owner, gated](packets-wave5.md) | CT-50..CT-58 (CT-55 Cancelled), CT-U1..U6, CT-60 | one PR per packet | as each gate opens (table at the top of the wave file) |
| 6 [Revocation model, server half](packets-wave6.md) | CT-70..CT-76 | merges into `cto-camp`; PR #2031 | CT-70, CT-71, CT-72, CT-74, CT-73, CT-75, CT-76 |
| 7 [Ticket guards and handoffs](packets-wave7.md) | CT-77..CT-82 | merges into `cto-camp`; PR #2031 (opens on wave 6 merged) | CT-77, CT-80, CT-81, CT-82, CT-78, CT-79 |
| 8 [OIDC interop, relay CI](packets-wave8.md) | CT-83..CT-85 | merges into `cto-camp`; PR #2031 (parallel with 6) | CT-85, CT-83, CT-84 |
| 9 [Launcher sessions and controls](packets-wave9.md) | CT-86..CT-91 | merges into `cto-camp`; PR #2031 (parallel with 6 and 8) | CT-90, CT-86, CT-87, CT-88, CT-89, CT-91 |
| 10 [Review hardening](packets-wave10.md) | CT-95..CT-99 (the five external-review fixes) | merges into `cto-camp`; PR #2031 (CT-95, 97, 98 parallel with 6, 8 and 9) | CT-95, CT-97, CT-98, CT-99, CT-96 (CT-99 after CT-97; CT-96 after CT-73 and CT-82) |
| Tail ([wave 8 file](packets-wave8.md#tail), [wave 10](packets-wave10.md)) | CT-94, CT-101, CT-102, CT-92 (CT-93 and CT-100 Deferred) | merges into `cto-camp`; PR #2031 | after waves 6-10: CT-94, CT-101, CT-102, CT-92, then CT-100 when unblocked; then the campaign review loop and PR #2031 with `full-ci`. One `network-security-auth` pass over waves 6, 7 and 10 together |

The [consolidated PR #2031 ruling](packets-wave10.md) is the authority for waves 6-10 and the tail; where an older wave file disagrees, it wins.

## Wave 0: client login screen

Dispatch CT-01, CT-03 and CT-05 at once. CT-02 waits for CT-01 and CT-03 reviews to return `ship`; `ops` creates `cto-02` at the wave integration head after both merged in. CT-04 runs once the lab is free and the lab MCP is up (lab use approved in principle 2026-10-24); it does not block the wave PR.

### CT-01 Overlay patch 017: `Login.lua` and `Login.layout`

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-01-login-overlay` · **Worktree:** `cto-01`
- **Commits:** two, so the diff is reviewable like the Black Market overlay: `chore(client-patches): stock Login.lua and Login.layout for 017 (CT-01)`, then `feat(client-patches): 017 launcher sign-in login screen (CT-01)`.
- **Rules:** R1-R9. Contract: [C8](contract.md#c8-client-overlay-and-logininternallua-data-client-patches017-launcher-sign-in).

Files and changes:

1. Commit 1: copy the stock files byte for byte from the QA client next to the main checkout (`..\SGW\Stargate Worlds-QA\Working\SGWGame\Content\UI\Startup\Login\Login.lua` and `Login.layout`, relative to `<repo>`) to `data/client-patches/017-launcher-sign-in/Login.lua` and `Login.layout`. Record each file's size, date and SHA-256 for the README (`Get-FileHash`). Add `.gitattributes` lines after the 016 line: `data/client-patches/017-launcher-sign-in/*.lua -text`, `data/client-patches/017-launcher-sign-in/*.layout -text`, `data/client-patches/017-launcher-sign-in/test/*.lua -text`. Commit these three paths.
2. Commit 2, `Login.lua`: apply C8 "Overlay `Login.lua`" exactly. Keep stock names and order; add new functions next to the ones they extend; ASCII only; CRLF kept. The stock file is 192 lines; the result should stay under 330.
3. `Login.layout`: apply C8 "Overlay `Login.layout`". The two unnamed caption windows are the `StaticText` windows whose `Text` is `Account Name` (around l.26) and `Password` (around l.39). Place `Login_StatusText` over the account caption and edit box area (`{{0,0},{1,-160},{1,0},{1,-90}}`), and `Login_PasswordModeButton` below the login button inside the panel; widen the panel's lower bound by 30 px if the button would fall outside it.
4. `data/client-patches/017-launcher-sign-in/patch.json`, following `008-dialog-portraits/patch.json`:
   - `"id": "017-launcher-sign-in"`, `"title": "Login screen: sign in through the launcher"`.
   - `"description"`: "Lets the launcher sign you in: when you pressed Play after signing in, the login screen shows who you are signed in as and an Enter button instead of the name and password boxes, with a Use password button to type them instead. Without a launcher sign-in it behaves like the stock screen. Replaces Login.lua and Login.layout with whole files."
   - `"files"`: the two paths `Working/SGWGame/Content/UI/Startup/Login/Login.lua` and `.../Login.layout`, `from` the file names.
5. `data/client-patches/017-launcher-sign-in/README.md` (CRLF): base-file table (size, date, SHA-256 of the stock originals from step 1), the behaviour list from C8, the `CimmeriaLogin` version-1 field table, "published patches are append-only: a change after publishing needs a new id", and how to run the UAT (`L1`).
6. `data/client-patches/README.md` "The patches" table: one row after 016: `` | `017-launcher-sign-in` | The login screen shows a launcher sign-in and an Enter button; stock behaviour without one (see its [README](017-launcher-sign-in/README.md)) | 2 whole files | not built yet | ``. Nothing else in that file (it is 1019 lines, over the cap).
7. `crates/launcher-core/src/client_setup/stock_case.rs` `PATCH_TARGETS` (l.25-61): add `"Working/SGWGame/Content/UI/Startup/Login/Login.layout"` and `".../Login.lua"` in the list's order, with a `// 017-launcher-sign-in` comment.
8. `crates/launcher-core/src/client_changes.rs`: a title and description row for `"017-launcher-sign-in"` next to the 016 row, the same text as `patch.json`.

Tests:

| Test | Asserts | Fails on revert because |
|---|---|---|
| `every_patch_target_is_listed` (existing, `stock_case.rs`) | passes with 017 present | without step 7 the new `files` paths are unlisted |
| the client_changes per-id test that covers 016 (grep `016-hotbar` in the tests) | gains a 017 case asserting the title | without step 8 the id falls back to the generic text |

The Lua behaviour is tested by CT-02.

Checks, in order: S1 `cimmeria-launcher-core`, S2 `cimmeria-launcher-core`. Report the Login.lua and Login.layout line counts and SHA-256 of the stock originals.

### CT-02 Lua logic UAT for 017, and CI

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-02-login-uat` · **Worktree:** `cto-02` (at the wave head with CT-01 and CT-03 merged)
- **Commit:** `test(client-patches): 017 login screen logic UAT (CT-02)`
- **Rules:** R1, R3, R5, R6, R7. Contract: C8.

Files:

1. `data/client-patches/017-launcher-sign-in/test/stubs.lua`: CEGUI window stubs for every global `Login.lua` and `Login.layout` touch (read the overlay `Login.lua` and list them: the `Login_*` windows with `setText`, `getText`, `show`, `hide`, `isVisible`, `subscribe`; `LoginWin`; `Events`; `PromptMod.showPrompt`; `accountLogin`; the URL opener; the saved variables). Model it on `crates/client-patches/overlay/test/stubs.lua`. `accountLogin(server, user, pass)` records its arguments; `PromptMod.showPrompt` records the text.
2. `test/run.lua`: loads `stubs.lua`, a `LoginInternal.lua` variant, then the overlay `Login.lua`, calls `LoginMod.startup()`, and asserts. Cases:
   - **ticket, manual:** the golden `LoginInternal.ticket.lua` (CT-03). Boxes, labels, remember check and combo hidden; status text `Signed in as Player (Player_1234)`; button `Enter`; press calls `accountLogin("http://127.0.0.1:50123","Player_1234","AbCd...")` once and sets status `Connecting...` before the call; `GSavedVar_AccountName` unchanged.
   - **ticket, auto:** the golden file with `auto = false` replaced by `true` in memory: `accountLogin` called once by `startup()`; a second `startup()` call does not call it again.
   - **failure:** `onLoginFailure(nil, nil, "x", 4)` sets the status to `CimmeriaLogin.errors[4]` and the button to `Retry`; errorNum 13 gives the generic text containing `13`.
   - **use password:** the mode button shows the stock widgets, clears both boxes, and a later press uses the boxes' text.
   - **stock:** a plain `LoginInternal.lua` (the `login_servers::render` output for one server, inlined in the test): no `CimmeriaLogin`; widgets visible; status hidden; the stock login path runs; website/account/forums use the stock URLs.
   - **links:** `CimmeriaLogin.links.website` set, others nil: website opens it; account and forums buttons hidden.
   - **old screen fallback:** when env `SGW_UI_DIR` is set, load the stock `Login.lua` from it with the golden file: both boxes hold the account and ticket after `startup()`. Skip with a printed note when unset.
   Exit non-zero on the first failure, printing the case name.
3. `test/run_lupa.py`: copy `016-hotbar-learn-and-feedback/test/run_lupa.py`, changing the docstring's id.
4. `.github/workflows/test.yml` job `overlay-lua` (l.408-433): add a step after the 016 step: `- name: 017 launcher sign-in login screen` / `run: lua5.1 data/client-patches/017-launcher-sign-in/test/run.lua`.

Revert proof (in the report): run L1, then temporarily delete the `cimmeriaAutoTried` guard in a scratch copy loaded through an env override, or revert step 2 of CT-01 in the working tree without committing, and show the run fails; restore and rerun green.

Checks: L1.

### CT-03 `LoginInternal.lua` ticket rendering (launcher-core)

- **Host:** windows · **Agent:** packet-coder · **Branch:** `client-tls-oidc/CT-03-login-session` · **Worktree:** `cto-03`
- **Commit:** `feat(launcher-core): render a ticket LoginInternal.lua (CT-03)`
- **Rules:** R1-R8. Contract: C8.

Files:

1. New `crates/launcher-core/src/client_setup/login_session.rs`:

```rust
pub const TICKET_AUTO_SUBMIT: bool = false;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Links { pub website: Option<String>, pub account: Option<String>, pub forums: Option<String> }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TicketLogin {
    pub server_name: String, pub relay_url: String, pub account: String,
    pub ticket: String, pub display: String, pub links: Links,
    pub errors: Vec<(u32, String)>,
}
pub fn default_errors() -> Vec<(u32, String)>;          // the 4 and 5 texts of C8
pub fn sanitize(s: &str, max: usize) -> Option<String>;  // C8 rules; None when empty or refused
pub fn render_ticket(t: &TicketLogin) -> Result<String, String>;
pub fn write_ticket(install_dir: &Path, t: &TicketLogin) -> std::io::Result<()>;
```

   - `render_ticket` emits exactly the C8 text (CRLF), in that line order. `server_name` and `relay_url` must pass `login_servers`'s field rules and `relay_url` must start with `http://127.0.0.1:`; `account` must match `^[A-Za-z0-9_-]{3,20}$`; `ticket` must be 32 ASCII alphanumerics; `display` falls back to `account` when `sanitize(display, 32)` is `None`; a link that fails `sanitize(.., 160)` or lacks `https://` is dropped; an error text that fails `sanitize(.., 160)` is dropped. Any other failure is `Err` naming the field (never its value).
   - `write_ticket` writes to `login_servers::path(install_dir)` through a temp file in the same folder and a rename.
   - The header comment lines are the C8 ones.
2. `client_setup/mod.rs`: `pub mod login_session;`.
3. `data/client-patches/017-launcher-sign-in/test/LoginInternal.ticket.lua`: the C8 golden text with `CimmeriaLogin.links = {}` and no link lines, CRLF, no trailing text after the final `end\r\n`. (CT-01 adds its `-text` line; if CT-01 has not merged when you start, add the same `.gitattributes` line and say so.)

Tests (`login_session.rs` `mod tests`):

| Test | Asserts | Fails on revert because |
|---|---|---|
| `renders_the_golden_ticket_file` | `render_ticket(&fixture())` equals `include_str!` of the golden file byte for byte | the golden file is the Lua UAT's input; a drift breaks the overlay contract |
| `refuses_quotes_backslashes_and_bad_tickets` | `Err` for a ticket of 31 chars, a ticket with `"`, an account `a"b`, a relay URL `http://example.org:1` | the field rules are the only guard against a Lua injection |
| `display_falls_back_to_the_account` | display `"\u{1F600}"` renders `CimmeriaLogin.display = "Player_1234"` | sanitizing is new |
| `drops_unsafe_links_and_errors` | an `http://` link and an error text with `\` are absent from the output | the same |
| `auto_submit_stays_off_until_v2` | `assert!(!TICKET_AUTO_SUBMIT)` with a comment pointing at CT-57 | guards an early flip |
| `write_ticket_replaces_the_file` | in a canonicalized tempdir, `login_servers::write` then `write_ticket` leaves the ticket text | new |

Checks: S1 `cimmeria-launcher-core`, S2 `cimmeria-launcher-core`.

### CT-04 Lab spike V1-V3 (lab-gated)

- **Host:** windows · **Agent:** the orchestrator runs the `lab-uat` skill with a Haiku `lab-driver`; a `network-security-auth` agent reads the server log · **Branch:** `client-tls-oidc/CT-04-spike` · **Worktree:** `cto-04`
- **Gate:** lab use approved in principle (2026-10-24); before starting, confirm the lab is free and the lab MCP is up, else report `blocked - lab busy` or `blocked - lab MCP down`.
- **Output:** `docs/analysis/client-tls-oidc/worknotes/spike-v1-v3.md` (CRLF), own docs PR.

Steps, recorded with evidence (server log lines, `login_audit` rows, screenshots):
1. **V1:** with a hand-written `LoginInternal.lua` whose server is `http://127.0.0.1:<port>`, a throwaway HTTP listener records the request line: is it `POST /SGWLogin/UserAuth`? Repeat with `http://127.0.0.1:<port>/prefix`: does the prefix survive (F15)?
2. **V2:** with the CT-03 golden ticket file (real server, a ticket minted by hand is not available yet, so use the account's SHA-1 password as the "ticket" field and the stock `Login.lua`): do both boxes fill? Then with the CT-01 overlay files copied over a scratch install: the ticket-mode screen, Enter, Use password, and `auto = true`: does the auto submit log in and reach character select without a hang?
3. **V3:** at character select press Change Server: record which password hash the second `UserAuth` carries (server log at debug, `login_audit` rows).
4. Record every `errorNum` the client passes to `onLoginFailure` for codes 2, 4, 5, 13 and 17 (force each from a dev server).
5. **V5** (optional, needs the Mac): same as V1 under Wine.

Pass: V1 yes; V2 boxes fill and the overlay screen works; V3 re-sends the same hash. A "no" on any becomes a ruling for the architect (re-slice CT-35 or CT-57).

### CT-05 Doc fixes: F13 and the encryption ADR amendment

- **Host:** windows · **Agent:** documentation-writer · **Branch:** `client-tls-oidc/CT-05-docs` · **Worktree:** `cto-05`
- **Commit:** `docs: SHA-1 password label and the launcher relay in the encryption ADR (CT-05)`
- **Rules:** R1, R3, R5, R7, R9.

1. `docs/drafts/spec/mercury-wire-format.md`: R16 and Figure 39 call `ServerConnection+0x130` an "internal dispatch-table hash". It is `SHA-1("test")`, the stored password hash (design F13). Correct the label and caption; add one sentence citing the design.
2. The encryption ADR (find it: `Get-ChildItem docs -Recurse -Filter *.md | Select-String -Pattern "Phase 2" -List` under `docs/architecture/` or `docs/adr/`): add an "Amendment 2026-10-24" section: for launcher users, Phases 1 and 2's client patches are replaced by the launcher relay (TLS from the launcher to the server) and the per-launch ticket (D-CT1); direct clients keep plain HTTP and the legacy SHA-1 path (D-CT7); argon2id(SHA1HEX) at rest needs no client patch (D-CT10, CT-54). Link the ledger.
3. Do not touch `docs/client/sgw-launcher.md`: it is the retired egui launcher's archived page ([doc-update-map.md](../../agents/doc-update-map.md)). The desktop launcher docs change in CT-46.
