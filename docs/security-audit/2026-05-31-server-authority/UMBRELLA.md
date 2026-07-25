# Server-authority audit — full sweep [umbrella]

Date: 2026-05-31
Worktree branch: `worktree-server-authority-audit`

> Tracking record for an exhaustive server-authority / anti-cheat / anti-replay
> audit of every player-facing wire surface in the Cimmeria SGW server emulator.

---

> **Status re-verification — 2026-07-25.** The findings files remain the
> 2026-05-31 point-in-time snapshot; per-finding status banners have been
> added where code has moved. Summary of the P0 bundle:
>
> | P0 item | Status |
> |---|---|
> | CAT-N-03 — `access_level` in cell dispatch | **Resolved and merged** (#475) — with two known gate gaps, below |
> | CAT-A-01 — SOAP over TLS (#476) | **Partial** — TLS listener exists but is opt-in; plain HTTP still served |
> | CAT-A-02 — per-packet IV (#477) | **Partial** — Mercury v2 has it, but v1 is still the default and v2 is untested against a live client |
> | CAT-A-03 — inbound dedup (#477) | **Open, unchanged** |
> | CAT-B-05 — position replay sequence (#477) | **Open, unchanged** |
> | CAT-B-01 + CAT-B-09 — movement validation | **Resolved and merged** (#478/#522) — speed layer is warn-only by design |
> | CAT-J-01 — dialog-state authority | **Not re-verified in this pass** |
>
> Two gaps in the #475 GM gate that the "resolved" status above does **not**
> cover: `requires_gm` allow-lists in-range indices `2 | 3 | 6 | 92`, which
> misses `RESET_MY_ABILITIES` (CM 72, CAT-N-02) and the four MinigamePlayer
> `debug*` methods (CM 20–23, CAT-K-02). Both are stubs today, so both are
> latent — but the systemic protection #475 provides stops at index 109 plus
> those four named indices.
>
> Work verified as landed on a branch but **not on `origin/main`**: the
> Black Market implementation (CAT-I, `feat/571-black-market-phase1`), the
> auth XML escaping fix (#447, PR #604), and the `requestAmmoChange`
> fail-closed fix (#448, PR #602).

## Scope + methodology

All findings are evidence-backed against authoritative sources only:

- **Ghidra MCP** decompilation of the live SGW.exe client (`/SGW.exe`, 173,223 functions, 634,156 symbols, image base 0x00400000) — the actual bytes the client constructs and sends.
- **`../sgw/` client install** at `C:\Users\Steve\source\projects\sgw\Stargate Worlds-QA\` — game data files (XSD schemas under `Common/xml/SGWShared/CookedData/`), client-generated logs.
- **Compiled-binary RTTI evidence** — every `Event_NetOut_*` client message class confirmed via Ghidra string addresses (cited inline in each finding).

Deliberately **NOT** treated as sources of truth:

- The deprecated Python in `deprecated/python/` (referenced only where the wire shape can be cross-checked, never as authority).
- The Rust server code in `crates/` (the *thing being audited*, cited only as fix-author cross-reference).
- The PostgreSQL schema in `db/`.
- Any markdown docs in `docs/`.

Methodology: each category was audited by a dedicated `server-authority-enforcer` agent with read-only access. Each agent (a) extracted the inbound `Event_NetOut_*` wire surface from Ghidra, (b) traced the matching server handler in Rust, (c) flagged trust-boundary violations where the server reads client-supplied state and acts on it without server-side validation, and (d) filed only "demonstrable" or "likely-exploitable theoretical" findings (no pure speculation).

## Stats

- **Categories audited**: 15 (A through O)
- **Total findings**: 177
- **Severity breakdown**:
  - **10 Critical** (Demonstrable + Latent-Critical-when-impl combined)
  - **~35 High**
  - **~50 Medium**
  - **~55 Low / Latent-when-impl**

## Six systemic cross-cutting patterns

These recur across multiple categories and warrant focused attention before
the per-finding work begins.

### 1. `access_level` is not plumbed into cell-method dispatch

**Lead finding**: CAT-N-03 (Critical, systemic).

The `access_level` (GM / admin / player) lives on `ConnectedClientState` in the
base layer. The cell-method dispatcher in `crates/services/src/cell/dispatch/router.rs`
has no access to it. Every future `gm*` handler added to the cell layer will be
unauthenticated-by-default. This blocks every GM-gating fix in CAT-N until it's
addressed structurally.

**Related findings**: CAT-L-06 (SendGMShout), CAT-N-01/02 (WORLD_INSTANCE_RESET / RESET_MY_ABILITIES exposed on regular SGWPlayer), CAT-O-04 (onWorldInstanceReset exposed on player).

### 2. Auth-channel crypto + anti-replay foundation is wide open

**Lead findings**: CAT-A-01 (Critical, plaintext SOAP), CAT-A-02 (High, zero-IV AES-CBC reuse), CAT-A-03 (High, no inbound replay/dedup on encrypted datagrams), CAT-A-05 (High, ticket no source-IP binding), CAT-B-05 (Medium, no anti-replay on position updates).

The encrypted channel uses a fixed zero IV reused per packet (passive plaintext-structure leak) and has no inbound sequence/dedup wired on the game-packet path. **Every other anti-cheat finding in this audit assumes a secure channel that isn't actually secure.** Fix these *first*: per-handler validation can be bypassed by replaying captured packets if this layer doesn't hold.

### 3. Wire surface live, handler stubbed — "future implementer ships the dupe"

**Categories most affected**: CAT-G (mail), CAT-H (trade), CAT-I (black market), CAT-F (crafting), CAT-J (mission), CAT-K (minigame), CAT-L (chat), CAT-M (org/duel).

Consistent pattern: the cell-method dispatcher returns `true` (handled) and logs
`UNIMPLEMENTED:` while doing nothing. The wire surface is fully exposed, the validation
contract is undocumented, and the next contributor to fill in the handler body will inherit
all the trust violations by default. The audit findings serve as the *spec* for what
guard-rails the implementation must include.

Most acute for:
- **Trade (CAT-H)** — 4 Critical-latent (TOCTOU, atomicity, disconnect, concurrent inventory mutation)
- **Auction (CAT-I)** — 5 Critical-post-impl (no current-bid check, no atomicity, no expiry sweep)
- **Mail (CAT-G)** — 5 High-latent (no validation contract; double-take race surface)
- **Crafting (CAT-F)** — 3 Critical-latent (`craft`, `research`/`reverseEngineer`, `alloying`)
- **Org/Duel (CAT-M)** — 1 Critical real (guild bank dupe via signed-int) + 7 High

### 4. Position validation is absent on the inbound write path

**Lead finding**: CAT-B-01 (Critical).

`AVATAR_UPDATE_EXPLICIT` (system message 0x03) writes the client-supplied position to
the cell entity with **zero validation** — no navmesh check (the helper `is_position_valid`
exists but is unwired), no speed check, no Z-axis check, no cross-space sanity. Every
per-tick movement is a free teleport. Downstream AoI, region triggers, quest gates,
threat radius, navmesh distance — every higher-level system reads from this corrupt
position.

**Related findings**: CAT-B-09 (navmesh helper exists but unwired), CAT-B-06 (cross-space spaceId discarded).

### 5. Content-engine chains can be reached by forging a single packet

**Lead finding**: CAT-J-01 (Critical).

`DIALOG_BUTTON_CHOICE` fires arbitrary content chains with no check that the dialog is
actually open. Forge one `(dialog_id, button_id)` packet → reach chain actions like
`GrantXP / GrantItem / Teleport / AcceptMission / CompleteMission`. Combined with CAT-J-04
(no mission prereq validation on accept), this is the universal content-bypass exploit.

### 6. Server-must-hold-pending-state — the response-without-correlation anti-pattern

**Findings**: CAT-J-01 (DialogButtonChoice), CAT-M-13 (sendDuelResponse), CAT-M-16 (pvpOrganizationLeaveResponse), CAT-M-17 (strikeTeamResponse), CAT-M-18 (organizationInviteResponse), CAT-K-04/05 (minigame cancel/spectate), and the mail invite-response analogues (CAT-G when implemented).

Pattern: a response message carries NO correlation id (no challenger id, no invite id, no pending-prompt id, etc.). The server MUST hold pending-state per-session for these, and reject responses that don't match. Naïve handler implementations that just decode the payload and act on it will be exploitable.

## Per-category status

| Category | Findings | Critical | High | Medium | Low | Findings file |
|---|---:|---:|---:|---:|---:|---|
| CAT-A — Auth / Session / Character lifecycle | 14 | 1 | 4 | 3 | 6 | [CAT-A-auth.md](findings/CAT-A-auth.md) |
| CAT-B — Movement / Teleport / Position | 10 | 1 | 3 | 3 | 3 | [CAT-B-movement.md](findings/CAT-B-movement.md) |
| CAT-C — Combat / Abilities | 15 | 0 | 4 | 4 | 7 | [CAT-C-combat-abilities.md](findings/CAT-C-combat-abilities.md) |
| CAT-D — Inventory / Items | 9 | 0 | 3 | 4 | 2 | [CAT-D-inventory.md](findings/CAT-D-inventory.md) |
| CAT-E — Vendor | 6 | 0 | 1 | 1 | 4 | [CAT-E-vendor.md](findings/CAT-E-vendor.md) |
| CAT-F — Crafting / R&D | 8 | 3 (latent) | 1 (latent) | 2 | 2 | [CAT-F-crafting.md](findings/CAT-F-crafting.md) |
| CAT-G — Mail | 8 | 0 | 5 (latent) | 1 | 2 | [CAT-G-mail.md](findings/CAT-G-mail.md) |
| CAT-H — Trade (P2P) | 10 | 4 (latent) | 5 (latent) | 1 (latent) | 0 | [CAT-H-trade.md](findings/CAT-H-trade.md) |
| CAT-I — Black Market | 6 | 5 (post-impl) | 1 | 0 | 0 | [CAT-I-black-market.md](findings/CAT-I-black-market.md) |
| CAT-J — Mission / Dialog / Interaction | 9 | 1 | 2 | 2 | 4 | [CAT-J-mission-dialog.md](findings/CAT-J-mission-dialog.md) |
| CAT-K — Minigame | 9 | 1 | 1 | 4 | 3 | [CAT-K-minigame.md](findings/CAT-K-minigame.md) |
| CAT-L — Chat / Contact | 9 | 0 | 3 (latent) | 3 | 3 | [CAT-L-chat-contact.md](findings/CAT-L-chat-contact.md) |
| CAT-M — Organization / Squad / Duel | 18 | 1 | 7 | 8 | 2 | [CAT-M-org-squad-duel.md](findings/CAT-M-org-squad-duel.md) |
| CAT-N — GM / Debug / Cheat commands | 40 | 2 (incl. systemic) | 8 | 12 | 18 | [CAT-N-gm-commands.md](findings/CAT-N-gm-commands.md) |
| CAT-O — World / Space / Gate / Ring | 6 | 0 | 3 | 2 | 1 | [CAT-O-world-space-gate.md](findings/CAT-O-world-space-gate.md) |
| **Totals** | **177** | **~18** | **~46** | **~46** | **~57** | |

## P0 / P1 / P2 triage

### P0 — Foundational; fix before anything else

These are systemic. Every other audit finding rides on top of them.

1. ~~**CAT-N-03** — Plumb `access_level` from base session state into cell-method dispatch context.~~ **Done (#475)**, modulo the CM 72 / CM 20–23 allow-list gaps noted at the top of this file. (Critical, systemic, blocked every GM-gating fix in CAT-N.)
2. **CAT-A-01** — Move SOAP auth to TLS. **Partially done (#476)**: the listener exists but is opt-in and plain HTTP is still served. (Critical, exposes credentials + session keys.)
3. **CAT-A-02 + CAT-A-03 + CAT-B-05** (crypto-and-replay bundle) — Per-packet IV derived from sequence number, wire `Channel::receive_packet` dedup into the inbound encrypted path, anti-replay sequence on position updates. **Only the IV half is done, and only under Mercury v2, which is not the default.** The dedup and position-sequence halves are untouched. (Without these, every per-handler fix is replay-bypassable.)
4. ~~**CAT-B-01 + CAT-B-09** — Server-side speed / navmesh / Z-axis validation on the `AVATAR_UPDATE_EXPLICIT` write path.~~ **Done (#478)** — bounds, navmesh, and teleport layers hard-reject and snap back via `BASEMSG_FORCED_POSITION`. Note the **speed layer is warn-only** pending tolerance calibration, so the speed-hack half of the original finding is still live.
5. **CAT-J-01** — Track open-dialog state server-side; reject `DIALOG_BUTTON_CHOICE` unless caller has that dialog open with that button. (Without this, every content chain can be triggered by a single forged packet.)

### P1 — Critical individual exploits (currently demonstrable)

These are localized single-finding fixes.

- **CAT-C-01** — `respawn`/`callForAid` heal non-dead player to full → effective combat-state reset + free heal.
- **CAT-C-02** — `callForAid(respawner_id)` accepts any id → arbitrary teleport via respawner table.
- **CAT-C-03** — `useAbility` has no faction/friendly-fire filter → PvP damage open + damage to vendors/quest-givers.
- **CAT-C-04** — `useAbility` has no LOS / navmesh / AoI-membership check → shoot through walls.
- **CAT-D-01** — Bandolier ammo TOCTOU keyed on `type_id` → same-type-swap ammo overwrite.
- **CAT-D-02 + CAT-D-03** — `lootItem` no range/state recheck + no loot reservation → vacuum loot from 100 units.
- **CAT-E-01** — Vendor REPAIR/RECHARGE wire-shape inversion: omit trailing 4 bytes → free repair/recharge.
- **CAT-K-01** — Minigame `PlaceholderGame` instant-victory → bypass every mission with a minigame gate.
- **CAT-M-09** — `organizationTransferCash` signed `INT32 aCash` → negative-amount sign-flip = guild-bank dupe.
- **CAT-B-02 / B-03 / B-04** — onDialGate / ring transport / region trigger trust client claim with no source-position proof (combine with P0 #4).
- **CAT-O-01** — `onDialGate` ignores `knownStargateAddresses` → any char dials any gate in catalog.
- **CAT-O-03** — `triggerClientHintedGenericRegion` has no rate-limit → chain-fire region triggers across mission sequences at packet rate.

### P2 — Guard-rail tests for stubbed handlers

These are *latent* findings: dispatcher returns `true` while handler is stubbed.
File now so the spec exists before the implementer ships the dupe.

- **All of CAT-G (mail)** — sendMail, takeCash, takeItem, payCOD, returnMail.
- **All of CAT-H (trade)** — full state machine, item lock, atomicity, disconnect handling.
- **All of CAT-I (black market)** — bid/cancel/expiry/COD-to-seller cascade.
- **CAT-F-03/04/05** (crafting) — material consumption atomicity, server-side outcome roll, alloying tier check.
- **CAT-J-05/06** (mission) — ChosenRewards (offered-reward-set authority), ShareMission (ownership + group membership + recipient consent).
- **CAT-K-02/04/05/06** (minigame) — debug commands GM-gating, cancel-actor auth, spectate perception, contact request gating.
- **CAT-L-04/05/06/08** (chat) — contact list ownership, BroadcastMinimapPing org-membership, SendGMShout access_level, channel-op session-side bit tracking.
- **CAT-M (whole category)** — rank-comparison invariants, transferCash positivity + atomic, response-correlation per session.
- **CAT-O-04** (world-instance-reset on player entity).
- **All of CAT-N** — every `gm*` command needs access_level + bounds checks at implementation time.

## Live-debugger triage

A subset of findings flag "would benefit from x64dbg trace" for confirmation under a real client connection. These are noted per-finding (e.g. CAT-D-01 bandolier race window timing, CAT-D-09 wire-width confirmation). They can be batched into a debugger session when the user is available — issues will be tagged so they're surfaced as a group.

## Specialist consults

Each finding's "Suggested remediation" line tags the appropriate specialist agent at fix time:

- `combat-systems-advisor` — combat math, threat lifecycle, BSF_InCombat semantics, focus restoration
- `items-systems-advisor` — bandolier state, stack semantics, container ACL, vendor state machine, material consumption
- `movement-physics-advisor` — speed validation primitive, navmesh containment, LOS raycast
- `movement-teleport-advisor` — BASEMSG_FORCED_POSITION + AoI refresh, ring/DHD/gate canonical primitives
- `social-systems-engineer` — mail/trade/auction state machines, item locks, guild ranks, duel handshake
- `mission-systems-advisor` — chain pipeline, mission prereq model, reward selection authority
- `minigame-systems-advisor` — SmartFoxServer 1.x protocol, ticket exchange, per-game result schema
- `network-security-auth` — TLS migration, IV derivation, replay window wiring, dev-mode bypass gate

## How to triage from here

1. Land the **P0 bundle** as a coordinated change set. Per-handler fixes from P1/P2 are bypassable without P0.
2. **P1 individual exploits** as separate small PRs — each has localized fixes.
3. **P2 guard-rails**: file as tests/specs against the stub handlers so the implementing PR has a failing test that proves the trust violation is closed.
4. Use the per-category child issues as the working list — each lists its findings with checkboxes.
