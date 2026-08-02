---
title: "Project Status Validation Plan"
type: how-to
audience: testers re-verifying project status with the live game client
last_updated: 2026-08-02
---

# Project Status Validation Plan

The playbook for the human re-verification campaign that repopulates [project-status.md](project-status.md), whose statuses were cleared on 2026-08-02. Every one of the 443 tracked features gets its verdict from a **human at the live game client** (or, for the handful of server-only systems, from the explicitly noted alternative in Phase 5) — not from unit tests, code reading, or memory of past sessions.

The plan is organised into six phases ordered by dependency: nothing in Phase 1 can run until Phase 0's login-through-world-entry smoke passes, and the two-client Phase 2 builds on a working Phase 1 core loop.

## Ground Rules

### What counts as verification

- A human drives the retail SGW client against a server built from the `main` commit under test, and **observes the behavior directly** (screen, chat log, second client's screen).
- Server-side evidence (SigNoz traces, server logs, DB rows) *corroborates* a client observation; it never substitutes for one, except in Phase 5 where the plan says so per system.
- One session verifies the features it exercised — not the whole system. A system's roll-up status is the *worst* verdict among its features (a system with one KM feature and nine CW features rolls up as IM at best, per the gap-analysis convention).

### Verdict rules

| Observed at the client | Record |
|------------------------|--------|
| Behaves correctly end-to-end, matches expected SGW behavior | CW |
| Works but with a defect, wrong values, or missing sub-behavior | IM — and file/link a GitHub issue for the defect |
| Feature does nothing / errors / server logs `UNIMPLEMENTED` | KM — even if code exists, if the client can't observe it working |
| Could not be exercised (blocked by an earlier failure, missing content, missing second tester) | NT — note the blocker so the session can be rescheduled |
| Not client-observable (server-internal) | NU or the Phase 5 alternative verification noted per system |

### Evidence requirements

Every verdict recorded into [project-status.md](project-status.md) / [gap-analysis.md](gap-analysis.md) must cite:

1. **Date + server commit SHA** the session ran against.
2. **Tester** (who was at the client).
3. **Artifact** — at least one of: screenshot/video, client chat-log excerpt, SigNoz trace link, server-log excerpt, or DB query result. Store artifacts under the session log (template below).
4. For **failures**: the GitHub issue number filed.

### Recording flow

1. Run a session against a checklist below; fill out a session log (template at the end of this document).
2. Update the feature rows in [gap-analysis.md](gap-analysis.md) with the new verdicts and evidence pointers.
3. Recompute the roll-ups in [project-status.md](project-status.md) (per-system table + Overall Completion).
4. Commit the session log alongside the doc updates so history ties verdicts to evidence.

## Test Environment

Set up once; reuse across sessions.

- **Server**: build per [building.md](building.md) (Windows target — the server runs alongside the client; see CLAUDE.md build rules). Record the `main` SHA. Fresh DB from `db/database.sql` at the start of the campaign; do **not** reseed mid-phase unless a checklist says to, since progression state (XP, inventory) is itself under test.
- **Client**: installed via the launcher per [guides/getting-started.md](guides/getting-started.md), pointed at the test server ([multiplayer.md](multiplayer.md) — `BASE_EXTERNAL` for the two-machine phases).
- **Accounts**: one **normal player** account (default access level) and one **GM** account (elevated access level — the SGWGmPlayer class flip requires it). Developer mode allows duplicate logins if you must run two clients from one account, but prefer two accounts so access-level gating is actually exercised.
- **Second client** (Phase 2): a second machine on the LAN, or a second instance where the setup permits. Two humans is ideal; one human alternating between two windows is acceptable for everything except simultaneity tests (marked ⏱ in Phase 2).
- **Observability**: SigNoz up per [operations/signoz-deployment.md](operations/signoz-deployment.md) with Mercury packet logging on, so every session automatically captures corroborating evidence. Keep the server log tailing in a visible terminal.
- **Discord** (optional, for P0.6): a throwaway webhook per [config/discord.toml.example](../config/discord.toml.example).
- **Reference docs while testing**: [commands.md](commands.md) for the `/` command surface, [gameplay/](gameplay/) per-system docs for expected original-game behavior.

---

## Phase 0 — Infrastructure Smoke

Single client, one session. Everything later depends on this passing.

### P0.1 — Authentication & login

Covers: SOAP login, password validation, shard list, shard key exchange, session establishment, login audit, duplicate-login prevention, developer-mode bypass, TLS listener + cert hot-reload, telemetry token.

1. Launch the client via the launcher; log in with the normal account and a **correct** password → expect shard list, then character select.
2. Log in with a **wrong** password → expect a clean auth failure at the client (no hang, no crash).
3. Check the `login_audit` table records both attempts with distinct outcomes.
4. While logged in on client A, attempt a second login with the same account from a second client (developer mode **off**) → expect rejection at character select.
5. Repeat with developer mode **on** → expect the duplicate to be allowed.
6. If TLS is configured: confirm the auth leg negotiates TLS (launcher log / packet capture), then swap the cert on disk and confirm hot-reload picks it up without a server restart on the next login.

### P0.2 — Mercury protocol

Covers: v1 transport, encryption, reliable delivery, ACK behavior, fragmentation, keepalive. Mostly verified *implicitly* — a stable session is the test.

1. Stay in-world for 30+ minutes of active play across Phase 1 sessions with **zero** unexplained disconnects, rubber-banding storms, or stuck states → v1 transport CW.
2. In SigNoz, spot-check the Mercury packet log for retransmit storms or ack stalls during a busy combat scene.
3. **Mercury v2**: no shipping client speaks it. Record the v2 rows NU with the note "requires a patched client; out of campaign scope" unless a patched client is available.
4. Reconnection grace: pull the network cable / kill the client process, reconnect immediately → document what happens (expected today: session lost, relog required). Record under P0.5.

### P0.3 — Game data pipeline

Covers: resource load, PAK overrides, DB-backed content delivery.

1. Server boots with no resource-load errors in the log (22 categories).
2. Client world entry shows correct items/NPC names/dialog text (i.e. DB rows actually reached the client).
3. Confirm one known **PAK override** is live: the mission-override handshake fires (server log `InvalidKeys` exchange) and an overridden mission step displays its Cimmeria-authored text, not the canonical PAK text.
4. Hot reload: not implemented — confirm and record KM.

### P0.4 — Database persistence

Covers: character/world persistence, outbox durability.

1. During Phase 1, after gaining XP + items + mission progress, log out, **restart the server**, log back in → character state fully intact.
2. Kill the server **ungracefully** (process kill) mid-session after a persistent action (item pickup), restart, relog → the action survived (outbox drained on restart).

### P0.5 — Session management

1. Idle at the character-select screen and in-world past the inactivity timeout → confirm the server disconnects the session and logs it.
2. Reconnect-after-drop behavior from P0.2 step 4 → expected missing (no grace period); record KM for the grace-period feature with the observed behavior.

### P0.6 — Observability, metrics, and Discord

Covers: OTLP pipeline, Mercury packet instrumentation, dev-session telemetry, Discord notifications, metrics/telemetry system rows.

1. After the P0.1 login, find the corresponding auth + world-entry spans in SigNoz → pipeline CW if the trace is complete.
2. Confirm Mercury packet logs are queryable for the session's traffic.
3. If dev-session telemetry is configured: confirm the launcher uploaded a session artifact.
4. Discord: with a test webhook configured, trigger one event per enabled channel (e.g. a login event, a panic-hook test if a safe trigger exists) → embed arrives in the right channel.

---

## Phase 1 — Core Gameplay Loop (Castle Cellblock, Single Client)

The bulk of the campaign. Run in Castle Cellblock first (the best-content zone); Phase 4 repeats a subset elsewhere. Order within the phase roughly follows a natural play session.

### P1.1 — Character creation

1. Create a character exercising **every archetype** offered and at least two visual variations → each enters the world with the correct appearance and starting loadout ([gameplay/character-creation.md](gameplay/character-creation.md) for expected loadouts).
2. Attempt invalid creations (duplicate name, empty name) → clean client-side/server-side rejection.
3. Delete a character → gone from character select after relog; DB row handled correctly.
4. Create on the GM account → verify the GM enters as the GM entity class (the `/` GM commands in P1.23 working is the observable proof).

### P1.2 — World entry & spaces

1. Enter Castle Cellblock from character select → loading completes, avatar at a valid spawn point, HUD live, no error spam server-side.
2. Verify entities in the entry area are all visible and named correctly (NPCs, objects, doors).
3. Relog into the same character mid-zone → re-enter at the logout position.

### P1.3 — Movement & navigation

1. Walk/run through the zone: slopes, stairs, doorways, tight geometry → no snap-backs or stuck states under normal play.
2. Deliberately hug walls and jump against collision → no fall-through, no server-forced repositions during legitimate movement.
3. Observe an NPC pathing around obstacles (goes around, not through) → Detour pathfinding observably working.
4. Movement *validation* (the anti-cheat side) is tested in P2.9, where a second client observes the correction.

### P1.4 — Combat & abilities

Reference: [gameplay/combat-system.md](gameplay/combat-system.md), [gameplay/ability-system.md](gameplay/ability-system.md).

1. Engage an NPC with basic weapon fire → hit/miss/crit results display, damage numbers sane, ammo decrements, NPC health drops, NPC dies and gives kill credit.
2. Reload flow: run a magazine dry → fire gates until `requestReload` completes; magazine refills.
3. Use at least **5 distinct abilities** across damage / heal / buff archetypes → each activates, respects its cooldown (spam-click to confirm), consumes what it should, and lands its effect.
4. Channelled ability: confirm channel bar, cancellation on movement (unless the ability allows movement), and cancellation on damage if applicable.
5. AoE ability: multiple targets take damage; eyeball falloff with near/far targets and note values for the calibration watch item.
6. **LOS watch item**: fire a targeted ability at an NPC fully behind a wall → document whether it lands (known gap: player `useAbility` checks range only). If it lands, record IM + issue, not CW.
7. Die to an NPC → death state, respawn flow, respawn point sane ([gameplay/death-respawn-system.md](gameplay/death-respawn-system.md)).
8. Combat state: `BSF_InCombat` visibly enters/clears correctly with one and with multiple aggroed mobs.

### P1.5 — Effects & buffs

1. Apply a buff, a debuff (from an NPC), a DoT, and a HoT → icons appear, tick values sane, durations expire on time, icons clear.
2. Stacking: apply the same effect twice and two different effects that interact → matches expected stacking semantics ([gameplay/effect-system.md](gameplay/effect-system.md)).
3. **Long-tail sample**: pick 10 effect rows at random from the 3,217 in the DB (spread across effect types), trigger each via ability/item where reachable → record the worked/inert ratio as the content-coverage evidence.

### P1.6 — Stats

1. Character sheet shows a full, sane stat list at creation and updates on level-up with per-level scaling.
2. Equip/unequip a stat-bearing item → document whether equipment bonuses apply (expected missing).
3. Confirm a stat-driven derived value (e.g. max health) changes when its base stat does.

### P1.7 — Inventory & items

1. Move, split, and stack items; fill a bag; attempt overfill → correct behavior incl. rejection.
2. Equip/unequip weapons and armor; verify appearance changes on your own avatar (witness-side appearance is P2.1).
3. Bandolier: place weapons in slots, swap between same-type and different-type weapons, verify per-slot ammo tracking survives swaps and relog.
4. Use a consumable → consumed exactly once, effect applies.
5. Slappack stacking override: verify the known stacking behavior on the overridden item.
6. Item persistence: covered by P0.4.

### P1.8 — Missions

1. Run the Castle Cellblock chain end-to-end: accept → objectives track (kill count, collect, visit, talk, use) → step advancement → turn-in → rewards granted.
2. Abandon a mission and re-accept → state resets cleanly.
3. Decline a mission offer → no stuck dialog state.
4. Mission persistence across relog mid-chain.
5. Sharing and mission-gated loot: attempt → expected missing, record.

### P1.9 — Loot

1. Kill lootable NPCs → loot window opens on corpse right-click, items and cash transfer, take-all works, corpse cleans up.
2. Walk out of range with a loot window open, then loot → per-item distance re-validation rejects.
3. Note empty-loot frequency as loot-table-content evidence (tables mostly empty is the expectation).
4. Eligibility: single-client can't test contested loot; flag for a P2 add-on if both testers can tag one mob.

### P1.10 — Vendors

1. Full loop at a Castle vendor: browse → purchase (cash decrements, item arrives) → sell (cash increments, item leaves) → **buyback** the sold item.
2. Repair a damaged item and recharge a rechargeable one → durability/charge restored, correct price charged.
3. Insufficient-funds purchase attempt → clean rejection, no partial state.

### P1.11 — XP & leveling

1. Kill XP: verify per-kill XP lands and scales with target level/relative level.
2. Level up → level-up presentation, stat scaling applies, training points granted.
3. **Mission XP watch item**: confirm mission turn-ins grant 0 XP (seed rows all 0) → record with evidence; that's KM-for-content, not code.
4. ASP on level-up: document (expected missing).

### P1.12 — NPC AI & behavior

1. Aggro: approach a hostile → aggros at a sane range; ranged attacks aggro from distance.
2. Observe patrol and wander behaviors on NPCs that have them.
3. Leash: drag a mob far from spawn → it leashes back and resets health.
4. Threat: with a healer-and-damage setup if possible (or damage-only) verify the mob targets sensibly and re-targets on threat changes.
5. Cover system: fight NPCs in a cover-noded area → NPCs observably move to and fire from cover positions; two NPCs never claim the same node.
6. Mob ability use: NPCs use abilities, not just autoattack.
7. Hearing radius / mob groups / kill-credit tapping: probe each → expected missing, record.

### P1.13 — Spawn system

1. Clear a spawn area, wait → respawn timers repopulate it at correct positions.
2. Observe population caps (a farmed area doesn't over-spawn).
3. Time-of-day spawns, detection radius, linked sets: probe → expected missing, record.
4. Spawned NPCs persist correctly across your relog (no doubled spawns).

### P1.14 — Crafting

1. On a character with blueprints, attempt every player-facing verb reachable in the UI: craft, research, reverse-engineer, alloy, ASP spend, respec → **expected: each fails / server logs `UNIMPLEMENTED`**. Record each verb KM with the log line as evidence.
2. Verify the Phase-1 state layer indirectly: blueprint ownership persists across relog.

### P1.15 — Stargate travel

1. Dial and traverse the Castle ↔ neighbor gate → cinematic plays, zone transition completes, arrival position correct, character state intact.
2. Return trip → document return-trip state handling (expected gaps).
3. Dial an invalid/locked destination → clean rejection.
4. Gate cooldown: probe → expected missing.
5. Multi-player gate sync is P2.5.

### P1.16 — Chat

1. `/say`, `/emote`, yell → render locally with correct ranges (verify ranges properly in P2.3).
2. Join state: confirm all 8 canonical channels appear joined, then send on a non-spatial channel (e.g. trade/help) → **expected: no traffic routes**; record those channel rows KM.
3. Tells (`/tell`): attempt → expected unported; confirm in P2.3 with a real second target.
4. DND flag: toggle and document what it observably does.

### P1.17 — Ring transport

1. Activate a cross-region ring transporter → full FSM runs (activation, effects/cinematic, transport, arrival), position correct, no stuck state.
2. Repeat for a cross-world ring.
3. Walk away mid-activation / log out mid-sequence → no undefined state on return (known risk area — test deliberately).

### P1.18 — Contact lists

1. Create a named list; add a contact (an offline character name), remove one; relog → list persists.
2. Flags: set per-contact flags and confirm they persist.
3. Presence events (login/level/death/gate) are P2.4.

### P1.19 — Mail

1. Send mail to one of your own alt characters: plain text, then with an item attachment, then with cash → each arrives, attachments detach correctly, mail deletes.
2. CoD and return-to-sender: attempt → expected missing, record.
3. New-mail notification without reopening the mailbox: observe → expected missing (fanout unbuilt).

### P1.20 — Minigames

1. **Livewire**: launch and play to a *legitimate* win and a *legitimate* loss → results reach the game (e.g. the door opens / mission advances on win; nothing on loss).
2. Launch each of the other reachable minigames (Hack, Bypass, Activate, Analyze, Converse, …): confirm they connect and complete, then **submit a garbage/instant result** where possible → expected: accept-anything placeholder accepts it. Record those games IM (placeholder validation) with evidence, not CW.
3. Alignment + GoauldCrystals: attempt to launch → expected TODO/broken, record.

### P1.21 — Content engine

Mostly verified through P1.8 missions — record it CW only if, additionally:

1. Dialog trees: run 10 distinct dialog interactions (options, branches, portraits, speaker names correct).
2. Region triggers: walking into a known trigger region fires its scripted behavior.
3. Consumable-driven chains: a `UseInventoryItem`-triggered chain fires exactly once per use (no double-consume).
4. **Cinematic watch item**: trigger a scripted sequence (gate dial is one) → the sequence fires; note that authored NameValuePair parameters (sound banks etc.) are expected absent (`sequences_nvp` unread). Missing polish = IM note, not a failure of the engine row.

### P1.22 — Economy

1. Reconcile one session's cash flow by hand: starting cash → vendor buys/sells, loot cash, mission rewards → ending cash. Every delta explained.
2. AH listing fees and cash-flow tracking: not testable (black market unmerged / tracking unbuilt) → record per verdict rules.

### P1.23 — Admin / GM commands

Reference: [commands.md](commands.md), the `.`-console doc ([architecture/dev-console-channel.md](architecture/dev-console-channel.md)).

1. On the **GM account**: `/` teleport to coordinates and to a named target → arrives, world streams in correctly, other-client view checked in P2.9.
2. Item grant → item appears in inventory.
3. Exercise 5 further GM `/` commands from the working set in [commands.md](commands.md) → behave as documented.
4. `.`-console: run 3 dev/authoring commands (e.g. a query + a record→confirm authoring flow) → work as documented.
5. **On the normal account**: attempt the same GM commands → server-side access-level gate rejects every one (this is a security check — a bypass here is a critical issue).
6. Ban/mute: attempt → expected missing, record KM.

---

## Phase 2 — Two-Client Verification

Requires the second client (see Test Environment). ⏱ marks steps needing genuine simultaneity (two humans, or accept reduced confidence).

### P2.1 — Entity lifecycle (AoI)

1. Client B walks into A's view → B's avatar appears promptly with correct appearance/equipment; walks out → disappears at a sane range.
2. A equips/holsters/swaps a weapon → B sees the appearance change without A relogging.
3. A dies → B sees the death; A respawns → B sees the respawn at the right place.
4. **Corpse repro (carried issue)**: stage the Castle Cellblock GuardBody scenario repeatedly (10+ attempts, varying approach) with B watching → if any entity introduction is missed (invisible corpse until relog), capture the moment plus the `aoi.create_emit` / `aoi.create_send_failed` (#582) log output. This repro is a first-class campaign goal.
5. B observes A's movement continuously for jitter/teleporting during normal running.
6. Long soak: both clients in-world 30+ min → no witness-list weirdness (entities that never appear or never disappear).

### P2.2 — Trading

1. Full happy path: propose → both add items + cash → both confirm → atomic swap; verify both inventories and cash on both screens **and** in the DB.
2. Cancel at each stage (before lock, after one lock, after one confirm) → both sides unwound, items unlocked.
3. ⏱ Modify-after-lock: one side changes the offer after the other confirmed → confirmation resets.
4. Disconnect mid-trade (kill client B's process) → A's trade cleanly cancels, no locked/duplicated items on either side after B relogs.

### P2.3 — Chat ranges & tells

1. `/say` and emote ranges: B walks to the edge of range → messages cut off at a sane distance; yell carries farther.
2. `/tell` from A to B → expected unported; record with evidence.

### P2.4 — Contact presence fanout

1. A adds B as a contact. B logs out and in → A receives the LoggedInStatus notification.
2. B levels up, dies, and gate-travels → A receives each event per the flag bitfield.

### P2.5 — Stargate multi-player sync

1. ⏱ A dials while B watches the gate → B sees the dial/kawoosh.
2. A and B traverse together → both arrive; each sees the other on the far side.

### P2.6 — Dueling

1. Attempt a duel challenge A→B → expected missing; record all 6 rows KM with the observed behavior.

### P2.7 — Groups / parties

1. Attempt a group invite A→B → expected missing; record KM.

### P2.8 — Mail between players

1. A mails B (text, item, cash) → B receives on next mailbox open; attachments correct.

### P2.9 — Anti-cheat & rate limiting

Perform on a **designated cheat-test account**; B observes, SigNoz captures.

1. Teleport-shaped movement: use any means available (packet-timing abuse, a modified speed if a test hook exists — otherwise the GM teleport as the *authorized* control case) to present an impossible position jump from the **normal** account → server hard-rejects + snaps back, and B never sees the cheated position. GM teleport (authorized) must **not** trigger the same rejection.
2. Speed: sustain maximum legitimate speed → no false-positive corrections; check the server log for the warn-only speed layer's output and record its calibration state.
3. Ability range: attempt to use an ability on an out-of-range target (client UI usually prevents; try edge-of-range) → server-side range check holds.
4. Cooldown enforcement: spam an ability → server rejects early re-use even under client-side spam.
5. Rate limiting: flood chat and rapid-fire actions → expected: no throttling exists; record KM for the throttle rows and note observed server behavior under the flood.
6. Max-damage cap: not client-testable without a damage exploit; record per verdict rules with a pointer to the audit finding.

---

## Phase 3 — Expected-Missing Confirmation

Quick single-session sweep confirming the stub systems are actually inert from the client. For each: attempt the entry-point UI/command, capture the failure, record KM.

### P3.1 — Organizations / guilds

Attempt create/invite/roster via whatever UI or command surface exists → expected stubs; all 15 rows KM.

### P3.2 — Black market

Attempt to open/search/list on the auction UI → expected stubs on `main` (Phase 1 exists only on the unmerged `feat/571-black-market-phase1`). Record KM with the scope note.

### P3.3 — Pets

Attempt pet summon items/abilities if any are obtainable → expected missing; 7 rows KM.

---

## Phase 4 — Multi-Zone Sweep

Repeat a **condensed loop** in every populated zone beyond Castle Cellblock, prioritised by the zone audit ([content/zone-audit.md](content/zone-audit.md)): the PLAYABLE and PARTIAL zones get the full condensed loop; transport-only zones get steps 1–2; SHELL zones get step 1 plus a note of what's absent.

Condensed loop per zone:

1. **Enter** (gate/ring/GM teleport) → zone loads, spawn position valid, no server error spam.
2. **Traverse** a representative path → collision/navmesh sane, NPCs visible and named.
3. **Fight** one NPC group → combat + loot + XP function.
4. **Interact** — one dialog NPC and one mission step if the zone has content.
5. **Exit** via the zone's travel mechanism → arrival correct.

Log one session-log row per zone. The Content Coverage "Zones" row in project-status.md counts zones that completed the loop applicable to their tier.

---

## Phase 5 — Not Client-Observable

These systems can't be verified by a human at the retail client. Verify by the stated alternative; record with the same evidence discipline and mark the method in the notes.

### P5.1 — Mercury bundle

Internal transport optimisation. Verified by: SigNoz packet logs during a Phase 1 AoI-burst moment (many entities entering view) showing bundled packets + no client-visible artifacts during that moment. Client-side corroboration: no hitching/missing entities during the burst.

### P5.2 — Wireclient & chaos harness

Developer tooling, not a player system. Verified by: running the documented harness commands ([architecture/wireclient.md](architecture/wireclient.md), [architecture/network-chaos-testing.md](architecture/network-chaos-testing.md)) and confirming they do what their docs claim. The known gap stands: no UDP socket loop / no live replay — confirm and record honestly.

### P5.3 — Tauri admin app & tools

Verified by: a human driving each app — admin panel (view live sessions while a Phase 1 session is running), content editor (open, edit, save a chain; verify the edit fires in-game), scene editor, launcher (already exercised in P0.1). The launcher row can inherit its Phase 0 evidence.

### P5.4 — World state & scheduler

1. Outbox durability: already evidenced by P0.4 step 2 — inherit.
2. Gate/door persistent state: open a door / change world state, restart server → expected: state lost (no world-state table); record KM with evidence.
3. Scheduler: observe a content-engine timer chain firing on schedule in-game (mission timer from P1.8); global cron absent → record split verdict.

---

## Session Log Template

Store as `docs/testing/validation-sessions/YYYY-MM-DD-<phase-or-zone>.md`, one per session, committed with the doc updates it justifies. (First session in the campaign creates the directory.)

```markdown
# Validation Session — <date> — <phase/section>

- Server commit: <SHA>  | Client build: <version>  | DB seed: <fresh/carried>
- Tester(s): <names>    | Clients: <1 or 2, machines>
- Checklist sections run: <P1.4, P1.5, …>

| Step | Observed | Verdict | Evidence | Issue |
|------|----------|---------|----------|-------|
| P1.4.1 | … | CW | screenshot link / SigNoz trace / log excerpt | — |
| P1.4.6 | ability landed through wall | IM | video link | #NNN |

## Notes / anomalies
…
```

## Related Documents

- [project-status.md](project-status.md) — the tables this campaign repopulates
- [gap-analysis.md](gap-analysis.md) — per-feature rows updated from session logs
- [guides/getting-started.md](guides/getting-started.md) / [building.md](building.md) / [multiplayer.md](multiplayer.md) — environment setup
- [commands.md](commands.md) — the `/` command surface for P1.23
- [content/zone-audit.md](content/zone-audit.md) — zone prioritisation for Phase 4
- [known-issues.md](known-issues.md) — file new defects found during sessions here as well as GitHub
