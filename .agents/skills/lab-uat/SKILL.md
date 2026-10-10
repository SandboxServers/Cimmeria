---
name: lab-uat
description: Drive the live SGW game client through the Live Research Lab (cimmeria-lab and lab-server MCP tools) for a UAT step, a lab probe or a reproduction, and grade the result against server evidence. Use when asked to "test it in the lab", "run the UAT", "check it in game", "drive the client", "lab probe", "lab_uat_run", or to confirm a server change against the real client. Covers asking the user first, the lab lease, getting in the world in one call, briefing a cheap lab-driver agent with a gotcha sheet, verifying the driver's claims against the server log and packet tap, and attesting rows.
---
<!-- Generated from .claude/skills/lab-uat/ by tools/agent-skills/sync.py. Edit the source, then rerun the script. -->

# Lab UAT

The canonical guide is [docs/guides/live-research-lab.md](../../../docs/guides/live-research-lab.md).
Automated rows: [docs/guides/automated-uat.md](../../../docs/guides/automated-uat.md).
Manual rows: [docs/guides/unified-uat.md](../../../docs/guides/unified-uat.md).
This skill is the order of operations, plus the mistakes that keep costing runs.

## 0. Ask before using the lab

The lab is shared with other sessions and with the user's own desktop. Lab input hooks also swallow the user's own mouse clicks.

- **Ask the user, and wait for a yes,** before any `lab_lease_acquire`, client start or restart, or change to the server the lab points at.
- If `lab_lease_status` shows another holder, wait. Never `force` a takeover unless the holder is gone and the user agrees.
- Worker and coder briefs say "do not drive the lab".

## 1. Prepare before touching the client

- **Trace the real flow first.** Read the seed chains, dialogs and spawn coordinates in `db/resources/`, so the brief names real NPCs, real positions and the real order. Every guessed step costs a round trip.
- **Pick targets that have an interaction at the current mission state.** "No window" is often "no interaction". Confirm it with `server_entity_get` or the server log.
- **Testing a branch, not the release:** run a local server and point the lab at it, as in `.claude/agent-memory/main-session/reference_lab_local_server.md`. A DB reload deletes the lab character; recreate it.

## 2. Lease and get in the world: two calls

1. `lab_lease_acquire {owner, purpose}` returns a `lease_id`. Pass it to every driving tool. Reads (screenshots, UI readers, status) need no lease.
2. `lab_ensure_in_world {character?, create?}` handles every state for you: it starts the client if needed, logs in, logs out of the wrong character, plays, finishes the intro dialog and sets virtual focus. Don't chain `lab_client_status`, `lab_login` and `lab_play_character` by hand.

Release the lease with `lab_lease_release` when you are done. An idle lease expires after `ttl_s`. With no lease held, the watchdog leaves a dead client down.

## 3. Fewest calls, smallest results

Most of a driver's cost is fixed context resent on every turn, so cut **turns**, not bytes:

- Batch reads with `client_batch` (`lua`, `mem_read`, `player_state`, `window_text`, `wait` steps).
- Run a scripted UI step with `client_ui_sequence` (`click`, `key`, `type`, `drag`, `wait_window`).
- Run whole graded rows with `lab_uat_run`.
- Skip `lab_client_status` polling: `lab_ensure_in_world` already observes the client.
- Use `fields: [...]` to trim a result. Ask for `verbose: true` only when you need it.
- Screenshots save to a file and return the path; open the file only when you need to look. **Never pass `image: true`**: an inline capture costs over 1k tokens on every later turn.
- Take one screenshot **before** an important click, and one after a failed click to find the obstruction. Don't screenshot every step.

## 4. Brief a lab-driver agent

Game driving runs in a fresh agent on a cheap model (Haiku in Claude Code): a `lab-driver` agent where your harness defines one, otherwise a general-purpose agent given the lab MCP tools. Check its tool list covers the gotcha sheet (entity lookup, Lua, typing) before you rely on it. Keep each run under ~100k tokens: one probe per agent, and start a new agent rather than resuming one. Give it an exact step list and what to report; interpret the results yourself. Paste this gotcha sheet into every brief:

```text
Lease: <lease_id>. Standing autonomy: Lua reads, GM teleports and following the game's
own mission text are fine; stop only on a real block, and report raw results compactly.
Gotchas:
- Turn virtual focus on before input (lab_ensure_in_world does this).
- Teleport with `.gotolocation <World> x y z`. Never `.gotoxyz`: it moves your SELECTED target.
- Chat/GM: click Inst1Chat_Input, type, read it back (Lua: return Inst1Chat_Input:getText())
  before Enter. Close every dialog before a chat command or GM jump; a jump under an open
  dialog orphans it (close with DialogMod.onDialogDoneClicked(DialogWin, DialogWin)).
- Interact: find the entity id (entity find, or client_entity_table for unrendered or
  unnamed ones). Stand 3-4 m away (server range 5.0 m) with open floor behind you, never
  against a wall. Face it, correct pitch to about -15 deg, then click BY entity_id, not by point.
  A hover answering other_entity 2 is your own avatar: reposition, don't force.
- Escape does not close Flash windows: read the close button's rect, move there, click.
- Character names need 3+ letters; lab_create_character can't make single-name races.
- Failed click: screenshot and look at the PNG before retrying.
Steps: 1. ... 2. ...   Report: <exact fields>.
```

The full interaction procedure (camera gains, click retries, windows) is in the live-research-lab guide's *Driving the client* and *Fewer calls* sections. Keys: Q/E rotate, B bag, Tab target.

## 5. Verify, don't trust

A driver's summary is a claim. Before you grade a step, check it against server evidence:

- `server_log_tail` (keeps only 500 lines), searching for `interact`, `too far`, `fire_interact_tag` and WARNs.
- `server_packet_tap_start` / `_read` / `_stop` to see what was actually sent.
- `server_entity_get` / `server_entity_query`, `server_ability_state`, and read-only `server_db_query`.
- `lab_timeline`, which merges client events and the packet tap on one clock.
- SigNoz, for anything older than the log tail (see the `telemetry-triage` skill).

If no window opened, read the server log before you retry the click.

## 6. Grade and record

- `lab_uat_run`: run `plan_only: true` first. Then run batches of about 5 rows on one `run_dir`, and pass `server_version`.
- SigNoz clauses come back PENDING with their filter. Run each filter, `lab_uat_attest` the row, then `lab_uat_report { ledger: true }`.
- Manual rows: record the result and evidence in the campaign ledger and the unified UAT guide row.
- A FAIL becomes a fix packet in the campaign's ledger.
- Release the lease.

## Known lab tool traps

- **Driver says a tool is missing:** after a labd restart, a session keeps its old tool list until `/mcp` reconnects `cimmeria-lab`.
- **`lab_login` types the password into the account box:** set both edits with Lua (`Login_AccountEdit:setText`, `Login_PasswordEdit:setText`) and click `Login_LoginButton`.
- **`bad_token`:** a stale telemetry grant cache causes it; move `binaries/sessions/lab-telemetry-grant.json` aside.
- **~1 min freeze at character select:** swapping a `Cache.en-US` `.pak` forces a full resync.
