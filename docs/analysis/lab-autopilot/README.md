# lab-autopilot: honest, puppet-grade lab tools

> **Type:** Explanation and Reference (campaign ledger)
> **Audience:** The orchestrator, the owner, packet-reviewers
> **Last updated:** 2026-10-31
> **Companions:** [work packets](work-packets.md), [contract](contract.md), [resume card](RESUME.md), waves [1](packets-wave1.md), [2](packets-wave2.md), [3](packets-wave3.md), [4](packets-wave4.md), [5](packets-wave5.md), [6](packets-wave6.md), [user directives](user-directives.md), [stack inventory](inventory-lab.md), [UE3 back-port check](ue3-backport-verified.md)

## Purpose

The live research lab (`crates/lab`, its daemon, and the lab-only bridge in `crates/client-telemetry` behind feature `lab-bridge`) drives the real Stargate Worlds client for UATs and probes. Today most of its tools report what they *sent*, not what *happened*: a click says the cursor was over the target, a walk says it released W near the goal, and a Haiku driver retells that as success. Round 1 of the Haiku colo UAT (#1660) showed the cost: 12 of 32 rows were NATIVE_SHORTFALL and about 7 needed a Sonnet escalation to find out what had really happened.

This campaign lifts what is worth lifting from ramenvr/autopilot (an Unreal 5 agent harness) and back-ports it to our UE3-era client (engine build about 3004), under the [user directives](user-directives.md):

1. **No lies.** Every action tool returns `evidence` with a verdict of `confirmed`, `denied` or `inconclusive`, built from post-reads taken at least two client frames after the action, and server corroboration where the server can say. A claim the evidence does not support is downgraded by a gate before any agent sees it.
2. **Puppet control.** Key holds with a lease, a per-frame watch, same-frame batch reads, a line trace, a coast-aware walker, a ledge guard, stop reports, recovery recipes and detectors.
3. **Lab only.** New client code lives only in the `lab-bridge` feature, never in a release build, the signed manifest or the launcher path, and every DLL packet proves it (C13).

Design only so far: no code is written and no lab run is part of the design. Two packets need the lab (AX-02, AX-64), and each waits for the user's OK; the lab binaries are rebuilt from `main` before wave work and before each of them (O1, [§ Ops chores](#ops-chores)).

## Findings: autopilot concept to SGW

Class: **RE** needs static reverse engineering first; **spike** needs a live lab run (ask first); **pure** is code with stubbed tests (the `Sim`, `fake_bridge`, host-side DLL tests), no lab.

| Autopilot concept | SGW back-port | Client evidence ([ue3-backport-verified](ue3-backport-verified.md)) | Packets | Class |
|---|---|---|---|---|
| Action results carry evidence and a verdict; a gate refuses unsupported claims | `crate::proof`: `Evidence`, `Check`, why codes, `judge`, the gate in `call_tool` and the UAT invoker, strict switch | n/a (daemon) | AX-10, AX-11, AX-12, AX-44 | pure |
| Post-conditions are read after the effect, not in the same frame | Bridge `meta {frame, mono_ms, game_t}` on every response; `wait_fresh` | Row 4 VERIFIED (Tick hook drains the bridge) | AX-20, AX-22 | pure |
| Ask the authority, not only the client | Server corroboration for `move_to` and non-placeholder `minigame_play` (D-AX11) | n/a | AX-13, AX-42 | pure |
| Structured stop reports with what was tried | `StopReport` (C8): stuck, timeout, ignored input, stalled, blocked UI, ledge | n/a | AX-14, AX-32, AX-34, AX-56 | pure |
| Resume a long run where it stopped | `lab_uat_run resume_from` (C10) | n/a | AX-16 | pure |
| Held inputs with a lease that releases on loss | DLL `input_hold` with a 10 s lease, released on disconnect; daemon `HoldBook`; raw `client_input_key` presses go through it; one release path that also runs when a lease ends | Row 15 PARTLY: held state persists between posts (live); a raw key down stayed held after a daemon drop until `client_input_release` (seen 2026-10-31) | AX-21, AX-23, AX-26, AX-27 | pure |
| Per-frame watch of chosen values | DLL `watch_*` ring on the Tick; daemon `client_watch` and `WatchStore` | Rows 3, 18, 21: pawn pose and view point VERIFIED live (L4); game clock from `tickSync` | AX-24, AX-50 | pure |
| Read many values in one frame | DLL `batch` (read-only allowlist); `client_batch` uses it for literal reads | Row 4 | AX-25, AX-52 | pure |
| Click, camera and UI actions judged by read-back | `expect_visible`/`expect_hidden`, target and range checks, camera view checks, item and drag evidence, typing read-back | Rows 7, 8, 13, 16 PARTLY; mouse-look 521.5 counts/rad (L5) | AX-31, AX-33, AX-40, AX-41, AX-43, AX-46 | pure |
| Discover clickable widgets | `client_ui_clickables`: one Lua walk with full CEGUI names, `__auto_` children included | Row 7: Lua walk works; 21416 windows live (L8), so it pages | AX-45 | pure |
| Recovery recipes and a quick-fail cooldown | `lab_recover` (C11), cooldown (C9) | n/a | AX-35, AX-36 | pure |
| Anomaly detectors over the run | `lab_detectors_read`: hitch, teleport, long fall, out of world, NaN, stuck input, actor growth, repeated errors | Row 21; actor count needs AX-01 (static) | AX-51 | pure |
| Frames named on every capture | Screenshots bracketed by heartbeat ticks; capture stays PrintWindow (D-AX2) | Row 17 PARTLY | AX-52 | pure |
| Movement that models latency and coasting | Lag `Sim`; learned coast lead; arrival judged where the pawn settled | Coast and latency numbers come from AX-02 (defaults until then) | AX-53, AX-54 (priors from AX-02) | pure; spike for priors |
| Sonar: line traces for walls and ledges | DLL `trace_line` through `UWorld::SingleLineCheck` `0x00875970`; `client_trace_line`; ledge guard in `move_to` | Row 5 VERIFIED live (L1, L2: `ret 0x1c`, flags `0x2086`/`0x20bf`; `0x3fff` faults); only interiors traced, so the ledge guard self-checks; terrain is AX-64 U4 | AX-55, AX-56 | pure |
| A blind describer of frames | `frame-describer` agent: image paths and neutral questions only | n/a | AX-60, AX-61 | pure (no code) |
| Honest end-to-end check | A live UAT with lie-bait cases | all of the above | AX-02, AX-64 | spike |

Not lifted, and why:

- **Reflection calls (`ProcessEvent` by name).** The lookup works live (`FindFunctionChecked` `0x0049ffb0`), but a hook or a general by-name call is a large new capability; D-AX5 keeps it out (an FName allowlist if it is ever added).
- **Input below the OS (`InputKey` injection).** The `InputKey` slot is not located and posted messages work live; D-AX12.
- **Direct widget clicks (`fireEvent`).** Would count as N3 at best; D-AX7.
- **Backbuffer capture through `Present`.** The `CreateDevice` hook can miss the device; PrintWindow works; D-AX2.
- **Openings and ways-up fans, property reads by name, combat-text OCR** (rows 6, 2, 9): follow-ups once `trace_line` is live; not in this campaign.

## #1660 and what guards it now

Every #1660 defect was fixed by the lab-harness campaign (LH, PR #1663, [ledger](../lab-harness/README.md)). This campaign re-fixes none of them; it makes each one come back as `denied` or `inconclusive` instead of a false pass.

| #1660 item | Fixed by | Guarded by |
|---|---|---|
| Split drags never start | LH-01 | AX-41 (`drag_that_never_started_is_ignored`) |
| Main-bag drags fail on another tab | LH-02 | AX-41 |
| `client_item_action` reports a pass but sends nothing | LH-03 | AX-41 (`item_use_that_changed_nothing_denies`) |
| `__auto_` children not addressable | LH-04 | AX-45 (`client_ui_clickables`), AX-40 |
| `client_input_mouse` ignores x/y | LH-05 | AX-43 |
| Camera pitch sticks at the clamp | LH | AX-33 (`pinned_pitch_notes_the_clamp`) |
| Right-click out of range opens nothing | LH-08 | AX-31 (`far_right_click_is_denied_out_of_range`) |
| `client_move_to` stuck in the hub | LH | AX-32, AX-34 (stop reports), AX-54, AX-56 |
| `client_chat_log` misses a reply | LH | AX-46 (typed text read back); the log read itself stays a NO_CLAIM read |
| Start-up hang under build load | LH | AX-51 (`hitch` in heartbeat mode) reports it; the watchdog is unchanged |
| `client_player_state` empty stats, `server_log_tail` flood | LH | not guarded: reads, NO_CLAIM |

## What "confirmed" does and does not mean

Be exact about this in every report:

- **Server-corroborated:** `client_move_to` (the server's position of the player) and `client_minigame_play` for minigames that are not placeholders (D-AX11). A `confirmed` there means the server agrees.
- **Client-read only:** every other `confirmed` rests on a read of the client's own state taken at least two frames after the action (C4). The client is the system under test, so a client bug can fool its own read-back. These verdicts are much better than "posted", but they are not server truth.
- **Pixels:** a screenshot is evidence only through the blind `frame-describer` (AX-60), never the coordinator's own reading.
- **Unjudged:** until AX-44 sets the strict switch, an unjudged tool's `effect_ok` stays as it was (D-AX13). After AX-44, unjudged counts as inconclusive.
- **Grading:** an `inconclusive` action makes its UAT row UNVERIFIED unless a required clause passed on its own evidence (D-AX1).

## Client-side safety

Every client-side addition is lab-only and proves the C13 checklist in its packet report. In short, for each:

| Addition | Packet | Idle | Kill switch | Fingerprint gate | Absent from release |
|---|---|---|---|---|---|
| Response `meta` and the Tick hub | AX-20 | at most two atomic loads per Tick; a test asserts an idle Tick posts nothing and reads no game memory | `CIMMERIA_CLIENT_HOOKS_DISABLE=all` removes the Tick hook, so nothing runs | `game_t` only when the gate passed | all code in `bridge/`, compiled only with `lab-bridge`; `build-desktop.sh` builds default features and refuses `LAB_BRIDGE_MARKER` |
| Key holds with a lease | AX-21 | no holds, no posts | as above | posts only the `WM_KEYDOWN`/`WM_KEYUP` the lab already sends | as above; every hold released on disconnect and by `input_release` |
| Watch sampler | AX-24 | stopped watch reads nothing | as above | reads through the existing guarded readers; on a mismatch a sample is `{i, f, ms}` | as above; stops on disconnect |
| Same-frame batch | AX-25 | runs only when called | as above | read-only allowlist of methods that exist singly | as above |
| Line trace | AX-55 | runs only when called | as above | refused when the gate failed, `GWorld` is 0 or not the `UWorld` vtable, or any earlier call faulted (the latch: an AV can leave the engine's `GMem` mark unrestored, so there is never a second call); flags limited to the two proven live | as above; the one exception to "no new writes": it calls `SingleLineCheck`, which reads collision and writes only the caller's stack buffer |

The bridge needs both the compiled feature and the session file's `lab` block (`maybe_start`). No packet adds a hook. Release builds, the signed manifest and the launcher update path are untouched by every packet (directive 4).

## Decisions

| ID | Decision | Status | Reason |
|---|---|---|---|
| D-AX1 | An `inconclusive` action makes its UAT row UNVERIFIED unless a required clause passed on its own evidence | Architect ruling | A row cannot pass on an action nobody saw work; it also should not FAIL on missing evidence |
| D-AX2 | Frame capture stays supervisor-side (PrintWindow bracketed by heartbeats); an in-DLL `Present` hook needs its own ADR | **BlockedDecision (owner)**; default as written, blocks nothing | A `Present` hook is a new, deeper hook on the render path; the `CreateDevice` hook can miss the device (row 17) |
| D-AX3 | `frame-describer` model | **BlockedDecision (owner)**; default `haiku` | Cost vs. reading accuracy on small UI text; one line to change |
| D-AX4 | The user's OK for each live run (AX-02, AX-64) | **BlockedDecision (user, per run)** | Directive 7 |
| D-AX5 | No `ProcessEvent` hook; a by-name call, if ever added, only through an FName allowlist | Architect ruling | Large capability, little need: everything this campaign reads has a direct reader |
| D-AX6 | `out_of_world` fires below z -100000 UE3 units | Architect ruling, provisional | No measured world floor yet; revisit after AX-64 |
| D-AX7 | Direct widget clicks stay N3 and are not built | Architect ruling | They skip the input path a player uses, which is what a UAT tests |
| D-AX8 | Hold lease 10 s (clamped 200 ms..10 s), renewed at half | Architect ruling | Long enough for a slow daemon tick, short enough that a dead daemon frees keys quickly |
| D-AX9 | The C13 checklist binds every DLL packet | Architect ruling | Directives 3-4 made checkable |
| D-AX10 | Which new tools the `lab-driver` may call (`lab_detectors_read`, `client_ui_clickables`, `lab_recover`, `client_watch`; un-deny `lab_crash_report`, `lab_timeline`) | **BlockedDecision (owner)**; AX-61 step 2 only | Widens what a cheap model may do on the live client |
| D-AX11 | Server corroboration only for `move_to` and non-placeholder `minigame_play` | Architect ruling | The only actions with a server-side record cheap to query today; others would need new server endpoints |
| D-AX12 | No `InputKey` injection | Architect ruling | Slot not located; posted input works live |
| D-AX13 | Unjudged results stay legacy until AX-44's strict switch | Architect ruling | Lets waves 1-4 land without breaking current UAT rows |
| D-AX14 | `client_use_ability` judges whether the press got an answer, not whether the ability succeeded | Architect ruling | DA-U22 expects a refusal; a refusal is a truthful answer |
| D-AX15 | `already_true` (state held before the action) is `inconclusive` | Architect ruling | Reading back a state that was already there proves nothing |
| D-AX16 | Raw `client_input_key` presses become DLL holds; the daemon-drop release is the DLL's `on_disconnect`, not a daemon first-connect release | Architect ruling (2026-10-31) | The bridge already drops its connection on any error and the DLL releases on that drop; a first-connect release could clobber another live session's keys. Residual gaps: an old DLL (fallback) and posted mouse buttons on a daemon crash |
| D-AX17 | `trace_line` accepts only flags `0x2086` and `0x20bf` and latches off after its first fault | Architect ruling (2026-10-31) | The only values proven live; `0x3fff` access-violated, and an AV inside `MultiLineCheck` can leave `GMem` unbalanced |
| D-AX18 | `lab_login` fails, naming the rows, when the login-screen combo has two or more servers and none matches the account file; it never takes the preselected row | Architect ruling (2026-10-31) | The preselect is the first entry of a Lua hash table, so it can be a dead server; a guessed row is a lie by omission |

Owner decisions open: **D-AX2, D-AX3, D-AX10** (each has a default that blocks nothing before wave 6) and **D-AX4** (per live run).

## Packets

Statuses: Ready / BlockedDependency / BlockedDecision / Writing / Review / Integrated / UATPending / Done. "Ready" in waves 1-6 means ready when its wave opens ([work-packets § Waves](work-packets.md#waves)).

| Wave | Packet | Title | Agent | Status |
|---|---|---|---|---|
| 0 | AX-01 | Static RE: one anchors record, the actor count, the clock owner | game-archaeology-specialist | Ready |
| 0 | AX-02 | Live spike: coast, latency, frame bracketing, hover, clock; optional player death and DI mouse-look | lab-uat coordinator + lab-driver | BlockedDecision D-AX4 (after O1) |
| 0 | AX-03 | `lab_login` selects the login-screen server from the account file | packet-coder | Ready |
| 0 | AX-04 | `lab_client_start` names or clears a running screensaver | packet-coder | Ready |
| 1 | AX-10 | The proof module | packet-coder | Ready |
| 1 | AX-15 | Mechanical splits (server input tools, runner rows) | rust-gameserver-dev | Ready |
| 1 | AX-11 | The evidence gate | packet-coder | Ready |
| 1 | AX-12 | The runner reads verdicts | rust-gameserver-dev | Ready |
| 1 | AX-13 | Server corroboration, Sim split | packet-coder | Ready |
| 1 | AX-14 | Stop reports | packet-coder | Ready |
| 1 | AX-16 | `resume_from` | rust-gameserver-dev | Ready |
| 2 | AX-20 | Frame stamps and the Tick hub (DLL) | rust-gameserver-dev | Ready |
| 2 | AX-21 | Key holds with a lease (DLL) | rust-gameserver-dev | Ready |
| 2 | AX-24 | Watch sampler (DLL) | rust-gameserver-dev | Ready |
| 2 | AX-25 | Same-frame batch (DLL) | rust-gameserver-dev | Ready |
| 2 | AX-22 | Daemon reads frames, checks freshness | packet-coder | Ready |
| 2 | AX-23 | World tools hold keys through the DLL | packet-coder | Ready |
| 2 | AX-26 | Raw keys through DLL holds, one release path | packet-coder | Ready |
| 3 | AX-27 | Release every key and button when a lease ends | packet-coder | Ready |
| 3 | AX-30 | Splits: click result, move probe, unstick | rust-gameserver-dev | Ready |
| 3 | AX-31 | Click and target evidence | packet-coder | Ready |
| 3 | AX-32 | `move_to` evidence and stop reports | rust-gameserver-dev | Ready |
| 3 | AX-33 | Camera evidence | packet-coder | Ready |
| 3 | AX-34 | Ignored input and blocked UI | packet-coder | Ready |
| 3 | AX-35 | `lab_recover` | rust-gameserver-dev | Ready |
| 3 | AX-36 | Quick-fail cooldown | packet-coder | Ready |
| 4 | AX-45 | `client_ui_clickables` | packet-coder | Ready |
| 4 | AX-40 | Clicks that say what they expect | packet-coder | Ready |
| 4 | AX-46 | Typing and sequences that read back | packet-coder | Ready |
| 4 | AX-41 | Item, drag, vendor and map evidence | packet-coder | Ready |
| 4 | AX-42 | Combat evidence | packet-coder | Ready |
| 4 | AX-43 | Input primitives say what they know | packet-coder | Ready |
| 4 | AX-44 | Flow evidence and the strict switch | rust-gameserver-dev | Ready |
| 5 | AX-50 | `client_watch` and the watch store | packet-coder | Ready |
| 5 | AX-51 | Detectors and `lab_detectors_read` | packet-coder | Ready |
| 5 | AX-52 | Frame-bracketed screenshots, DLL batch path | packet-coder | Ready |
| 5 | AX-53 | Lag and coast in the `Sim` | packet-coder | Ready |
| 5 | AX-54 | The walker stops before it coasts past | packet-coder | Ready |
| 5 | AX-55 | DLL `trace_line` | rust-gameserver-dev | Ready (after the wave 2 PR) |
| 5 | AX-56 | `client_trace_line` and the ledge guard | packet-coder | BlockedDependency AX-55, AX-54 |
| 6 | AX-60 | `frame-describer` agent | packet-coder | Ready (model per D-AX3, default haiku) |
| 6 | AX-61 | lab-driver and lab-uat read verdicts | packet-coder | Ready (step 2 BlockedDecision D-AX10) |
| 6 | AX-62 | Guide: evidence, stops, watch, detectors | documentation-writer | Ready |
| 6 | AX-63 | ADR addendum, client-telemetry docs | documentation-writer | Ready |
| 6 | AX-64 | Live UAT: try to make the tools lie | lab-uat coordinator | BlockedDecision D-AX4 |
| 6 | AX-65 | Close-out | documentation-writer | Ready (after AX-64) |

Live lab needed: **AX-02** (spike) and **AX-64** (UAT), each after O1 and the user's OK. Static RE: **AX-01**, which gates only the watch's `actors` field and the `actor_growth` detector. Everything else is pure code or docs with stubbed tests. The live reads L1-L8 of 2026-10-31 answered the trace, view-point, loading, mouse-look, `GWorld` and window-registry questions; the full open list, live or static, is [work-packets.md § Live versus static](work-packets.md#live-versus-static-what-is-still-open). Still open after them:

- **Static (AX-01):** the actor count at `[[GWorld+0x50]+0x3C/+0x40]`; the owner of `tickSync`'s `+0x32C` and the `WorldInfo.TimeSeconds` offset (nice to have). Not in this campaign: the `InputKey` slot, the SCT modifier strings and the CEGUI `fireEvent` binding (ue3-backport-verified, open static questions), since D-AX7 and D-AX12 keep those features out.
- **Live, ask first (AX-02):** coast and input latency, screenshot frame bracketing, hover id `PC+0x4C0`, `game_t` against wall time; optionally the player's dead bit `+0x158` (seen only on an NPC) and DI-only mouse-look.
- **Live, ask first (AX-64):** terrain and outdoor traces with `0x2086` (U4), the key fail-safe (U11), the login combo (U12), the screensaver path (U13).

Lab failures folded in on 2026-10-31: `lab_login` could log into the combo's preselected (dead) server (AX-03, D-AX18); `lab_client_start` failed on a running screensaver it did not name, because the check enumerated windows on the daemon's desktop only (AX-04); a raw key down outlived the daemon (AX-26, AX-27, D-AX16).

## Ops chores

Machine chores, not packets; each needs the user's OK and runs in a `hands-on` session ([work-packets.md § Ops chores](work-packets.md#ops-chores-not-packets)):

- **O1 Lab rebuild and re-verify.** The installed lab daemon, CLI and DLL are older than `main`. Rebuild them from `main` (`tools/lab/install.ps1`, then `tools/lab/daemon.ps1 restart`) before wave 1 dispatches, and again before AX-02 and before AX-64. The live re-verify is the first step of the next approved live run.
- **O2 Test-database cleanup.** About 690 leftover per-worktree `sgw_*` databases make Postgres crash recovery take about 15 minutes. Drop every database no live worktree maps to, after showing the user the dry-run list.

## Attribution

The concepts in this campaign (evidence and verdicts, stop reports, held-input leases, the watch, the detectors, the blind describer, sonar traces) are adapted from ramenvr/autopilot, which is Apache 2.0. No code, text or test is copied from it; every packet is written from our own contract. A packet that ever copies code from it must add the Apache 2.0 notice and attribution with that code, and AX-63 checks the tree for it.

## Log

- 2026-10-31: designed. Research: [inventory](inventory-lab.md), [UE3 back-port check](ue3-backport-verified.md) (static passes 1-2 and the live pass L1-L8, run with the user's approval). 42 packets in 7 waves.
- 2026-10-31 (revision): reconciled with the live pass L1-L8. AX-01 shrank to the actor count, the clock owner and labels; AX-02 lost its trace and TimeSeconds steps and gained optional player-death and DI mouse-look steps; AX-55 no longer waits on AX-01 (wave 5b dropped); C14 now refuses on the `UWorld` vtable instead of the loading flag, allows only the two proven flags and latches off after a fault; C16 pages (21 416 windows). Added AX-03 (login combo), AX-04 (screensaver), AX-26 and AX-27 (key fail-safe), D-AX16..18, and the ops chores O1 and O2. 46 packets in 7 waves.

## Close-out

Filled by AX-65.
