# Lab parallel clients

> Type: ledger. Audience: the coordinator, packet workers and reviewers.
> Opened 2026-10-10 against `main`. Prefix `LP-`. Tracking issue #1312.
> Packet specs: [work-packets.md](work-packets.md).
>
> **Campaign status (2026-10-10): closed. LP-07 passed on the colo.** The fix is proven by hand
> (five clients at once, 2026-10-10); the packets turn it into lab code.

## Why

Two `SGW.exe` clients on one workstation froze one another, which blocked
two-player UAT rows and any parallel lab testing. The maintainer's goal: up
to five lab clients at once (one per seeded lab account, `lab` and `lab2` to
`lab5`), all driven through **one** shared lab daemon, with **one lease per
lab account** instead of one lease for the whole lab.

## What the 2026-10-10 runs proved

Full evidence: #1312 (issue body and comment), and
[multi-client-lab.md](../../reverse-engineering/findings/multi-client-lab.md)
once LP-04 updates it.

1. **Root cause: the cooked-data cache lock.** A running client holds all 22
   `Documents\My Games\Firesky\SGWGame\Cache.en-US\*.pak` open with
   `GENERIC_READ|GENERIC_WRITE` and read-only sharing for its whole life. A
   second client cannot open one (`Error opening static cache archive` for
   each, then `Error copying source archive`).
2. **The freeze is the resulting full resync.** The second client reports
   cooked version 0 for all 21 categories; the server pushes about 59,000
   entries (75 to 80 s on loopback, TextStrings alone 29,126 entries in 37 s)
   at every login. The second client's main thread is pinned (about 20 s of
   CPU in a 21 s `Responding = False` window); world entry takes 25 to 30 s
   instead of 6 s. Its writes fail, so every login starts from version 0
   again. The first client stays responsive.
3. **The fix: a per-instance `USERPROFILE`.** `SGW.exe` resolves My Documents
   in exactly one place, `SHGetFolderPathW(CSIDL_PERSONAL)` (`0x004c6333`;
   its only other folder lookup is `CSIDL_LOCAL_APPDATA` at `0x004935ad`).
   Windows expands the registry value `%USERPROFILE%\Documents` with the
   calling process's own `USERPROFILE`. A client launched with `USERPROFILE`
   pointed at a seeded per-instance profile opens its own 22 archives
   (checked with `NtQueryInformationProcess(ProcessHandleInformation)`).
4. **Scale.** Four lab clients (`lab` to `lab4`, each with its own profile)
   logged in within about 7 s each and entered the world in 6 to 10 s, all at
   once, with 20 categories `up_to_date` and one routine one-category resend
   each, and zero cache errors; all four then travelled to the Debug Area
   together. A fifth client (bare, `lab5` profile) booted to the login screen
   alongside them. The lab's own cap (`CEILING_MAX_CLIENTS = 4`) refused a
   fifth lab client.

Found on the way, fixed by this campaign:

- **One daemon, one lease.** The daemon hosts one supervisor; the lease book
  is process-wide ([`crate::lease::global`]). The maintainer chose one daemon
  hosting every account with a lease per account (2026-10-10) over one daemon
  per account.
- **The watchdog kills a slow boot.** With other clients running, a new
  client's bridge took about 25 s to come up; the default watchdog killed it
  at about 17 s as "heartbeat unreachable" (main thread idle), three times,
  then hit its recovery cap.
- **The client cap.** `CEILING_MAX_CLIENTS = 4`, `DEFAULT_MAX_CLIENTS = 2`.

Not fixed here (follow-ups):

- Cameras cannot be turned by the lab (`client_camera` mouse-look and the
  wheel do not reach the game; #1243). PR #1309 adds native camera control.
- A Documents folder redirected to an absolute path (OneDrive Known Folder
  Move) defeats the `USERPROFILE` redirect; LP-01 warns. The durable fix is a
  `SHGetFolderPathW` hook in the lab DLL (#1312, proposal 3).
- The cooked-data resync cost every player pays after a deploy (pacing, a
  progress display): a separate issue.

## Design

| Piece | Decision |
|---|---|
| User folder | Every lab instance, the default one included, launches the game with `USERPROFILE = Binaries\sessions\instances\<label>\profile`, seeded once from the real `SGWGame` folder (`Config`, `Content`, `Cache.en-US`, top-level files; never account folders, `Logs`, `CrashDumps`, `Stats`). `CIMMERIA_LAB_SHARED_USER_DIR=1` restores the old shared folder. Module `crates/lab/src/supervisor/instance_profile.rs` (written with the plan). |
| Instances in one daemon | `CIMMERIA_LAB_INSTANCES=default,p2,p3,p4,p5` (unset: the one instance `CIMMERIA_LAB_INSTANCE` names, as today). Instance `i` (0-based) gets bridge port `CIMMERIA_LAB_BRIDGE_PORT + i` (8770, 8771, ...). Accounts: `lab-account.json` for `default`, `lab-account.<label>.json` otherwise (unchanged). |
| Routing | `call_tool` picks the target instance: the `instance` argument (a label such as `p3`, or the account name such as `lab3`), else the instance whose book holds the call's `lease_id`, else the first instance. It runs the tool on a clone of `LabServer` whose `supervisor` is the target's. No tool body changes. |
| Leases | One `LeaseBook` per supervisor (per instance, so per account). `lab_lease_acquire` acquires on the routed instance; `lab_lease_status` with no `instance` reports every instance. A lease id is only valid on its own instance. |
| Cap | Default cap = the number of hosted instances, at least `DEFAULT_MAX_CLIENTS` (2); `CEILING_MAX_CLIENTS = 5` (the five seeded lab accounts). |
| Watchdog | A boot grace: until the bridge has answered once, failed heartbeats within 90 s of launch do not count. |
| Daemon lock | Kept. One daemon per logon session is right now that it hosts every instance. |

## Dispatch rules

- **Workers** are `packet-coder` agents (Haiku), one packet each, in their own
  worktree (`pwsh -NoProfile -File tools/build-lane/mk-worktree.ps1 <branch> <name>`).
  A worker implements
  exactly its packet, runs the packet's checks through the build lane, and
  commits locally. It does not push, open a PR or touch another packet's
  files.
- **Review:** each finished packet gets a read-only adversarial reviewer on
  Sonnet (`packet-reviewer`). Findings go to the coordinator, who fixes, then
  ships with `python tools/build-lane/ship.py pr -C <worktree> -m <msg>`.
- **Order:** wave 1 = LP-01, LP-02, LP-03, LP-06 (parallel). Wave 2 = LP-05a
  (needs LP-02). Wave 3 = LP-05b (needs LP-05a). Then LP-04 (docs) and LP-07
  (live UAT). LP-01 and LP-02 both edit `supervisor/mod.rs` (different
  hunks): merge LP-01 first and rebase LP-02.
- **Open PRs on the same files:** #1309 (`server/mod.rs`, `lease/policy.rs`,
  `supervisor/mod.rs`, `server/world.rs`, `server/ui/mod.rs`) and #1302. The
  coordinator rebases whichever lands second.

## Packets

| ID | Packet | Depends on | Status |
|---|---|---|---|
| LP-01 | Per-instance user folder and client cap | none | merged, #1321 (9 review fixes) |
| LP-02 | One lease book per supervisor | none | merged (3 review fixes; p2 is not relaunched mid-UAT, accepted) |
| LP-03 | Lab tooling: `instances.ps1`, `labd.env` and `.mcp.json.example` | none | merged, #1319 (4 review fixes) |
| LP-04 | Docs and memory | LP-01, LP-05b | merged |
| LP-05a | Instance registry and routing in one daemon | LP-02 | merged (review: per-instance UI memory, two-player refused until LP-05b, routing tests, instance log spans) |
| LP-05b1 | Account-name routing and per-instance lease tools (split from LP-05b) | LP-05a | merged (2 review rounds, fixed by the Haiku coder) |
| LP-05b2 | UAT p2 from the registry (rest of LP-05b) | LP-05b1 | merged (2 review rounds, fixed by the Haiku coder; guard verified to fail on revert) |
| LP-06 | Watchdog boot grace | none | merged, #1320 (test-gap fix) |
| LP-08 | Instance profiles outside the game install (found by LP-07: SGW.exe refuses a user folder inside its install) | LP-01 | merged (1 review round, fixed by the Haiku coder) |
| LP-07 | Live UAT: five clients, one daemon, five leases | all | passed 2026-10-10 (second run, after LP-08) |

## LP-07 result (2026-10-10, colo)

One daemon (`CIMMERIA_LAB_INSTANCES=default,p2,p3,p4,p5`, build bceb723) and five Haiku lab-driver agents, one per account, started at the same moment.

- **First run: failed.** Every client quit at boot with `Failed to create user directory ... Force quitting`. SGW.exe refuses a `USERPROFILE` inside its own install folder, which is where LP-01 had put the profiles. Direct launches proved it: an empty profile inside the install fails, and outside it any length works (95 to 158 characters), with or without spaces. LP-08 (#1329) fixed it.
- **Second run: passed.**
  - **In the world.** All five clients reached the world at once: Labone to Labfive, ready in 26 to 38 s from launch.
  - **Leases.** `lab_lease_status` listed five holders, one per instance, each with its own client pid.
  - **Routing.** A lease acquired on `lab2` was refused on `p3` with `(instance p3)`.
  - **No full resync.** Colo telemetry (`client.cooked.versions_held`) shows each of the five sessions holding 21 of 22 categories at real versions (`covernodes_local` is always 0), so no full resync happened.
  - **No cache errors.** None of the five profiles' fresh logs has a cache-archive error.
  - **Watchdog and seeds.** No watchdog kill. All five profiles were already seeded (`seeded="already_there"`).
- **Follow-ups.**
  - Two of the five clients hit `bridge error -32603: dispatch timeout` at `enter_world` on the first try, then succeeded on retry. `lab_ensure_in_world` could retry that step itself.
  - A session's tool list only shows `instance` after `/mcp`. Agents that follow the schema literally then drop the argument, though the server accepts it. Tell drivers to pass it, or reconnect.
  - A Windows service can't host the supervisor (Session 0 has no desktop for SGW.exe). The daemon stays a logon scheduled task; a tray or `lab` CLI is an option.
