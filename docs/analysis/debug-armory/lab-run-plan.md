# debug-armory: lab run plan for DR-L2 to DR-L10

> **Type:** runbook. **Audience:** the orchestrator running waves 4 and 5, the lab-driver and verifier agents it briefs. **Written:** 2026-11-02 by the architect, against the `dr-lab` worktree at `a6108ad48`.
> **Companions:** [README.md](README.md), [work-packets.md](work-packets.md) (wave 4 and 5 table), [results/DR-L1.md](results/DR-L1.md), [shared UAT contract § C1-C8](../starter-uat-praxis/work-packets.md#shared-uat-contract), [automated-uat.md](../../guides/automated-uat.md).

This note decides what runs, in what order, and how. It changes no spec and no code. Where it differs from the contract, it says so (rulings R1-R9).

## What is being run

22 spec files, 570 rows, every one `account = "gm"`, `character = "fresh"`: each file creates its own fresh character on the lab account (Celltest, the lab character, is protected from slot clearing and is not used). Instance `default` only. Server: the local server on DB `sgw_dr_lab`, as DR-L1.

| Packet | Spec file (section id = file name) | Rows | Spec-blocked | Weapon rows | Fresh character | Est. lab min |
|---|---|---|---|---|---|---|
| DR-L2 | `armory-abilities-soldier-0` / `-1` / `-2` | 23 / 26 / 26 | 0 | 15 / 18 / 0 | SGU Soldier, SGC_W1 | 55-70 |
| DR-L3 | `armory-abilities-commando-0` / `-1` / `-2` | 24 / 23 / 20 | 1246, 1250, 1355 (in `-1`) | 0 / 4 / 11 | SGU Commando | 50-65 |
| DR-L4 | `armory-abilities-scientist` | 60 | 997 | 13 | SGU Scientist | 35-50 |
| DR-L5 | `armory-abilities-archeologist-0` / `-1` / `-2` | 22 / 21 / 25 | 0 | 0 / 6 / 0 | SGU Archeologist | 50-65 |
| DR-L6 | `armory-abilities-asgard-0` / `-1` / `-2` | 25 / 22 / 22 | 0 | 0 | SGU Asgard (holding profile, OD-CS09) | 55-70 |
| DR-L7 | `armory-abilities-goauld-0` / `-1` / `-2` | 22 / 22 / 21 | 0 | 9 / 10 / 7 | Praxis Goa'uld, Castle_CellBlock (holding profile, OD-CS08) | 50-65 |
| DR-L8 | `armory-abilities-sholva` | 52 | 0 | 14 | SGU Sholva | 31-44 |
| DR-L9 | `armory-gear-weapons-1` / `-2` / `-3` | 30 / 30 / 13 | 0 | 16 unbound rows (9 / 2 / 5) | SGU Soldier | 55-70 |
| DR-L10 | `armory-gear-armor-1` / `-2` | 30 / 11 | 0 | 0 | SGU Soldier | 30-40 |

Row counts are the `[[row]]` tables in each file; every ability file's first row is its setup row `<Archetype>-00` (travel to `.gotolocation DebugArea 246 6.6 -878`, `/gmgiveallabilities`, one hostile `@dummy`). Only 1 of the 107 weapon ability rows has `required_ammo > 0` (997, spec-blocked), so empty guns (character-creation.md:145, OD-CS13) do not touch the ability rows.

Time model: about 5 min per file (fresh character, first-login hold, intro, the `-00` world load, session end) plus 30-45 s per row. Total about **7 to 9 hours of lab time**, over six sessions.

## Rulings (architect)

- **R1. One driver dispatch per spec file.** A split file is its own plan, execute and `run_dir` (contract § C8.3); never two files in one run (the gear headers forbid it: one file's grants fit the 40-slot bag).
- **R2. Batches of 6 rows per execute call**, all on one `run_dir`, with `end_session: false` on every batch but the last, so the character stays in the world between calls (each call would otherwise log out and disconnect, automated-uat.md § End of the run). The `rows` order is the plan's order. A resumed call adopts the fresh character from `run.json` (automated-uat.md:236).
- **R3. Canary first.** Session 1 starts with 3 short runs (§ Canary) before any full file. No full file runs until the canary is graded.
- **R4. The driver passes no `lease_id` and no `server_version`.** The run takes and releases its own lease; the verifier attests the version. The driver never calls `lab_client_restart` (it has none); `lab_uat_run` itself starts and logs in the client when it is down.
- **R5. The verifier may call `lab_uat_report`, and for attests only `lab_lease_status`, `lab_lease_acquire`, `lab_uat_attest` and `lab_lease_release`**; never a driving tool. `lab_uat_attest` is lease-guarded and `lab_uat_run` releases its own lease, so the verifier works like this:
  1. It grades from the bundle and writes its results while the next file drives.
  2. It lists every pending attest (row, clause, value, query) in `results/<packet>.md` § Attests.
  3. Only when `lab_lease_status` shows no holder, it calls `lab_lease_acquire {owner: "dr-verify-<packet>", purpose: "attest"}`, runs every attest with that `lease_id`, and calls `lab_lease_release` at once.
  4. If the lease is held, it does not wait. It returns `attests-pending`, and the orchestrator re-dispatches only the attest step in the next gap: after a driver returns and before the next `lab_uat_run`, or at the session end.

  The orchestrator never passes its own lease, and never dispatches a driver while a verifier attest is in flight. This amends contract § C3's "no lab tools" for spec runs, as automated-uat.md § Attest requires. (Ruling: [worknotes/ruling-weapon-select.md](worknotes/ruling-weapon-select.md).)
- **R6. Gear specs run where the fresh character arrives** (SGC_W1 201.5 1.31 49.724): they have no travel row, and `@dummy` spawns beside the player. If the canary's gear rows fail on placement or targeting, DR-L9 waits for **DR-07b** (§ Follow-up packets). Armor rows do not depend on place.
- **R7. A bound gear row whose server log shows `use_ability_no_ammo`** is graded **PARTIAL** (the press left the client and the binding resolved; the gun is empty by design), as DR-L1 L1-5. No issue; the count goes to DR-99 and to DR-07c.
- **R8. Order.** Soldier first (most weapon rows, the canary's spec). Asgard runs late in wave 5: its creation may fail (F42 / HELD-B16), which is cheap to grade. DR-L9 runs last so DR-07b can land first.
- **R9. Server evidence comes from SigNoz.** `server_log_tail` held nothing in DR-L1; the local server exports with resource attribute `cimmeria.lab_run = dr-lab`.

## Sessions and run order

Each session is one ASK-FIRST question (contract § C1): "May I take the lab now for session N (packets ..., about M minutes, local server sgw_dr_lab)?" A yes covers that session only (see D-DR8 for a wider yes).

| Session | Wave | Runs, in order | Lab time | Driver dispatches | Driver calls |
|---|---|---|---|---|---|
| 1 | 4 | Canary (3 runs), then DR-L2 soldier-0, -1, -2 | 85-100 min | 3 + 3 | 9 + 20 |
| 2 | 4 | DR-L3 commando-0, -1, -2; DR-L4 scientist | 85-115 min | 4 | 24 |
| 3 | 4 | DR-L5 archeologist-0, -1, -2. Then the wave-4 results PR | 50-65 min | 3 | 19 |
| 4 | 5 | DR-L7 goauld-0, -1, -2; DR-L8 sholva | 80-110 min | 4 | 23 |
| 5 | 5 | DR-L6 asgard-0 (then -1, -2 only if `-00` PASSed); DR-L10 armor-1, -2 | 85-110 min | 5 | 25 |
| 6 | 5 | DR-L9 weapons-1, -2, -3 (after DR-07b if R6 triggered). Then the wave-5 results PR | 55-70 min | 3 | 19 |

Driver calls per file = 1 status + 1 plan + ceil(rows / 6) executes + 1 report: about 150 driver calls in all. Verifiers add about 200 (SigNoz queries and attests; DR-L10 alone about 50). Total about **350 tool calls** and **7.5 to 9.5 hours** including the canary.

Verifiers grade while the next file drives; their attests wait for an idle-lab gap (R5). One verifier per packet, after its last file; it writes `results/<packet>.md`.

## Before each session (orchestrator)

1. Check no peer coordinator or other lease holder runs the lab, and no starter-uat-shops SS-D1 run is open (dispatch rule 6).
2. Ask (§ C1). In the same question, ask the user to confirm the screensaver (`Ribbons.scr`) is off, the build lane is idle, and the local server is up on `sgw_dr_lab` (the orchestrator has no shell).
3. Never edit a spec in the `dr-lab` worktree during a session: `run.json` records each spec's SHA-256, and labd reads `C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dr-lab\docs\guides\uat-specs`.

## Canary (session 1, before DR-L2)

Three runs, each its own driver dispatch with the template below, `BATCH` 6, and an explicit `ROWS` list instead of the whole file:

| Run | Section | ROWS |
|---|---|---|
| C1 | `armory-abilities-soldier-0` | `Soldier-00`, `Soldier-598`, `Soldier-717` and the next 3 row ids in file order |
| C2 | `armory-gear-weapons-1` | `GW-3128` (bound), `GW-17` (unbound), `GW-5680` (bound) |
| C3 | `armory-gear-armor-1` | `GA-33`, `GA-2933` |

A canary verifier (same verifier brief, packet `DR-L2-canary`) checks, and the orchestrator continues only if all hold:

- each run created its fresh character and `run.json` names it (`fresh_characters`);
- `Soldier-00` is PASS (world 1300 reached; the runner's own `client_wait_for` world wait works, #1677 is closed);
- each row JSON has an `anchor` with a bookmark id (DR-L1 never saw a `.bug` reply in chat; a missing anchor is a flag, record it, not a stop);
- every ability row shows `client.ability.sent`, and every ability and bound-weapon row has a SigNoz `abilities` `ability_launched` row (or, on GW-3128, a `wrong_weapon_refused` until the item gets its moniker, DR-99) for the fresh character; gear weapon rows never show `client.ability.sent` (the right-click is an `interact`);
- the `GW-17` SigNoz clause attests to 1 row (`weapon_unbound`, reason `no_ranged_attack`, the spec's filter as written) and `GA-33`'s `moved` clause finds `Inventory move persisted`.

Any miss: stop the session, write `worknotes/canary.md` (what failed, run dirs) and ask the architect for a ruling. The canary rows are graded again from the full runs; the canary results only go in `results/DR-L2.md § Canary`.

## Lab-driver prompt template

The orchestrator fills the `<...>` fields and pastes the whole block as the driver's prompt (the driver cannot read files). One dispatch per spec file.

```text
PACKET: <DR-Lx>   FILE: <section id>   ATTEMPT: <n>
You run one committed UAT spec with lab_uat_run and report raw results. Everything you need is here; do not look for files.

SECTION: "<section id>"
SPECS_DIR: "C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\dr-lab\docs\guides\uat-specs"
RUN_DIR: <none | the run_dir to resume>
ROWS: <all | an explicit list>
SKIP_ROWS: [<rows already in RUN_DIR; rows with a spec blocked reason: Commando-1246, Commando-1250, Commando-1355, Scientist-997>]
SETUP_ROW: "<Archetype>-00" (none for gear files)
BATCH: 6

Rules:
- Tools you may call: lab_lease_status, lab_uat_run, lab_uat_report. Nothing else.
- Never pass lease_id or server_version. Pass SPECS_DIR exactly as written on every lab_uat_run call.
- Never retry a failed lab_uat_run call. Never improvise.

Steps:
S1. lab_lease_status. If anyone holds the lease, report the holder and stop.
S2. Plan: lab_uat_run {sections: [SECTION], specs_dir: SPECS_DIR, plan_only: true} (add rows: ROWS when ROWS is a list).
    Record every row id and its result in the order listed. READY = the SKIPPED rows in that order, minus SKIP_ROWS.
    Record each BLOCKED row's reason word for word; do not run it. If READY is empty, go to S5.
S3. Split READY into batches of BATCH rows, in order. For each batch call
    lab_uat_run {sections: [SECTION], rows: [<batch>], specs_dir: SPECS_DIR, end_session: false}
    adding run_dir: <RUN_DIR, or the run_dir the first execute call returned> on every call after the first
    (and on the first one too when RUN_DIR is given). On the last batch leave end_session out.
    After each call record: the run_dir, and for each row its result and first reason word for word.
S4. Stop (go to S5) when any holds after a call:
    a. SETUP_ROW was in this batch and its result is not PASS.
    b. The call returned an error or timed out.
    c. Every row of this batch and of the batch before it is FAIL or BLOCKED.
    d. A result says the lease is held by someone else, or the client could not start.
    e. Your context passes 80k tokens.
S5. If you have a run_dir: lab_uat_report {run_dir: <run_dir>}. Record its counts per result.

Report (raw values only, no diagnosis):
PACKET <id> FILE <section> ATTEMPT <n>
lease: <free | holder>
plan: ready <n>, blocked <n>; blocked rows: <id>: <reason>
calls: | # | rows | run_dir | ok or error text |
rows: | row | result | first reason |
report: PASS <n> FAIL <n> BLOCKED <n> NEEDS_HUMAN <n> UNVERIFIED <n> NATIVE_SHORTFALL <n>
stopped_by: <a-e or none>; not run: <row ids>
```

Resume: when a driver stopped on S4b (a timeout) the run may still be going on the daemon. Wait 10 minutes, then dispatch attempt n+1 with `RUN_DIR` set and `SKIP_ROWS` extended by every row the last `lab_uat_report` lists. Three attempts per file, then one Sonnet takeover (dispatch rule 5).

## Verifier steps

Agent `general-purpose`, one per packet (and one for the canary). Paste: the driver reports, every `run_dir`, the section ids, and these steps. Rules: grade from the run bundle and SigNoz, never from the driver's summary; call only the R5 lab tools (`lab_uat_report`; for attests `lab_lease_status`, `lab_lease_acquire`, `lab_uat_attest`, `lab_lease_release`); write only `docs/analysis/debug-armory/results/` in the `dr-lab` worktree; CRLF; no fixes.

1. **Bundle.** For each run dir (under `C:\Users\Steve\AppData\Local\cimmeria-lab\uat-runs\`): read `run.json` (fresh character name and archetype, start and end UTC, spec SHA) and `ledger.md`; read a row's JSON only when its result is not PASS or it has a PENDING clause.
2. **Build.** SigNoz logs, resource `cimmeria.lab_run = 'dr-lab'`, the run window: read `service.version`; `lab_uat_attest` it for the run.
3. **PENDING SigNoz clauses** (gear: 16 `unbound`, 41 `moved`). Run the clause's own filter (with `${character}` already filled in) AND `cimmeria.lab_run = 'dr-lab'`, over the clause's window. Attest `{run_dir, row, clause, row_count, rows, query_ran}` with the exact query. A "key not found": discover the key with `signoz_get_field_keys`, retry once with the existing key, and record the change in `## Recipe`; never drop a condition silently. If the SigNoz MCP is down, use ClickHouse over ssh (`reference_signoz_clickhouse_via_ssh`).
4. **Server cross-check** (ability and weapon rows): one query per run: scope `abilities`, `player_name` = the fresh character, events `ability_launched`, `ability_refused`, `cast_refused`, `use_ability_no_ammo`, `weapon_unbound`, `weapon_draw_queued`, over the run window. Match each row by `ability_id` and the row's `started_utc` to `ended_utc` plus 10 s.
5. **Grade** (contract § C5):
   - runner PASS and `ability_launched` for the row's ability: **PASS**;
   - runner PASS, no `ability_launched`, a refusal row: **PARTIAL**, cite the event and reason (R7 for `use_ability_no_ammo` on gear rows);
   - runner PASS and no server row at all: **PARTIAL** ("client only");
   - runner FAIL: **FAIL**, with the first reason and the server row (or "no server row"); then classify it as server defect, spec defect or lab defect;
   - spec-blocked rows (1246, 1250, 1355, 997): **BLOCKED** citing D-DR3;
   - `Asgard-00` FAIL or BLOCKED on `character_create_failed`: every Asgard row **BLOCKED** citing F42 / HELD-B16;
   - gear unbound rows: **PASS** when the `weapon_unbound` attest holds (a measured gap, F8);
   - armor rows: "PASS (server clause); human clause NEEDS_HUMAN" when `moved` holds, with the screenshot path `rows/<section>/<row>/final.png`;
   - UNVERIFIED left after attesting: **UNVERIFIED**, with the reason.
6. `lab_uat_report {run_dir, ledger: true}` per run; copy the counts.
7. **Write** `results/<packet>.md` (contract § C6 with the § C8 `## Runs` table first: `| Call | Spec or split file | plan_only | run_dir | Outcome |`), then the grade table, `## Server-side defects`, `## NEEDS_HUMAN`, `## Not run`, `## Resume hint`, `## Recipe`, and the driver reports under `## Driver reports`. A packet over 500 lines splits its rows into `results/<packet>-<file>.md`.

## Failures and issues

Per contract § C3 rule 7. The verifier writes one issue body per **distinct defect**, not per row: rows that fail with the same server event and reason (or the same runner reason) share one body.

- Path `results/issues/<packet>-<slug>.md`, CRLF. Body: title; `Rows:` spec section and row ids; `Runs:` run dirs; `Expected:` the spec text; `Observed:` the server evidence with the exact query and time; `Repro:` the `lab_uat_run` arguments (absolute `specs_dir`, `sections`, `rows`); `Area:` the file if known, else "unknown"; `Labels:` per [docs/agents/](../../agents/).
- Server or client-behaviour defects get an issue body. **Spec defects** (wrong weapon name, a bad macro, a placement miss) get no issue: the orchestrator asks the architect for a re-spec ruling (`worknotes/DR-Lx.md`). **Lab defects** (a tool error, a runner bug) get an issue body labelled lab.
- The four spec-blocked abilities share **one** body, `results/issues/DR-weapon-audit-fail.md` (D-DR3): the DR-L3 verifier writes it with 1246, 1250 and 1355; the DR-L4 verifier adds 997.
- DR-30 files the bodies (`gh issue create --body-file`) and records the numbers back.

## Stop conditions (orchestrator)

- The canary misses any check (§ Canary).
- A file ends with more than half its driven rows FAIL, BLOCKED or UNVERIFIED with the same first reason: no next file until the architect rules (a systemic cause wastes the rest of the session).
- The client crashed twice in one file, or a driver hit S4d: end the session and tell the user.
- A session runs 30 minutes past the time asked: stop at the next file boundary and ask again.
- Asgard-00 fails creation: skip `asgard-1` and `-2` (grade BLOCKED, R8).

## Follow-up packets (only when triggered)

- **DR-07b** (if R6 triggers): packet-coder, Host: windows. `debug_armory.py specs gear` emits a first row `GW-00` / `GA-00` per file with the abilities files' travel setup (`.gotolocation DebugArea 246 6.6 -878`, the world-1300 wait, the `@player_state` `world_id = 1300` clause); regenerate the five gear files (LF); unit test; check `pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-lab committed_specs_parse_and_validate every_committed_spec_tag_is_seeded`. Merge into `dr-integration` and the `dr-lab` worktree before session 6.
- **DR-07c** (candidate, recorded by DR-99, not run in this campaign): load the gun before the right-click in the bound gear rows, once the reload path a spec can drive is known. Only if R7 PARTIALs are many.

## Shipping

Verifiers write in the `dr-lab` worktree; `ops` commits each packet's results (`docs(debug-armory): DR-Lx results`). Wave 4 ships as **one docs PR** from `dr-integration` after session 3: merge order DR-L2, DR-L3, DR-L4, DR-L5, then the README status row and RESUME. Wave 5 ships as one docs PR after session 6: DR-L7, DR-L8, DR-L6, DR-L10, DR-L9 (DR-07b, if any, merges as its own PR first). Then wave 6 (DR-30, DR-99).

## Owner decisions

| ID | Question | Default until answered |
|---|---|---|
| D-DR8 (new, BlockedDecision, user) | May one yes cover a whole wave (wave 4 about 4 hours, wave 5 about 5 hours), driven unattended with the lab unavailable to anyone else? | Ask per session (six asks) |
| D-DR9 (new, BlockedDecision, user or lomiada) | Who answers the 41 armor human clauses and any other NEEDS_HUMAN: a person with `lab_uat_attest` (verdict, answer, by) from the screenshots, or leave them NEEDS_HUMAN in DR-99? | Leave NEEDS_HUMAN; list the screenshot paths |
| D-DR7 / D-SP2 (open) | This plan runs on the local server, as DR-L1 did. A colo sweep after a release stays open. | Local only |
| D-DR5 (open) | `/gmremoveitem <itemId> <quantity>` now works (docs/commands.md:274), so the `.removeitem` half of D-DR5 may be moot. | No change in this plan |
