# Triage findings — batch `features`

Code of record: `main-ro` (origin/main 059d6038, 2026-09-25). All paths below are relative to `main-ro`.

## #572 — Implement contact list system (friends/ignore, presence fanout, persistence)

- Verdict: REWRITE
- Priority: P3
- Labels: no change (keep `enhancement`)
- Summary: The epic's Phases 1–4 all landed: schema, login push, CM 55–60, and online/offline presence (#574), the eventId bitfield fix (#578), GainLevel/Death/GateTravel events (#579), initial presence for the player logging in (#581), and lighting up a contact who is already online when added (#583). The owner confirmed contact lists working on 2026-06-20. D.1 and D.3 are resolved (the owner's comments and the client's Social.lua). Only Phase 5 is still open. The `chatIgnore`/`chatFriend` Communicator base methods (0xC5/0xC6) have no handler and fall into the "Unhandled SGWPlayer base method" warn arm. There is also no ignore-list filtering in chat. The title and body still describe a greenfield system at 15%, so they should be cut down to that remainder.
- Evidence:
  - `crates/services/src/base/contact_list/{handlers/{header_ops,member_ops,presence_fanout}.rs,persistence/,wire.rs}`; login push at `crates/services/src/base/world_entry_appearance/client_ready/mod.rs:431,453`; logout fanout at `crates/services/src/base/dispatch/session.rs:88`.
  - Rich events wired: `base/world_entry/methods/progression/mod.rs:11-12` (GainLevel), `cell/abilities/death/mod.rs:34` (Death), `base/world_entry/gate_travel/mod.rs:17-18` (GateTravel).
  - Merged PRs #574, #578, #579, #581, #583.
  - Phase 5 gap: `crates/services/src/base/dispatch/mod.rs:40-70` defines only 0xC0–0xC4, 0xD5–0xD8 and PERF_STATS. 0xC5/0xC6 reach the warn arm at `mod.rs:151`.
  - Doc drift: `docs/protocol/sgwplayer-base-method-dispatch-table.md:42-43` lists `chatIgnore`/`chatFriend` as `WSTRING playerName` only. The canonical `entities/defs/interfaces/Communicator.def:149-160` has `chatIgnore(WSTRING aPlayerName, UINT8 aFlag)` and `chatFriend(WSTRING aPlayerName, WSTRING aPlayerNick, UINT8 aFlag)`. Fix the dispatch table in the same PR.
  - No ignore filter in `crates/services/src/cell/chat.rs` or `base/dispatch/chat.rs`. The security side of this is also CAT-L-01 and CAT-L-07 in #471.
- Related/duplicates: #471 (CAT-L-01 ignore filter, CAT-L-07 chatFriend/chatIgnore unhandled); superseded #71 and #275 (both closed).

### Action text

**Comment:**
> Triage 2026-09-25: Phases 1–4 of this epic are merged and owner-confirmed working: #574 (schema, login push, CM 55–60, presence), #578 (eventId bitfield), #579 (GainLevel/Death/GateTravel), #581 (initial presence) and #583 (already-online contact lights up when added). The D.1 and D.3 RE items are resolved in the comments above. The one piece left is Phase 5, the `chatFriend`/`chatIgnore` Communicator base methods (0xC5/0xC6). They currently hit the unhandled-base-method warn arm, and chat has no ignore-list filter. I've rewritten the title and body to cover only that remainder. Note that the base-method dispatch table has the wrong arg lists for these two methods; `Communicator.def` is correct.

**New title:** Contact list: wire `chatFriend`/`chatIgnore` (base 0xC5/0xC6) and ignore-list chat filtering

#### New body

## What

Contact lists (CM 55–60 plus presence events 1/2/4/8) are done and owner-confirmed working (#574, #578, #579, #581, #583). Two contact-list features are still missing:

1. **`chatFriend` / `chatIgnore` base methods.** These are Communicator base methods 5 and 6 (wire 0xC5/0xC6). Right now they fall through to the `Unhandled SGWPlayer base method` warn arm in `crates/services/src/base/dispatch/mod.rs`, so the button does nothing on the first press. Canonical signatures from `entities/defs/interfaces/Communicator.def:149-160`:
   - `chatIgnore(WSTRING aPlayerName, UINT8 aFlag)`
   - `chatFriend(WSTRING aPlayerName, WSTRING aPlayerNick, UINT8 aFlag)`

   With `aFlag = 1`, add `aPlayerName` to the caller's system Ignore/Friends list. With `aFlag = 0`, remove it. Reuse `base/contact_list/persistence` and `handlers/member_ops.rs` so the client gets the same `onContactListAddMembers` [87] / `onContactListRemoveMembers` [88] echo as the CM 59/60 path, plus the online-presence event when a friend is added.
2. **Ignore-list chat filtering.** When a speaker is on the recipient's Ignore list, the server should drop direct (tell) messages and the recipient's copy of broadcast channels. There is no filter anywhere today (`cell/chat.rs`, `base/dispatch/chat.rs`).

## Acceptance criteria

- [ ] 0xC5/0xC6 are handled and no longer log the unhandled-method warn. Add/remove goes through the existing ownership-scoped persistence, with the ≤100-name clamp and no self-add.
- [ ] The first press gives visible feedback: an 87/88 echo, or an error on the feedback channel for an unknown player name.
- [ ] A speaker on the recipient's Ignore list never reaches the recipient's client on any chat channel.
- [ ] `docs/protocol/sgwplayer-base-method-dispatch-table.md` rows 5 and 6 are corrected to the `.def` arg lists.

## Open question

Does the stock client's `/friend` and `/ignore` UI emit `chatFriend`/`chatIgnore` (0xC5/0xC6) or `contactListAddMembers` (CM 59)? Confirm before scoping. If nothing in the client emits 0xC5/0xC6, only item 2 remains. The emit functions are `Event_NetOut_ChatFriend`/`ChatIgnore` in `docs/protocol/message-catalog.md:182-183`.

## Tests

- Wire-format: decode both base-method payloads byte-exact, including the WSTRING+WSTRING+UINT8 shape of `chatFriend`.
- Live-DB: flag=1 adds and flag=0 removes on the correct system list. Reverting the ownership scoping must fail the test.
- Unit: the ignore filter drops an ignored speaker's message and does not drop a non-ignored one.

## Related

#471 (CAT-L-01 ignore filter, CAT-L-07 unhandled chatFriend/chatIgnore). RE: `docs/reverse-engineering/findings/contact-list-restoration.md`.

---

## #571 — Implement Black Market / Auction House system (listings, bids, expiry, CoD delivery)

- Verdict: KEEP (plus an owner decision on PR #586)
- Priority: P3
- Labels: no change
- Summary: Nothing server-side has reached main. `cell/cell_methods/black_market.rs` is still an 80-line stub file, and it still decodes `auctionLength` as INT32 at `args[12..16]` in the wrong field order. There is no `base/black_market/`, no `sgw_auction` tables, no `send_mail_to_player` helper, and nothing sends `onBMOpen` (90). PR #586 implements Phases 1–3 plus search, sweep and a content-engine BM action. It has been open since 2026-06-22 and is now `DIRTY` (it conflicts and touches `castle_cellblock_chains.sql`, which the Castle campaign has since rewritten). Several blocked-on items have since been recovered: `EBlackMarketError` = 1/2, `auctionLength` = `EBlackMarketTime` 1–5, and the createAuction order is item, buyout, length, starting. The feature is invisible without the client patch tracked in #587, because client methods 90–95 are never bound in the stock client.
- Evidence:
  - `crates/services/src/cell/cell_methods/black_market.rs:29-32`: `duration_days = i32::from_le_bytes(args[12..16])`. The canonical `entities/defs/interfaces/SGWBlackMarketManager.def:50-56` is INT32 itemInstanceId, INT32 buyoutPrice, UINT8 auctionLength, INT32 startingPrice (13 bytes).
  - `crates/services/src/cell/client_methods/black_market.rs:4`: `ON_BM_OPEN = 90` is defined but never sent (grep finds no call site).
  - `db/sgw/` has no BlackMarket directory.
  - PR #586: OPEN, `mergeStateStatus=DIRTY`, 59 files, last updated 2026-06-22.
  - `docs/reverse-engineering/findings/black-market-client-window-patch.md:113,118-130,236-250`: recovered values, and the fact that only method 90 is revived client-side (91–95 are still dropped and need a native binding).
  - The body says "@Steve"; the owner's handle is @Cadacious.
- Related/duplicates: #587 (client window patch, a hard prerequisite for anything to be visible), #468 (CAT-I security findings), #72 (mail delivery path for CoD/sweep), superseded #67 (closed).

### Action text

**Status comment:**
> Triage 2026-09-25: None of the server-side BM work is on main yet. `cell_methods/black_market.rs` still decodes `auctionLength` as INT32 (the `.def` says UINT8, 13-byte payload), there is no `base/black_market/` or `sgw_auction` schema, and `onBMOpen` (90) is never sent. PR #586 has Phases 1–3 plus search and sweep, but it has been conflicting since June. It also edits `castle_cellblock_chains.sql`, which the Castle rebuild has rewritten since then. RE has recovered several of the "blocked-on" items: `EBlackMarketError` = 1/2, `auctionLength` = `EBlackMarketTime` 1–5, createAuction order item/buyout/length/starting (`black-market-client-window-patch.md` §Implementation impact). Players see nothing without the #587 client patch, and that patch currently only revives method 90, so the listing tabs stay empty until 91–95 are bound natively. @Cadacious, do you want #586 rebased onto main, or closed and re-cut per phase after #587's scope is decided?

---

## #570 — Implement pet / companion system (spawn, command dispatch, ownership, AoI sync)

- Verdict: KEEP
- Priority: P3
- Labels: no change
- Summary: The body is accurate. The only pet code on main is still the three command stubs (CM 88/89/90) in `cell_methods/player/social.rs`. There is no SGWPet entity, no spawn/despawn, and no stance or ownership logic. The body's wire-format warning also still applies: `docs/reverse-engineering/findings/pet-wire-formats.md` still lists `onPetStanceList` as `ARRAY<INT32>` and `onPetStanceUpdate` as INT32 (5 bytes). The canonical `entities/defs/SGWPet.def:87-93` has `ARRAY<INT8>` and `INT8`. Pets would also need the Follow behavior from the NPC-AI side.
- Evidence:
  - `crates/services/src/cell/cell_methods/player/constants.rs:26,28`; `cell_methods/player/social.rs:15,47` (stubs); `cell/spawner/npcs.rs:133-139` (`class_id_for_class` has no pet mapping).
  - `docs/reverse-engineering/findings/pet-wire-formats.md:20-31` is still the uncorrected INT32 version.
  - #69 is an older duplicate of this issue (see below).
- Related/duplicates: #69 (duplicate, close in favor of this issue), #48 (NPC AI, Follow state).

### Action text

**Status comment:**
> Triage 2026-09-25: Still accurate. Pet code on main is only the CM 88/89/90 stubs. The correction in the body is still needed: `pet-wire-formats.md` still says `onPetStanceList` is ARRAY<INT32> and `onPetStanceUpdate` is INT32, while `entities/defs/SGWPet.def:87-93` says INT8 for both. Fix the doc in the Phase 1 PR. Closing #69 as a duplicate of this issue.

---

## #569 — Implement duel system — challenge handshake, arena marker, PvP flag lifecycle

- Verdict: REWRITE
- Priority: P3
- Labels: no change
- Summary: The duel itself is still unimplemented: CM 102/103 log `UNIMPLEMENTED`, client-method constants 143 and 151–153 exist but are never sent, and there is no SGWDuelMarker. Much of the body's "hard dependency" section is out of date, though. `send_entity_method_to_self_and_witnesses` is no longer dead code: it has 10 call sites across damage_apply, death, messaging and console. The idbase-61 player-ghost bug is fixed and pinned by a test. #232 (death broadcast) is closed, and PR #737 added player-to-player AoI. #278 and #279 are still open, so appearance recomposite remains a partial dependency. D.3 is answered by the docs: `sendDuelChallenge` is SGWPlayer base method 25, wire 0xD9. Several file paths and line numbers have also moved.
- Evidence:
  - Stubs: `crates/services/src/cell/cell_methods/player/social.rs:92-101`; constants: `cell/client_methods/player.rs:94,110-114`.
  - Fanout now live: `cell/abilities/damage_apply/mod.rs`, `cell/abilities/death/{mod,side_effects}.rs`, `cell/abilities/messaging.rs`. Idbase test: `base/world_entry/cell_dispatch/tests_dispatch_arms/witness_broadcast.rs:174` (`witness_entity_method_player_ghost_uses_idbase_61_npc_uses_62`).
  - D.3: `docs/protocol/sgwplayer-base-method-dispatch-table.md:105`, which gives `| 25 | 0xD9 | sendDuelChallenge | WSTRING playerName, INT8 squadDuel |`.
  - Moved paths: the faction gate is `cell/abilities/use_ability/handle.rs:221-224` (body says :223, close enough). The AoE gate is now `cell/abilities/dispatch.rs:114-124` (body says `cell_methods/player/dispatch.rs`). The cone gate is `cell/abilities/cone_aoe/geometry.rs:87` (body says `cone_aoe.rs`). The 2× hack is `cell/abilities/damage_apply/mod.rs:139-142` (body says :133). The threat guard is the `generate_threat` player early-return in `cell/combat/threat/aggro.rs`, pinned by the test at :329.
  - #232 CLOSED 2026-06-20; #278 and #279 OPEN.
- Related/duplicates: #278, #279, #737 (player-to-player AoI), #472 (CAT-M security findings), superseded #70 (closed).

### Action text

**Comment:**
> Triage 2026-09-25: The duel is still unimplemented (CM 102/103 stubs, no SGWDuelMarker), but the body's prerequisites section is out of date. `send_entity_method_to_self_and_witnesses` is now used in damage and death (10 call sites). The idbase-61 player-ghost encoding is fixed and pinned by `witness_entity_method_player_ghost_uses_idbase_61_npc_uses_62`. #232 is closed, and #737 added player-to-player AoI. What remains of the dependency is #278/#279 (appearance recomposite and any stragglers). D.3 is answered in `sgwplayer-base-method-dispatch-table.md:105`: `sendDuelChallenge` is base method 25 (0xD9). I've rewritten the body with the current file paths.

#### New body

## Context

The duel system was never implemented server-side, either in the original game (the Python `SGWDuelMarker` is a skeleton) or here. The client ships all of it. RE: `docs/reverse-engineering/findings/duel-restoration.md` and `duel-wire-formats.md`.

Current Rust:

- `sendDuelResponse` (CM 102) and `duelForfeit` (CM 103) log `UNIMPLEMENTED` in `crates/services/src/cell/cell_methods/player/social.rs:92-101`.
- `onDuelChallenge` [143] and `onDuelEntitiesSet/Remove/Clear` [151–153] are defined in `cell/client_methods/player.rs` but never sent.
- There is no `SGWDuelMarker` entity (type index 6) and no PvP-flag lifecycle.

State enum: `EDUEL_STATE_{None=0,ResponsePending=1,Challenged=2,StartPending=3,Engaged=4}`. Defeat reasons: `Health=1,LeftSquad=2,Connection=3,Range=4,Teleport=5,InDuel=6,Forfeit=7`.

## Prerequisite status (witness fanout)

Mostly in place. `send_entity_method_to_self_and_witnesses` drives damage and death fanout (`cell/abilities/damage_apply/`, `cell/abilities/death/`). Player ghosts encode with idbase 61. #232 is closed, and #737 added player-to-player AoI. Still open: #278 (umbrella) and #279 (appearance recomposite on equip change). A spectator won't see weapon draw/holster correctly until #279 lands. Verify the first duel with a real two-client plus spectator session.

## Phase 1: challenge handshake (base)

`sendDuelChallenge` is SGWPlayer base method **25 (wire 0xD9)**: `WSTRING playerName, INT8 squadDuel` (`docs/protocol/sgwplayer-base-method-dispatch-table.md:105`). The handler resolves name → entity, validates (both alive, neither in a duel, same space), and forwards to the cell. Keep an in-memory `duel_state` per player; it is not persisted.

## Phase 2: cell challenge and client notify

Set the target to `Challenged`, then send `onDuelChallenge` [143] (`INT32 challengerId`, `ARRAY<INT32> squad`) and `Event_UI_DuelTimerStart`. The duration is provisional (30 s) until D.1 is pinned.

## Phase 3: response handling

Replace the CM 102 stub. Accept moves to `StartPending`; decline resets both players to `None`. A timeout task aborts on no response. The first press must give visible feedback.

## Phase 4: SGWDuelMarker entity

New cell entity (type index 6) with `duel_entities` and `duel_detector_id`. `onEntityDefeated` removes the entity and sends `onDuelEntitiesRemove` [152]. When the set is empty, send `onDuelEntitiesClear` [153], clear PvP, and destroy the marker.

## Phase 5: arena entry and PvP flag

Send `onDuelEntitiesSet` [151]. Set `GENERICPROPERTY_PvPFlag` (4) = 1, fanned out to witnesses, and clear it at the end. D.7: confirm whether the duel sets PvPFlag server-side, or whether marker membership alone gates combat.

## Phase 6: PvP combat enablement

- Let damage through between the two duel partners only, at all three gates:
  - `cell/abilities/use_ability/handle.rs:221-224` (single target)
  - `cell/abilities/dispatch.rs:114-124` (ground AoE)
  - `cell/abilities/cone_aoe/geometry.rs:87` (cone)

  Bystanders must stay untouchable.
- Exclude duels from the temporary 2× player-damage hack at `cell/abilities/damage_apply/mod.rs:139-142`.
- Non-lethal end: clamp HP to 1 and route to duel-end before the death transition, so there is no corpse, loot, credit or respawn. D.6 is whether to use clamp-to-1 or kill-and-respawn.
- `BSF_InCombat`: keep the `generate_threat` player-target guard (`cell/combat/threat/aggro.rs`). Give duels their own combat source with a symmetric set and clear on both duelists.

## Phase 7: disconnect, teleport, range and forfeit

Route each to `onEntityDefeated` with the matching `EDUEL_DEFEAT_*`: forfeit (CM 103) = 7, disconnect = 3, teleport = 5, leave-squad = 2, range = 4 (D.2).

## Phase 8: squad duel

Defer until 1v1 works.

## Still-open RE (x64dbg, @Cadacious)

D.1 timer duration (BP `0x00cd65a0`); D.2 range limit; D.5 response-byte polarity; D.6 non-lethal vs respawn; D.7 PvPFlag semantics. D.3 is resolved (base index 25).

## Tests (per TESTING.md)

- Wire-format: `onDuelChallenge` and `onDuelEntitiesSet` byte-exact, plus `onEntityProperty(4,1)`.
- Unit: state transitions, partner-only gating (bystander untouchable), non-lethal clamp, symmetric in-combat clear, and routing for each defeat reason.
- Mercury-session: two duelists plus a spectator.

## Related

#278, #279, #737, #472 (CAT-M security), superseded #70.

---

## #568 — Implement Organization / Squad / Guild system (create, invite, ranks, roster, permissions, fanout)

- Verdict: KEEP (plus an owner decision on PR #584)
- Priority: P2
- Labels: no change
- Summary: On main, the org surface is still stubs: 12 `UNIMPLEMENTED` arms in `cell_methods/organization.rs`, and no `db/sgw/Organizations/`. The CM 17 `organizationSetRankName` stub still parses only the org_id and rank ints and drops the trailing WSTRING. The body's ⚠️ pre-work note is obsolete: `crates/game/src/social/guilds.rs` (the 3-rank `GuildRank`) was deleted in #699. PR #584 implements Phases 1–3 (OrgRank/OrgPermission models, schema, SquadManager, OrgAuthority, fanout, ~65 tests). It has been open since 2026-06-21 and is `DIRTY`. I rated this P2 because squads are the basic multiplayer grouping, and the owner's 2026-06 status note lists multiplayer as not done.
- Evidence:
  - `crates/services/src/cell/cell_methods/organization.rs` (162 lines, 12 `UNIMPLEMENTED`); `:127-135` SET_RANK_NAME logs `org_id`/`rank` only.
  - `crates/game/src/social/` was removed by commit 03cc48a2 (#699, "delete the unused social module").
  - PR #584: OPEN, `mergeStateStatus=DIRTY`, 55 files, last updated 2026-06-21.
- Related/duplicates: #472 (CAT-M security), superseded #68 (closed).

### Action text

**Status comment:**
> Triage 2026-09-25: Nothing from this epic is on main yet. The org cell methods are still 12 `UNIMPLEMENTED` stubs, and CM 17 still drops the rank-name WSTRING. The "pre-work bug" in the body no longer applies: #699 deleted `crates/game/src/social/guilds.rs` along with its 3-rank `GuildRank`. PR #584 has Phases 1–3 but has been conflicting since June. @Cadacious, should #584 be rebased and reviewed as-is, or split into Phase 1 (schema/models) and Phase 2 (ephemeral squads) so squads can land first?

---

## #587 — Launcher: integrate Black Market client-window runtime patch (deferred wide-Lua-injection)

- Verdict: NEEDS-OWNER
- Priority: P3
- Labels: add `enhancement`, `needs-triage` (it currently has no labels)
- Summary: The launcher does not apply this patch yet. `crates/launcher` has no BM or detour code. The acceptance criterion is also not reachable as written. The body says "the server already sends `onBMOpen` (method 90)", but on main nothing sends it; that code exists only on the unmerged PR #586 branch. The finding doc also says the method-90-only patch opens an empty window. Getting listings to render needs native bindings for 91–95 (C++ store-write handlers that are not yet located). The repo now has a second way to deliver the patch: `crates/client-launch` does `CreateProcess(SUSPENDED)` plus DLL injection, and `cimmeria-client-telemetry` already contains inline-hook/detour primitives. That may be a better vehicle than hand-written caves in the egui launcher. Under the project rules, shipping any client patch needs a maintainer decision.
- Evidence:
  - `crates/launcher/src/**`: no match for black/BM/detour/VirtualAllocEx.
  - `crates/client-launch/src/inject.rs:4-5,199-233` (VirtualAllocEx/WriteProcessMemory injection); `crates/client-telemetry/src/bridge/dynamic_hooks/native.rs:6-26` (inline-hook detours).
  - `crates/services/src/cell/client_methods/black_market.rs:4`: `ON_BM_OPEN` has no call site on main.
  - `docs/reverse-engineering/findings/black-market-client-window-patch.md:14` ("revives only method 90 … tabs render empty"), `:118-130`, `:236-250` (92–95 write a C++ store; the store-write functions still need locating).
  - CLAUDE.md project rule: "Prefer server-authoritative changes that need no client patch … new client UI need a maintainer decision first".
- Related/duplicates: #571 (server side), PR #586, #74 (launcher).

### Action text

**Owner questions (post as a comment):**
> Triage 2026-09-25: Before this can be scheduled, it needs three decisions.
> (1) **Ship a client patch for BM at all?** The server side (#571/#586) isn't on main. Right now nothing sends `onBMOpen`, so even a launcher-applied patch would open nothing.
> (2) **Scope.** A patch for method 90 only opens an empty window (finding doc line 14). Should this issue widen to "bind 90–95", including locating the 92–95 store-write handlers, or ship the 90-only patch as a first step?
> (3) **Delivery vehicle.** Hand-written caves in the egui launcher, as described, or the existing `client-launch` injection plus the `client-telemetry` inline-hook primitives? The second option already handles process creation, remote allocation and detours, and could host the native-binding recipe from the finding.

---

## #72 — Mail system: send, receive, attachments, COD

- Verdict: REWRITE
- Priority: P2
- Labels: no change
- Summary: The 2026-05-27 triage at the top of the body is wrong, and `docs/gap-analysis.md` §24 repeats the error. Only 4 of the 9 mail cell methods work on main: requestMailHeaders, requestMailBody, archive and delete. `sendMailMessage` (44), `returnMailMessage` (47), `takeCashFromMailMessage` (49), `takeItemFromMailMessage` (50) and `payCODForMailMessage` (51) all log `UNIMPLEMENTED` and still return "handled". The client's Send button therefore does nothing and gives no feedback on the first press. There is no `onNewMail`/`notifyPlayersOfNewMail` path and no expiry column on `sgw_gate_mail`. The "live-DB tests for Send" the triage cites are receive-side tests (headers/body/multi-character isolation). I rated this P2 because a UI action silently no-ops, and #466 lists these handlers as High security findings for when they get wired.
- Evidence:
  - `crates/services/src/cell/cell_methods/mail.rs:32-35` (send), `:52-57` (return), `:67-72` (take cash), `:74-87` (take item), `:90-95` (pay COD): all `UNIMPLEMENTED`, all return `true`.
  - `crates/services/src/cell/messages/data.rs:5-14`: `MailOp` has only RequestHeaders, RequestBody, Delete and Archive.
  - `crates/services/src/base/world_entry/methods/mail/mod.rs` (316 lines) handles just those four ops.
  - `db/sgw/Mail/Tables/sgw_gate_mail.sql`: no `expires_at`, no COD amount column (COD would go in `flags` plus `cash`).
  - `crates/services/src/cell/client_methods/mail.rs`: 76–79 are defined; `sendMailResult` (79) is never sent.
  - `docs/gap-analysis.md:597-600` wrongly lists "Send mail | IM". `docs/gameplay/mail-system.md:140` correctly says "Mail Send Flow (not implemented)".
  - #466 CAT-G-01..06.
  - PR #586 (unmerged) contains a `send_mail_to_player` helper that could be reused.
- Related/duplicates: #466 (CAT-G security), #571 (auction CoD/sweep uses mail delivery), PR #586.

### Action text

**Comment:**
> Triage 2026-09-25: The 2026-05-27 triage in this body overstated what's done. Checked against main, only headers, body, archive and delete work. `sendMailMessage` (44), `returnMailMessage` (47), `takeCash` (49), `takeItem` (50) and `payCOD` (51) are all `UNIMPLEMENTED` stubs that still report "handled", so the client's Send button silently does nothing. There is also no new-mail notification and no expiry. `docs/gap-analysis.md` §24 has the same error ("Send mail | IM") and should be fixed together with this issue. I've replaced the body with the real remaining scope, cross-referenced to the #466 security findings each handler has to satisfy.

#### New body

## What

In-game mail. The read side works on main: `requestMailHeaders` (43), `requestMailBody` (48), `archiveMailMessage` (45) and `deleteMailMessage` (46), backed by `sgw_gate_mail`, with live-DB tests in `crates/services/src/base/world_entry/methods/mail/tests.rs`.

**Everything on the write side is a stub.** Each of these logs `UNIMPLEMENTED` but returns "handled" (`crates/services/src/cell/cell_methods/mail.rs`):

| CM | Method | Args (`entities/defs/interfaces/SGWMailManager.def`) |
|---|---|---|
| 44 | `sendMailMessage` | INT32 RecipientFlags, ARRAY<WSTRING> Recipients, WSTRING Subject, WSTRING Body, INT32 Cash, UINT8 bCOD, INT32 ItemId, INT32 ItemQuantity |
| 47 | `returnMailMessage` | INT32 MailId |
| 49 | `takeCashFromMailMessage` | INT32 MailId |
| 50 | `takeItemFromMailMessage` | INT32 MailId, INT32 ContainerId, INT32 SlotId |
| 51 | `payCODForMailMessage` | INT32 MailId |

The server never sends `sendMailResult` (client method 79: UINT8 ResultCode, ARRAY<WSTRING> FailedRecipients, INT32 FailedRecipientFlags), so pressing Send gives no feedback. The new-mail path (`onNewMail` cell method, `notifyPlayersOfNewMail` base method) is also missing.

## Acceptance criteria

- [ ] **Send (44).** Resolve recipients. Debit attached cash and move the attached item into escrow in **one sqlx transaction** with the `sgw_gate_mail` insert. Always reply `sendMailResult` (79), including the failed-recipient list. `MailOp` gains the new ops.
- [ ] **Take cash (49) / take item (50).** Ownership-checked. Atomic with clearing the attachment so a double-take is impossible (CAT-G-02..04). `take item` handles inventory-full.
- [ ] **COD (51).** The recipient pays the sender's COD amount before taking the item. The debit, the credit (by return mail to the sender) and the item release happen in one transaction. The COD amount is server-side and never client-supplied (CAT-G-05).
- [ ] **Return to sender (47).** Swaps sender and recipient and keeps the attachment. Blocked while there is an unpaid COD (CAT-G-06).
- [ ] **New-mail notification.** An online recipient gets a fresh `onMailHeaderInfo` push (or whatever the client expects from `onNewMail`) when mail arrives.
- [ ] **Expiry (decision needed).** Either add `expires_at` to `db/sgw/Mail/Tables/sgw_gate_mail.sql` plus a sweep that returns unread mail, or declare mail non-expiring. Coordinate with the auction sweep in #571.
- [ ] Every button press produces visible client feedback on the first press.
- [ ] Fix `docs/gap-analysis.md` §24 (it lists Send as implemented) and update `docs/gameplay/mail-system.md`.

## Tests (per TESTING.md)

- Live-DB for each write op, including negative guards: taking someone else's mail, double-take, COD underfunded, return during unpaid COD. Each guard must fail if its WHERE-clause scoping is reverted.
- Wire-format for `sendMailResult` byte-exact.

## Related

#466 (CAT-G-01..08), #571 (auction CoD and expiry reuse mail delivery; PR #586 has a reusable `send_mail_to_player` helper).

---

## #69 — Pet system: summoning, abilities, stances

- Verdict: CLOSE (not planned, duplicate)
- Priority: P3
- Labels: no change
- Summary: #570 fully supersedes this issue. #570 was filed 2026-06-20 from the RE assessment and has phased scope, wire-format corrections, the ownership spoofing guard, and the open summon→template binding list. #69 adds nothing #570 lacks except its in-game test checklist, which I copy into the closing note. #69's "no Rust pet code" claim is also slightly stale: the CM 88/89/90 stubs exist.
- Evidence:
  - #570 body vs #69 body (same scope).
  - `crates/services/src/cell/cell_methods/player/social.rs:15,47`.
- Related/duplicates: #570 (survivor).

### Action text

**Closing comment (reason: not planned, duplicate of #570):**
> Closing as a duplicate of #570, which tracks the pet system from the RE assessment (`pet-restoration.md`) with phased scope, the INT8 stance wire corrections, and the summon→template binding list. When #570 is implemented, please carry over this issue's in-game validation checklist: summon, follow, attack on command, stance change (passive/aggressive), pet death, dismiss.

---

## #66 — Minigame framework: port Livewire, GoauldCrystals, Alignment from Python

- Verdict: REWRITE
- Priority: P3
- Labels: no change
- Summary: Most of this issue is done. The SmartFoxServer 1.x host, the ticket/session registry, session expiry, abort-on-close, content-triggered `StartMinigame` (with difficulty), the victory-chain callback, the full Livewire port, and the six auto-win placeholders are all on main. Livewire is already used by Castle mission 701 (#659). The body's triage claim that there is "no actual minigame logic in Rust" is false. What remains: Alignment and GoauldCrystals ports (factory arms still commented out, and an unknown name silently falls back to an auto-win). Tech competency is hardcoded to 1. The player-facing MinigamePlayer cell methods 20–34 (manual start, debug start, spectate, helper call protocol, NPC contacts, session-cancel) are stubs. No seeded content currently launches Alignment or GoauldCrystals, so the ports are low priority.
- Evidence:
  - `crates/services/src/minigame/{server/{mod,framing,handshake,result_dispatch}.rs,session.rs,protocol.rs,games/livewire/,games/placeholder.rs}` (about 3,400 lines).
  - `crates/services/src/minigame/games/mod.rs:13-15` (commented-out Alignment/GoauldCrystals arms) and `:19-22` (unknown name → placeholder).
  - `crates/services/src/cell/content/executor/mod.rs:299-324` (StartMinigame wired).
  - `crates/services/src/cell/cell_methods/minigame.rs:31+` (CM 20–34 `UNIMPLEMENTED`).
  - PRs #186, #244, #306, #609, #652, #659.
  - `docs/gameplay/minigame-system.md:39-60,209-217` (authoritative status and remaining work).
  - `db/resources` seeds reference only `'Livewire'` (8 hits), never Alignment or GoauldCrystals.
- Related/duplicates: #532 (SFS DoS hardening), #470 (CAT-K security), #269 (content-engine small hookups, possibly partly done by #652; that one is in another batch). #567 is crafting, not a minigame duplicate.

### Action text

**Comment:**
> Triage 2026-09-25: The framework half of this issue is done. The SFS host, tickets, expiry, abort-on-close, content `StartMinigame` with difficulty, the victory callback, the Livewire port (used by Castle 701) and the auto-win placeholders all shipped (#186, #244, #306, #652, #659). The earlier triage's "no actual minigame logic" line is out of date. I've rewritten the body to the remaining list in `docs/gameplay/minigame-system.md` § Remaining Work. No seeded content launches Alignment or GoauldCrystals yet, so those ports are nice-to-have.

**New title:** Minigames: port Alignment + GoauldCrystals, and implement the player-facing MinigamePlayer cell methods

#### New body

## What

The minigame framework is live: SmartFoxServer host, ticket/session registry, TTL expiry, abort-on-close, content-triggered start, victory callback, Livewire, and the six auto-win placeholders. Authoritative status: `docs/gameplay/minigame-system.md`. This issue tracks what's left.

## Remaining work

1. **Port Alignment and GoauldCrystals** from `deprecated/python/base/minigame/{Alignment,GoauldCrystals}.py`. Note that both Python files are themselves incomplete. The factory arms are commented out at `crates/services/src/minigame/games/mod.rs:13-15`.
2. **Don't silently auto-win unknown names.** An unrecognised game name falls back to the placeholder (`games/mod.rs:19-22`), which looks exactly like a real win. Log at error level and reject, or fail closed.
3. **Tech competency.** The ticket's tech-competency field is hardcoded to `1`. Read it from the player, along with abilities, intelligence and player_level.
4. **Player-initiated start.** `startMinigame` (24), `endCurrentMinigame` (25), the debug starts (20–23, GM-gated since #609) and `minigameStartCancel` (30) are stubs in `crates/services/src/cell/cell_methods/minigame.rs`.
5. **Helper call protocol** (28/29/31/32/33), including tip cash movement.
6. **Spectating** (26/27) and **NPC contacts** (34).

Items 1–3 are self-contained. Items 4–6 need RE of the client flow before scoping; split them into their own issues when picked up.

## Acceptance criteria

- [ ] Alignment and GoauldCrystals resolve to real `MinigameInstance` implementations, with unit tests for layout/seed, the win condition and the timeout, modelled on `games/livewire/tests.rs`.
- [ ] An unknown game name can no longer produce a victory.
- [ ] Tech competency comes from the player entity.
- [ ] `docs/gameplay/minigame-system.md` status table updated.

## Related

#532 (SFS DoS hardening), #470 (CAT-K security).

---

## #64 — GM commands: spawn, teleport, kill, giveitem, setlevel, shutdown

- Verdict: CLOSE (completed / superseded)
- Priority: P3
- Labels: no change
- Summary: The design this issue plans was dropped: a server-side `/cmd` chat intercept routing into the `crates/game` `CommandRegistry`. GM commands now go through the client's native `/` console (the SGWGmPlayer class flip in #518, plus GM gating from #512) and the GM-gated `.`-console (#523, P-series PRs #640, #642, #644, #749). Project memory records that a server-side `/`-chat parser should never be built, because the client consumes `/` commands. Most of the requested behavior shipped: `/gmspawnbycmd`, `.spawn`/`.savespawn`, `/gmgoto*` and `.goto`/`.gotoxyz`/`.summon`, `/gmkilltarget`, `/gmgiveitem`, and `/gmshowplayer`/`.players` for "who". Still open: `gmSetLevel` (152), already tracked in the ADAPT roadmap, and a server-shutdown broadcast, which is an operator/admin-API concern rather than a GM chat command. The `crates/game/src/commands/{gm_cmds,player_cmds}.rs` handlers are no longer `todo!()`. They are never-registered placeholder closures (no caller of `register_gm_commands`/`register_player_commands` outside the crate) and are dead code.
- Evidence:
  - PRs #518, #512, #523/#524, #640, #642, #644, #749.
  - `docs/commands.md:242-244,263,324,326,352` (✅ rows) and `:300` (`/gmsetlevel` ❌).
  - `docs/architecture/gm-cell-method-adapt-plan.md:141` (gmSetLevel 152, planned).
  - `crates/services/src/cell/console/registry/commands/{spawn,travel,...}.rs`.
  - `crates/game/src/commands/gm_cmds.rs:106-126` (TODO stubs, unregistered); `grep register_gm_commands` finds no external caller.
  - #37 was already closed as a duplicate (2026-05-31).
- Related/duplicates: #473 (CAT-N GM security audit, still open), #37 (closed).

### Action text

**Closing comment (reason: completed):**
> Closing as superseded. This issue planned a server-side `/cmd` chat intercept into the `crates/game` `CommandRegistry`. That design was replaced by the client's native GM console (SGWGmPlayer, #518, gated by #512) and the GM-gated `.`-console (#523; travel/spawn in #640, #642, #644, #749). Coverage of this issue's list, per `docs/commands.md`: spawn (`/gmspawnbycmd`, `.savespawn`), teleport (`/gmgoto`, `/gmgotoxyz`, `/gmgotolocation`, `.goto`, `.summon`), kill (`/gmkilltarget`), giveitem (`/gmgiveitem`), and who/inspection (`/gmshowplayer`, `.players`) are all done. `gmSetLevel` (152) is still open and is tracked in `docs/architecture/gm-cell-method-adapt-plan.md`. Server shutdown with a warning broadcast belongs on the operator/admin-API side, not a GM chat command. Follow-up: the unregistered placeholder handlers in `crates/game/src/commands/{gm_cmds,player_cmds}.rs` are dead code and can be deleted, the same way #699 removed `crates/game/src/social`.

---

## Batch summary

| # | verdict | priority | one-line reason |
|---|---|---|---|
| 572 | REWRITE | P3 | Phases 1–4 merged (#574/#578/#579/#581/#583) and owner-confirmed; only chatFriend/chatIgnore 0xC5/0xC6 and the ignore filter remain |
| 571 | KEEP (+owner Q) | P3 | Nothing on main (INT32 auctionLength decode still wrong); PR #586 has Phases 1–3 but has conflicted since June; invisible without the #587 client patch |
| 570 | KEEP | P3 | Accurate; only CM 88–90 stubs exist; pet-wire-formats.md INT32→INT8 correction still pending |
| 569 | REWRITE | P3 | Duel still stubs, but the witness-fanout prerequisite is mostly landed, D.3 is resolved (base idx 25/0xD9), and paths moved |
| 568 | KEEP (+owner Q) | P2 | Org still stubs on main (CM 17 WSTRING drop is real); guilds.rs pre-work note obsolete (#699); PR #584 conflicting |
| 587 | NEEDS-OWNER | P3 | Client patch; "server already sends onBMOpen" is false on main; 90-only patch gives an empty window; launcher vs client-launch/telemetry-DLL vehicle |
| 72 | REWRITE | P2 | Earlier triage was wrong: send/take/COD/return are all UNIMPLEMENTED stubs; the Send button no-ops; gap-analysis §24 wrong too |
| 69 | CLOSE (not planned) | P3 | Duplicate of #570 |
| 66 | REWRITE | P3 | SFS host, Livewire, placeholders and StartMinigame shipped; only Alignment/GoauldCrystals, tech competency and the player-facing CM 20–34 remain |
| 64 | CLOSE (completed) | P3 | Superseded by the native / console (#518) and .-console (#523 + P-series); only gmSetLevel remains (ADAPT roadmap); crates/game commands are dead code |

Cross-batch notes:

- Duplicate pairs: #69 → #570 (close #69). No #66 duplicate: #567 is crafting, not minigames. #72 has no duplicate but overlaps #466 (CAT-G) and feeds #571.
- Two stale feature PRs, #586 (BM) and #584 (org), have been `DIRTY` since June. Whoever handles open PRs should decide whether to rebase or re-cut them. #586 also edits `castle_cellblock_chains.sql`, which the Castle campaign rewrote.
- Doc fixes found along the way: `docs/gap-analysis.md` §24 (mail "Send" wrongly marked IM); `docs/protocol/sgwplayer-base-method-dispatch-table.md:42-43` (chatIgnore/chatFriend args missing aFlag/aPlayerNick); `docs/reverse-engineering/findings/pet-wire-formats.md:20-31` (INT32 should be INT8); `black-market-client-window-patch.md` says "server side is already real … all six are sent", which is only true on the PR #586 branch.
- `crates/game/src/commands/` is dead code, like the social module removed in #699. It's a cleanup candidate and could be folded into whatever issue tracks crates/game dead code.
