# SS-E1 Worknotes — Client Evidence (Mail, Chat, Duel)

> Type: reference. Audience: the social-systems campaign coordinator and downstream packet workers
> (SS-M1–SS-M4, SS-C1–SS-C4, SS-D1–SS-D3).
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [audit.md](../audit.md),
> [mail-wire-formats.md](../../../reverse-engineering/findings/mail-wire-formats.md),
> [chat-wire-formats.md](../../../reverse-engineering/findings/chat-wire-formats.md),
> [duel-wire-formats.md](../../../reverse-engineering/findings/duel-wire-formats.md),
> [duel-restoration.md](../../../reverse-engineering/findings/duel-restoration.md).

## Contract

- **Packet:** SS-E1 — Client evidence for the social-systems campaign (mail, chat, dueling).
  Documentation only.
- **Method:** Static Ghidra MCP against `SGW.exe`, plus the client Lua tree at
  `Stargate Worlds-QA/Working/SGWGame/Content/UI`. No debugger touched the live client.
- **Depends on:** none.
- **Source revision/base:** `afa0d940966300222ee7fc2cdf03f16e07e872d3` on branch `social/se1-re`
  (the `ss-e1` worktree), based on `origin/main`. The campaign ledger (`README.md`,
  `work-packets.md`, `audit.md`) was read from `origin/docs/social-systems-campaign` (unmerged),
  since it does not exist on `main` yet.
- **Owned paths (this packet only):**
  - `docs/reverse-engineering/findings/mail-wire-formats.md`
  - `docs/reverse-engineering/findings/chat-wire-formats.md`
  - `docs/reverse-engineering/findings/duel-wire-formats.md`
  - `docs/reverse-engineering/findings/duel-restoration.md`
  - `docs/reverse-engineering/findings/README.md`
  - `docs/analysis/social-systems/worknotes/ss-e1.md` (this file)
- **Read set:** `docs/analysis/social-systems/{README.md,audit.md,work-packets.md}` (on
  `origin/docs/social-systems-campaign`); the 2026-09-27 research reports at
  `%TEMP%\cimmeria-castle\kg-2026-09-27\{mail,chat,dueling}.md`; the ORG-E1 finding
  (`git show dc6bfdfd`, folded into `docs/analysis/organizations/README.md` D-ORG14 on `main`);
  the existing `docs/reverse-engineering/findings/{mail,chat,duel}-wire-formats.md` and
  `duel-restoration.md`; `entities/defs/{SGWMailManager.def,Communicator.def,SGWPlayer.def,
  SGWDuelMarker.def,enumerations.xml}`; `crates/cell-world/src/cell/space_manager/aoi.rs:190-220`
  (read-only, not edited — out of packet scope); client Lua under
  `Content/UI/Core/{GateMail,ChatWindow,Duel}/`.

## Verdicts (one line each)

| Q | Verdict | Confidence |
|---|---|---|
| M-Q1 (`sendMailResult` text) | CLOSED — byte-exact 0–7 + default switch recovered, confirms wire byte = enum declaration ordinal | HIGH |
| M-Q2 (`ItemId`, alias parsing) | CLOSED — `ItemId` is the item's own unique inventory id (not a type id); alias→`RecipientFlags` parsing is 100% client-side; several new client-side send-time rules recovered | HIGH |
| M-Q3 (`ExpiresHours`) | CLOSED — no wire field; client constant `0x2d0` = 720 hours = 30 days | HIGH (constant), MEDIUM (exact time-base) |
| M-Q4 (`MessageAttachment.id` join) | CLOSED — `id` is the mail id, used to join to the header; **correction**: `durability` is FLOAT, not INT32 | HIGH |
| M-Q5 (`ContainerId`/`SlotId`) | CLOSED — the shipped client sends uninitialized stack garbage for both fields, not even a reliable `-1,-1`; treat as always meaningless | HIGH |
| M-Q6 (unsolicited `onMailHeaderInfo`) | UNRESOLVED — not independently re-traced this session | — |
| M-Q7 (`ResetCategory`/`bArchive`) | CLOSED — client keeps two lists, routes each row by its own `flags & MAIL_Archive` bit, upserts by id; `ResetCategory` clears only the requested list | HIGH |
| C-Q1 (`/tell` channel byte) | Answered by ORG-E1 Q5 (tell = 10); not re-derived here per instruction | HIGH (cited, not new) |
| C-Q2 (`/gmshout`) | PARTIAL — native slash cmd → `sendGMShout` wiring confirmed, no Lua involved; exact arg-split not traced | MEDIUM |
| C-Q3 (chat max length) | CLOSED — no `MaxTextLength` on `Chat_Input`; uncapped client-side (negative evidence) | HIGH |
| C-Q4 (`onTellSent` rendering, AFK/DND) | UNRESOLVED — relied on existing research report, not independently re-RE'd | — |
| D-Q1 (duel timer duration) | CLOSED — no client constant; countdown length is entirely server-driven | HIGH |
| D-Q2 (range constants) | UNRESOLVED — not found this session | — |
| D-Q3 (death vs. duel-end branch) | UNRESOLVED — moot: D-SS20 is owner-approved regardless | — |
| D-Q4 (`GENERICPROPERTY_PvPFlag`) | PARTIAL — found a dedicated `pvpFlag` CELL_PUBLIC property + `setPvPFlag`/`startPvPTimer`; likely the real mechanism, client consumption unconfirmed | MEDIUM-HIGH |
| **D-Q5 (blocking)** | **CLOSED** — see below | **HIGH** |
| D-Q6 (moniker rendering) | UNRESOLVED — `onErrorCode` ruled out; feedback-text-over-existing-channel is the best guess, not confirmed | — |

## D-Q5 in full (blocking for SS-D2)

`onDuelEntitiesSet` [151], `Remove` [152], `Clear` [153] are all bound to **`GamePlayer`** methods
(found via the CME register-fn → `TypedEmitInfo` → `MemberCallback` chain, not the register
addresses named in the packet, which are only name strings):

- **151 (`FUN_00e07250`)** decodes `ARRAY<INT32> aEntityList` and **inserts** each id into an
  ordered set at `GamePlayer+0x16c` (`FUN_00c6bd20`, an MSVC `std::map`/`std::set` insert), then
  forces an interaction-flags recompute on that entity (`GameEntity__unknown_00e6e330(entity, 1)`).
- **152 (`FUN_00e07440`)** decodes one `INT32 aEntityId` and **erases** it from the *same* set
  (`FUN_00e083a0`), then does the identical recompute call.
- **153 (`FUN_00e071c0`)** swaps the whole container for an empty one (O(1) clear), then does the
  identical find-entity + recompute cleanup for everything that was in it.

**The interactability computation (`GameEntity__unknown_00e6e330` → `FUN_00e719d0`) never reads
this set.** It is a generic per-target-*template* + spatial-range lookup, keyed by the target's
own template/type id, entirely independent of duel bookkeeping. Set membership does nothing;
only the *act* of insert/erase/clear forces a (harmless, self-correcting) recompute.

**What this means for the server:**

- `crates/cell-world/src/cell/space_manager/aoi.rs:203-211`'s comment has the direction backwards
  — it says 152 "adds" to the set; the decompile shows it **erases**. AoI's use of 152 on
  interactable NPCs works only because the forced recompute re-derives the NPC's own (already
  correct) interactable flags — it has nothing to do with duel-entity-set membership. **Recommend
  fixing this comment in the same PR that next touches that block** (SS-D2, most likely).
- Since AoI never sends 151, and 152 on an id never in the set is a harmless no-op erase, the
  local player's real duel-entity set only ever contains what SS-D2 itself puts there.
  **Sending 151 with the two duelist entity ids at duel engage, and 153 at duel end, is SAFE** —
  it cannot make any NPC uninteractable, and it cannot leave stray NPC ids for 153 to mishandle.
  **D-SS25's blanket caution can be lifted for 151/153**; SS-D2 may implement issue #569's
  original plan as written. Leave AoI's existing use of 152 alone.

Full addresses and the intermediate call chain are in `duel-wire-formats.md`'s SS-E1 section.

## Notable out-of-scope findings worth flagging

- **D-Q4**: `SGWPlayer.def` declares a real `pvpFlag` CELL_PUBLIC property (auto-synced via the
  standard entity-property-change path) plus internal `setPvPFlag`/`startPvPTimer` cell methods —
  not previously documented anywhere. This may mean SS-D2 should sync `pvpFlag` the way any other
  def property is synced, rather than building on `map_loaded.rs:334`'s `GENERICPROPERTY_PvPFlag`/
  `onEntityProperty` assumption. I did not confirm which mechanism the client actually reacts to
  — recommend SS-D2 spend a short Ghidra pass on this before committing to the fan-out shape.
- **M-Q5**: the shipped client's `takeItemFromMailMessage` call carries uninitialized stack
  garbage for `ContainerId`/`SlotId`, not a reliable sentinel. This slightly strengthens
  CAT-G-03's existing caution — SS-M3 should never branch on these fields at all (not even to
  detect `-1,-1`), always placing the item in the caller's first free main-container slot.
- **M-Q2**: the client's own multi-recipient/attachment/alias validation rules were fully
  recovered (51-token cap, vault-can-carry-an-item-but-never-cash rule, COD requires item+cash
  both ways). These are all HIGH-confidence, byte-exact client behaviors — useful acceptance-test
  material for SS-M1/SS-M2 beyond what the ledger's decisions already specify.

## Commands run

Documentation only; no compiling commands were run. All work was Ghidra MCP calls against the
already-open `SGW.exe` program, plus `grep`/`find` against the client Lua tree and this repo's
`entities/defs/`. `pwsh tools/lint-md.ps1` was not run this session (Windows PowerShell tool,
not exercised — the CRLF line-ending discipline was checked manually with `file`/`cat -A` after
every edit instead, since the `Edit` tool silently rewrites touched lines to LF; each edited file
was re-normalized to CRLF with `sed` immediately afterward and re-verified).

## Known gaps

- M-Q6, C-Q4, D-Q2, D-Q3, D-Q6 are UNRESOLVED (see table above) — either genuinely not found in
  the time available, or (D-Q3) mooted by an owner decision. None of them block a wave-1 packet;
  D-Q6 should be revisited before SS-D1/D2/D3 pick a wire mechanism for the duel feedback strings
  if "send as literal feedback text" turns out to be wrong.
- C-Q2's exact `/gmshout` argument-split heuristic (how "global" vs. space-scoped is chosen from
  typed text) was not traced; the def's two-argument shape is already pinned and sufficient for
  SS-C2 to decode correctly regardless.
- D-Q4's client-side consumption of `pvpFlag` (targeting/nameplate/cursor) was not traced.

## Integration edits the coordinator/downstream packets should make

1. **SS-D2**: implement `onDuelEntitiesSet`[151]/`Clear`[153] as issue #569 originally planned
   (D-SS25's blanket hold is no longer warranted); fix `aoi.rs:203-211`'s comment in the same PR;
   spend a short pass confirming whether `pvpFlag` (CELL_PUBLIC property) or
   `GENERICPROPERTY_PvPFlag`/`onEntityProperty` is what the client actually reacts to before
   finalizing the PvP-flag fan-out.
2. **SS-M2**: key item escrow on the wire `ItemId` as an inventory *instance* id (matches
   D-SS08's existing assumption — no change needed there, just confirms it).
3. **SS-M3**: never interpret `ContainerId`/`SlotId` on `takeItemFromMailMessage`; always place
   into the caller's first free main-container slot.
4. **SS-M1/SS-M4**: `MessageAttachment.durability` is FLOAT, not INT32 — already corrected in
   `mail-wire-formats.md`; use the corrected layout when SS-M2 first emits a non-empty
   `MessageAttachments` array. Mail TTL should be exactly 30 days (`sent_time + 30d`), not a
   different fallback.
5. **SS-D1/D2/D3**: send duel feedback strings (872–880) as literal `onPlayerCommunication`
   `CHAN_feedback` text, not a moniker-id wire path, pending stronger evidence on D-Q6.
