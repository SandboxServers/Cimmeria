# Batch sec-a — security-audit umbrella + CAT-A..CAT-I (research only)

Code of record: `main-ro` @ 059d6038 (2026-09-25). All `file:line` refs are relative to
`crates/services/src/` unless they start with another top-level dir. The audit's own
finding files are ON MAIN at `docs/security-audit/2026-05-31-server-authority/` (with a
`docs/security-audit/README.md` index and a status-banner convention). The issues still
link to the `worktree-server-authority-audit` branch; every rewrite below repoints to main.

---

## #460 — [security-audit] CAT-A — Auth / Session / Character lifecycle (14 findings)

- Verdict: REWRITE
- Priority: P2 (A-01/A-02/A-03 are real exposure on a public colo, but each is owned by its own P0 issue #476/#477; what is left here is hardening)
- Labels: no change (`security`)
- Summary: Most of the 14 findings are still open. Six have moved. TLS (A-01) and Mercury v2 crypto (A-02) landed but are **opt-in**. The default deployment is unchanged: plain HTTP, and v1 with a zero IV. A-05 (IP binding) landed as warn-only in PR #738. A-07 is fixed on the argon2id path but still uses a variable-time compare on the legacy SHA-1 path. A-08 dev mode is now off by default. A-10's name validator checks format only and has no reserved-word list. A-03/A-09 (no inbound dedup), A-04 (TTL not checked when the ticket is consumed), A-11..A-14 are unchanged. The issue body predates all of this and links to a branch, so it needs a rewritten checklist.
- Evidence:
  - A-01 partial: TLS listener starts only when `auth_tls_cert_path`+`auth_tls_key_path` are set (`auth/service.rs:35-77`). A plaintext password over HTTP is rejected (`auth/credentials.rs:52,76`). The colo compose files set no TLS cert (only `docs/operations/container.md` mentions it). PRs #566, #577. Tracked by #476 and #434.
  - A-02 partial: `EncryptionVersion::V1` is `#[default]`, "the only version unpatched clients accept" (`crates/mercury/src/encryption/mod.rs:128-137`). v2 has not been validated against a live client (memory `project_confirmed_working_2026_06`). PRs #566, #575. Tracked by #477.
  - A-03/A-09 open: `base/connect_loop/encrypted/mod.rs:63-137` goes decrypt → parse → ACK queue → `process_acks`. `Channel::receive_packet` (`crates/mercury/src/channel/channel_core.rs:361`) has no production caller. Tracked by #477.
  - A-04 open: `base/login/mod.rs:46-51` is a bare `map.remove(ticket)` with no `created.elapsed()` check. The only gate is the reaper at `auth/service.rs:190`, `TICKET_TTL` 30s at `auth/mod.rs:49`.
  - A-05 partial: PR #738 (closes #442) persists the client IP. On mismatch it only WARNs (`reason="ticket_ip_mismatch"`, `base/login/mod.rs:63-78`; `session_ip_mismatch` at `auth/handlers.rs:262`). Hard reject is deliberately deferred.
  - A-06 open: nothing in `auth/` does rate-limiting or lockout (grep for `rate_limit|lockout|governor` finds nothing there).
  - A-07 partial: argon2id verification is constant-time (`auth/credentials.rs:256-258`). The legacy path uses `!=`/`==` (`auth/credentials.rs:186,195`) until the account is migrated on login (`:198-202`).
  - A-08 mitigated: `developer_mode` defaults to false (`crates/common/src/config.rs:160`, test at `:238`). It can only be turned on explicitly (`DEVELOPER_MODE` env, `crates/server/src/main.rs:407-408`). When on with no DB it still grants `(1, 99)` (`auth/handlers.rs:172-174`). That is an intended dev feature. A startup WARN would be enough.
  - A-10 partial: `base/character_create.rs:549-569` checks length 3-20 and ASCII/space/hyphen/apostrophe, but has no reserved-word or impersonation list. "Admin" and "GM Bob" pass.
  - A-11 open, low value: `base/connect_loop/account_arms.rs:152` only logs. The finding's own reasoning undercuts it: the version is client-asserted, so a modded client just lies. Recommend closing A-11 as won't-fix.
  - A-12 open: 0x0B is framed at `base/connect_loop/encrypted/mod.rs:514` and never inspected.
  - A-13 open: ticket removal (`base/login/mod.rs:46-51`) and duplicate-account eviction (`:90-110`) run under separate locks.
  - A-14 open: `base/connect_loop/encrypted/mod.rs:150-161` skips 0x01 ("ignored"). Tracked by #294.
  - Credential logging (the note under A-01): closed by PR #698 (#440).
  - The status banner at the top of `docs/security-audit/.../findings/CAT-A-auth.md` (2026-07-25) agrees with this, except that it predates #738 (A-05) and #698 (#440).
- Related/duplicates: #476 (A-01), #477 (A-02/03/09), #294 (A-14), #434 (encryption epic), #442 (closed, A-05), #440 (closed), #447 (auth XML escaping, not an audit finding)

### Action text

**Comment:**
> Re-verified against `main` @ 059d6038 (2026-09-25). Since the audit, TLS (#566/#577) and Mercury v2 crypto (#566/#575) have landed, but both are opt-in. v1 plus plain HTTP is still what a stock client and the colo run. IP binding landed as warn-only (#738). Credential logging was fixed (#698). Dev mode is now off by default. The body below replaces the original checklist with the current state of each item. Items that belong to a dedicated issue point there, so this issue only tracks the small leftovers. The audit files now live on `main` under `docs/security-audit/2026-05-31-server-authority/`, not on the audit branch.

#### New body

```markdown
Part of the server-authority audit (#459). Full findings: [`docs/security-audit/2026-05-31-server-authority/findings/CAT-A-auth.md`](../blob/main/docs/security-audit/2026-05-31-server-authority/findings/CAT-A-auth.md) (point-in-time; status banners on top).

## Problem
The auth handshake (SOAP Phase 1/2 → Mercury Phase 3 `baseAppLogin`) and the encrypted game channel still have several trust gaps. Ownership, CharDef derivation and access_level plumbing are sound. The remaining gaps are transport confidentiality, replay, ticket lifetime and brute force.

## Status (re-verified on main @ 059d6038, 2026-09-25)

Owned by a dedicated issue. Check them off here when that issue closes.
- [ ] **CAT-A-01** (Critical) Plaintext SOAP. TLS listener exists but is opt-in (`auth/service.rs:35-77`). Plain HTTP stays up, and the colo sets no cert. → #476 / #434
- [ ] **CAT-A-02** (High) Zero-IV AES-CBC. Mercury v2 (random IV + HMAC-SHA256) exists, but v1 is the default and the only version a stock client speaks (`crates/mercury/src/encryption/mod.rs:128-137`). v2 has not been validated live. → #477 / #434
- [ ] **CAT-A-03 / CAT-A-09** (High/Med) No inbound replay dedup. `Channel::receive_packet` has no production caller. `handle_encrypted_datagram` goes decrypt → dispatch (`base/connect_loop/encrypted/mod.rs:63-137`). → #477
- [ ] **CAT-A-14** (Low) `authenticate` 0x01 is skipped (`base/connect_loop/encrypted/mod.rs:150-161`). → #294

Tracked here:
- [ ] **CAT-A-04** (Med) Ticket TTL is not checked when the ticket is consumed. `base/login/mod.rs:46-51` removes it without checking `created.elapsed() < TICKET_TTL`. Only the 10s reaper (`auth/service.rs:190`) enforces the TTL.
- [ ] **CAT-A-05** (High → Med) IP binding is warn-only (#738, `reason=ticket_ip_mismatch` / `session_ip_mismatch`). Harden it to reject once SigNoz shows the NAT false-positive rate.
- [ ] **CAT-A-06** (Med) No rate limit or lockout on Phase 1 credential checks (`auth/handlers.rs`).
- [ ] **CAT-A-07** (Low) Legacy SHA-1 compare is variable-time (`auth/credentials.rs:186,195`). argon2id accounts are fine. Use a constant-time compare (`subtle`) until every account is migrated.
- [ ] **CAT-A-08** (High → Low) Dev-mode any-credential login with access 99 (`auth/handlers.rs:172-174`). Now off by default (`crates/common/src/config.rs:160`). Remaining: a loud startup WARN when it is on.
- [ ] **CAT-A-10** (Low) No reserved-word filter in `validate_character_name` (`base/character_create.rs:549-569`). "Admin" and "GM Bob" are accepted.
- [ ] **CAT-A-12** (Low) `restoreClientAck` 0x0B is framed but never checked against a pending restore (`base/connect_loop/encrypted/mod.rs:514`).
- [ ] **CAT-A-13** (Low) Ticket consumption and duplicate-account eviction are not atomic (`base/login/mod.rs:46-51` vs `:90-110`).

Resolved / dropped:
- [x] Credential values in logs, fixed by #698 (#440).
- [x] **CAT-A-11** dropped. A minimum client version the client reports about itself is no control, because a modded client lies. `PROTOCOL_DIGEST` at Phase 1 is the only meaningful gate.

## Acceptance criteria
- A-04: consuming a ticket older than `TICKET_TTL` logs `reason="ticket_expired"` and creates no session, even before the reaper runs.
- A-06: N failed Phase 1 attempts per account/IP inside a window get a `login_error`, a WARN with `reason="auth_rate_limited"`, and the password is not checked.
- A-07: the legacy compare goes through a constant-time primitive.
- A-10: a reserved-name list (`admin`, `gm`, `moderator`, `cimmeria`, the SGW staff prefixes) is rejected with the existing name-invalid error.
- A-12/A-13: a stray 0x0B logs at WARN. The ticket-consume + evict sequence holds one lock or has an equivalent guard.

## Test type
Unit (name validator, TTL predicate, constant-time wrapper), live-DB (rate-limit window), negative-log (`LogCapture`) for the expired-ticket and rate-limit seams.

## Docs to update
`docs/security-audit/.../CAT-A-auth.md` status banner; `docs/architecture/negative-logging-convention.md` if new reasons are added.

## Client impact
Free for A-04/06/07/08/10/12/13. A-01/A-02 need client-side work tracked in #434/#476/#477.

## Domain advisor
`network-security-auth`

## Needs a human for
Nothing for the local items. Deciding when A-05 moves from warn-only to reject needs the owner and colo SigNoz data.
```

---

## #461 — [security-audit] CAT-B — Movement / Teleport / Position (10 findings)

- Verdict: REWRITE
- Priority: P2
- Labels: no change
- Summary: The Critical finding, B-01 (free teleport through 0x03), is fixed. So are B-06 and B-09, all by PR #522 (#478). B-04 (forged region trigger) is also fixed: `resolve_hinted_region` now checks the id is non-negative, the region is in the caller's world, and the server-known position is inside it. B-02 is mostly mitigated. The dial is checked against the player's address book (CAT-O-01), and travel happens when the player walks into the gate volume, not when they dial. B-03 is partly mitigated. The ring FSM only teleports players who are on the pad, but `setRingTransporterDestination` does not check that the *caller* is on or near the source pad, so anyone can remotely trigger any ring. B-05 (position replay) belongs to #477. B-07, B-08 (Unstuck stub) and B-10 (0x02/04/05 dropped) are unchanged.
- Evidence:
  - B-01/06/09: PR #522 (closes #478). Banners in `docs/security-audit/.../CAT-B-movement.md:42,348,498`. Follow-up fix for jumps in PR #643.
  - B-02: `cell/gate_travel/mod.rs:72-140` still ignores `_source_address_id` but refuses unknown addresses (`cell/gate_travel/address_book.rs:50-120`). Travel happens on `REGION_FLAG_STARGATE` entry (`cell/cell_methods/player/world/mod.rs:112-117`), which is containment-gated. What remains: on a world with no gate volume, dialing travels immediately from anywhere (doc comment at `gate_travel/mod.rs:57-60`).
  - B-03: `cell/ring_transport/runtime/entry.rs:94-202` validates that the source→destination pairing, the mission gate and the destination state are all correct. It never checks that `entity_id` is registered on `source_region_id`'s pad or is in that ring's world. `enter_send_wait` adds the caller to `reserved_by` (`transporter/source.rs:22-29`). Only players standing on the pad travel (`source.rs:47`), so a remote caller can't teleport themselves. They can still start or grief any ring in the game.
  - B-04: fixed. `cell/cell_methods/player/world/mod.rs:57-67` ignores the client x/y/z. `resolve_hinted_region` (`:219-260+`) does the id, world and containment checks.
  - B-05: no sequence/dedup on 0x03. → #477.
  - B-07: `base/connect_loop/cell_arms.rs:87-88` reads `entity_id_from_client` and only logs it. No mismatch WARN.
  - B-08: `cell/cell_methods/player/combat/mod.rs:166-169` still has `UNIMPLEMENTED: unstuck`.
  - B-10: `base/connect_loop/encrypted/mod.rs:497-504` frames 0x02/0x04/0x05 and only 0x03 has a handler (`:219`).
- Related/duplicates: #478 (closed), #477 (B-05), #474 (CAT-O, overlaps B-02/B-03), **#443 and #63 are duplicates of B-01 and should be closed as completed by #522** (other batch)

### Action text

**Comment:**
> Re-verified on `main` @ 059d6038. B-01, B-06 and B-09 were fixed by #522 (#478). B-04 is fixed by the world-scope and server-side containment gate in `resolve_hinted_region`. B-02 is mostly closed by the address-book check plus walking into the gate volume. B-03 is only partly closed, because the caller of `setRingTransporterDestination` is never checked against the source pad. The checklist below is rewritten to match. The audit file lives on `main`.

#### New body

```markdown
Part of the server-authority audit (#459). Findings: [`docs/security-audit/2026-05-31-server-authority/findings/CAT-B-movement.md`](../blob/main/docs/security-audit/2026-05-31-server-authority/findings/CAT-B-movement.md).

## Problem
Inbound position is now validated (#478), but a few client-hinted travel RPCs and protocol gaps still trust the caller.

## Status (re-verified on main @ 059d6038, 2026-09-25)
- [x] **CAT-B-01** (Critical) 0x03 position unvalidated. Fixed by #522 (#478).
- [x] **CAT-B-06** spaceId cross-check. Fixed by #522.
- [x] **CAT-B-09** navmesh `is_position_valid` unused. Fixed by #522 (jump fix in #643).
- [x] **CAT-B-04** `triggerClientHintedGenericRegion` trusted region_id. Fixed: `resolve_hinted_region` checks the id is non-negative, the region is in the caller's world, and the server-known position is inside it on entry (`cell/cell_methods/player/world/mod.rs:219+`).
- [ ] **CAT-B-03** (High → Med) `setRingTransporterDestination` never checks that the caller is on or near the source pad or in its world (`cell/ring_transport/runtime/entry.rs:94-202`). Only on-pad players travel, but any client can start or grief any ring.
- [ ] **CAT-B-02** (High → Low) `onDialGate` is address-book gated (CAT-O-01) and travel waits for gate-volume entry. What remains: on a world with no gate volume, a dial travels immediately from anywhere (`cell/gate_travel/mod.rs:57-60`). `_source_address_id` is still ignored.
- [ ] **CAT-B-05** (Med) No replay/sequence check on 0x03. → #477
- [ ] **CAT-B-07** (Low) Cell-method `entityId` prefix is discarded silently (`base/connect_loop/cell_arms.rs:87-88`). Add a WARN when it does not match `player_eid`.
- [ ] **CAT-B-08** (Low, UX) `unstuck` is still an `UNIMPLEMENTED` stub (`cell/cell_methods/player/combat/mod.rs:166-169`). The button does nothing, which breaks the rule that every press gets visible feedback.
- [ ] **CAT-B-10** (Low) 0x02/0x04/0x05 avatar updates are framed but dropped (`base/connect_loop/encrypted/mod.rs:497-504`).

## Acceptance criteria
- B-03: `selectDestination` from an entity not in the source ring's `players` set (or not in its world) logs `reason="ring_caller_not_on_pad"` and changes no ring state.
- B-02: on a world with no gate volume, the immediate-travel path requires the caller to be within interact range of that world's DHD/gate point, or it is refused with the existing onErrorCode.
- B-07: a mismatched prefix logs `reason="cell_method_entity_mismatch"` at WARN. Dispatch still uses `player_eid`.
- B-08: `unstuck` moves the player to the nearest valid respawner or navmesh point, with visible feedback.

## Test type
Unit (ring FSM caller gate), negative-log (`LogCapture`) for each new refusal reason.

## Docs to update
CAT-B status banner; `docs/architecture/negative-logging-convention.md`.

## Client impact
Free.

## Domain advisor
`movement-teleport-advisor`

## Needs a human for
Nothing.
```

---

## #462 — [security-audit] CAT-C — Combat / Abilities (15 findings)

- Verdict: REWRITE
- Priority: P1 (C-01+C-02 give a free full heal plus a teleport to any respawner in any world on demand; C-05 lets a stunned player keep casting now that stun effects exist)
- Labels: no change
- Summary: Two of 15 are fixed. C-03 (friendly fire, PR #514/#444) and C-10 (AI-debug GM gate, PR #512/#475). C-11 (pets) and C-12 (confirmationResponse) are latent: the systems don't exist. C-15 is low: the auto-cycle premise is wrong and a forged shot only wastes the sender's own ammo. The rest are still open. C-05 is worse than when filed: the Stun effect now sets `BSF_MOVEMENT_LOCK`, but `useAbility` only checks for dead.
- Evidence:
  - C-01 open: `cell/cell_methods/player/combat/mod.rs:35-76` (callForAid) and `:160-164` (respawn) call `respawn::handle_respawn` with no `BSF_DEAD` check. It is shared with GM `gmRespawn` (`combat/respawn.rs:74-76`), so the gate goes in the two dispatch arms.
  - C-02 open: `combat/respawn.rs:396-400` looks up the global `space_mgr.respawners`. A respawner in another world routes through GateTravel (`:122-150`). The offered list is built per-world (`abilities/death/side_effects.rs:126-134`) but never stored or checked.
  - C-03 fixed: `abilities/use_ability/handle.rs:220-237` (PR #514, #444). Heal/buff abilities on friendly targets still need the inverse gate (TODO at the guard).
  - C-04 open: `handle.rs:239-258` checks range only. A navmesh LoS primitive now exists (`cell/space_manager/spatial.rs:21,36`, NPC-AI campaign) but player fire doesn't use it. Collision LoS is #784.
  - C-05 open and now live: `handle.rs:132` checks `is_dead_state` only. Stun sets `BSF_MOVEMENT_LOCK` (`cell/effects/scripts.rs:421-434`).
  - C-06/C-13 open: `cell/cell_methods/being.rs:20-64` stores any id > 0 and sends `onTargetUpdate` to all witnesses.
  - C-07 open: `combat/mod.rs:98-104` has raw floats and no `is_finite`. `abilities/dispatch.rs:143-176` never range-checks the click point.
  - C-08 open: only `max_range` is checked (`handle.rs:239-258`). NPC AI respects min_range (`cell/npc_ai/ability_select.rs:153-185`).
  - C-09 open: `cell/cell_methods/combatant.rs:31-94` has no dead check.
  - C-10 fixed: `cell/dispatch/gm_gate.rs:122-135` (PR #512).
  - C-11 latent: `cell/cell_methods/player/social.rs:15-58` stubs. Pets are not implemented. → #570
  - C-12 latent: `cell/cell_methods/ability_manager.rs:26-32` logs only. The server never sends a prompt.
  - C-14 open: `combat/respawn.rs:159-164` refills HEALTH and FOCUS. It only matters through C-01.
  - C-15: `handle.rs:596-609`. A shot with no target spends the sender's own ammo. The auto-cycle drain is impossible (`being.rs:31` stores None for ids ≤ 0).
- Related/duplicates: #444 (closed), #475 (closed), #570 (pets), #233 (per-player unlocked respawners, overlaps C-02), #784 (collision LoS), #569 (duels/PvP)

### Action text

**Comment:**
> Re-verified on `main` @ 059d6038. C-03 (friendly fire) was fixed by #514 and C-10 (AI-debug GM gate) by #512. C-05 is now reachable in play because stun effects set `BSF_MOVEMENT_LOCK` and `useAbility` doesn't check it. C-11 (pets) is folded into #570 as an acceptance criterion. C-12 stays here as a design requirement, since no confirmation prompt exists yet. The checklist is rewritten below with current file:line refs. C-01, C-02 and C-05 are the priority.

#### New body

```markdown
Part of the server-authority audit (#459). Findings: [`docs/security-audit/2026-05-31-server-authority/findings/CAT-C-combat-abilities.md`](../blob/main/docs/security-audit/2026-05-31-server-authority/findings/CAT-C-combat-abilities.md).

## Problem
The player combat RPCs still trust caller state and client-chosen targets in several places. The worst case today: a *live* player sends `callForAid(<any respawner id>)` and gets a full heal plus a teleport to any respawner in any world.

## Status (re-verified on main @ 059d6038, 2026-09-25)
Priority first:
- [ ] **CAT-C-01** (High) `respawn` / `callForAid` run on a live player (`cell/cell_methods/player/combat/mod.rs:35-76`, `:160-164`). Put the `BSF_DEAD` gate in the two dispatch arms, not in `handle_respawn`, which GM `gmRespawn` shares (`combat/respawn.rs:74-76`).
- [ ] **CAT-C-02** (High) `callForAid` accepts any respawner id from the global table (`combat/respawn.rs:396-400`), cross-world included (`:122-150`). Store the list offered at death (`abilities/death/side_effects.rs:126-134`) and only accept ids from it. See also #233.
- [ ] **CAT-C-05** (Med, now live) `useAbility` ignores stun / `BSF_MOVEMENT_LOCK` (`abilities/use_ability/handle.rs:132`; Stun sets the flag at `cell/effects/scripts.rs:421-434`). Same gap on the ground-target path.
- [ ] **CAT-C-04** (High) No LoS check on player single-target fire (`handle.rs:239-258`). Reuse the navmesh LoS primitive (`cell/space_manager/spatial.rs:21,36`) now. Full collision LoS is #784.
- [ ] **CAT-C-06 / CAT-C-13** (Med/Low) `setTargetID` stores and broadcasts any id (`cell/cell_methods/being.rs:20-64`). Check the target exists, is in the same space, and is in the caller's AoI before storing or fanning out.
- [ ] **CAT-C-07** (Med) Ground-target point has no `is_finite` check and no range check against the attacker (`combat/mod.rs:98-104`, `abilities/dispatch.rs:143-176`).
- [ ] **CAT-C-08** (Low) `min_range` is not enforced for player fire (`handle.rs:239-258`, `dispatch.rs:143`).
- [ ] **CAT-C-09** (Low) `setCrouched` / `requestHolsterWeapon` have no dead-state guard (`cell/cell_methods/combatant.rs:31-94`).
- [ ] **CAT-C-14** (Low) Respawn refills Focus as well as Health (`combat/respawn.rs:159-164`). Design question. Only exploitable through C-01.
- [ ] **CAT-C-15** (Low) A `useAbility` with no target still spends cooldown and ammo (`handle.rs:596-609`). Only the sender's own resources, so it is cosmetic.

Resolved / moved:
- [x] **CAT-C-03** friendly-fire filter, fixed by #514 (#444). Follow-up: an inverse gate for supportive single-target abilities.
- [x] **CAT-C-10** AI-debug messages GM-gated, fixed by #512 (#475) (`cell/dispatch/gm_gate.rs:122-135`).
- [x] **CAT-C-11** pet ownership moved to #570 as an acceptance criterion (pets are not implemented).
- [ ] **CAT-C-12** (design requirement) `confirmationResponse` (`cell/cell_methods/ability_manager.rs:26-32`). When prompts are implemented, accept only an effect_id matching a prompt the server issued to this player.

## Acceptance criteria
- A live player's `callForAid`/`respawn` logs `reason="respawn_not_dead"` and changes no health, position or world.
- `callForAid(id)` for an id not in the player's offered set logs `reason="respawner_not_offered"`.
- A stunned or movement-locked caller's `useAbility` / `useAbilityOnGroundTarget` is refused with feedback.
- `setTargetID` with an unknown or out-of-AoI id stores nothing and broadcasts nothing.

## Test type
Unit + negative-log (`LogCapture`) per refusal; chain-replay for respawn from a live vs dead state.

## Docs to update
CAT-C status banner; `docs/architecture/abilities-and-effects-system.md` if the stun gate changes the effect contract.

## Client impact
Free.

## Domain advisor
`combat-systems-advisor`, then `server-authority-enforcer` review.

## Needs a human for
Nothing. In-game UAT of the defeat window after C-01/C-02.
```

- Also post on **#570** (fold C-11): "Security acceptance criterion from CAT-C-11 (#462): `PetInvokeAbility` / `PetAbilityToggle` / `PetChangeStance` must resolve `pet_entity_id` through a server-side pet→owner map and refuse (with a negative log) any pet the caller does not own. Stubs today: `crates/services/src/cell/cell_methods/player/social.rs:15-58`."

---

## #463 — [security-audit] CAT-D — Inventory / Items (9 findings)

- Verdict: REWRITE
- Priority: P2 (the D-01 dupe is fixed; D-03 loot ownership is a design gap that only matters with more than one player; the related buyback dupe goes to #464)
- Labels: no change
- Summary: D-01 (bandolier type_id TOCTOU) is fixed by PR #520 (#445). D-02 is partly fixed: PR #515 (#446) re-checks range on every take, but not alive state or the loot flag, and it lets the take through if either position can't be read. D-04, D-07 and D-09 have wrong premises, as aurablacklight's comment of 2026-09-19 says and as re-confirmed here. D-03 (loot reservation), D-06 (ammo-change cache-miss fail-open, fix in PR #602 which is open and CONFLICTING) and D-08 (by design, harmless) are still open. D-05 is latent. While checking D-07 we found a **new, higher-severity gap**: MoveItem never checks the *source* container, so sold items can be moved out of buyback (16) for free. We file that under CAT-E (#464) because it is a vendor-economy dupe.
- Evidence:
  - D-01 fixed: `base/world_entry/methods/inventory/ammo.rs:32-36` has `WHERE ... AND item_id = $5`. `BandolierItem.instance_id` is at `crates/entity/src/cell_entity/mod.rs:107-114`. PR #520 (#445).
  - D-02 partial: `cell/interactions/loot.rs:132-163` re-checks distance. The `if let (Some, Some)` at `:149` lets the take through when a position is missing. No looter-alive or `INT_NormalLoot` recheck.
  - D-03 open: `LootItem` has no owner or claim field (`crates/entity/src/cell_entity/mod.rs:96-103`). `loot.rs:166-180` removes by index. `squadSetLootMode` is a stub (`cell/cell_methods/organization.rs:140-143`). The `loot.rs:142-146` comment names it as the #446 follow-up.
  - D-04 wrong premise: `bound` is never set true (`base/world_entry/methods/inventory/grant/grant_item.rs:331-336`, `db/sgw/Inventory/Tables/sgw_inventory.sql:16`).
  - D-05 latent: `target_id` is forwarded (`cell/cell_methods/inventory/item_ops.rs:155`) but nothing uses it (`cell/content/event_dispatch/inventory.rs:58,104` set `target_entity: None`).
  - D-06 open: `cell/cell_methods/inventory/bandolier/ammo_change.rs:78-84,128`. The fix is PR #602 (`fix/448-ammo-change-fail-closed`, CONFLICTING, last updated 2026-07-06). Tracked by #448.
  - D-07 wrong premise: `item_allows_container` (`grant/validation.rs:27-57`). No seeded `container_sets` contains 16.
  - D-08: `move_/mod.rs:337` copies `source.bound` under the per-player advisory lock. With `bound` always false this is harmless. No action.
  - D-09 wrong premise: `entities/defs/interfaces/SGWInventoryManager.def:144-148` declares `INT16 quantity`.
- Related/duplicates: #445/#446 (closed), #448 + PR #602 (D-06), **#165 is a duplicate of D-01/#445 and should be closed as completed by #520** (other batch), #62 (corpse lifecycle), #472 (CAT-M squad loot mode)

### Action text

**Comment:**
> Re-verified on `main` @ 059d6038. D-01 was fixed by #520 (#445). D-02's range recheck landed in #515 (#446), with gaps still open. D-04, D-07 and D-09 are closed as wrong premises; the evidence is in the 2026-09-19 comment above, re-confirmed. D-06 has a fix waiting in PR #602, which needs a rebase. While re-checking D-07 we found that MoveItem never validates the **source** container, so a sold item can be moved out of buyback (16) back into the bag without paying. That is a naquadah dupe, and it is filed under CAT-E (#464). The body below is the updated checklist.

#### New body

```markdown
Part of the server-authority audit (#459). Findings: [`docs/security-audit/2026-05-31-server-authority/findings/CAT-D-inventory.md`](../blob/main/docs/security-audit/2026-05-31-server-authority/findings/CAT-D-inventory.md).

## Problem
Inventory mutation is transaction-safe. What is left is loot authority (who may take what, and whether they are still allowed to) plus one fail-open ammo path.

## Status (re-verified on main @ 059d6038, 2026-09-25)
- [ ] **CAT-D-03** (High, needs design) No loot reservation or ownership. `LootItem` has no owner (`crates/entity/src/cell_entity/mod.rs:96-103`) and `cell/interactions/loot.rs:166-180` removes by index for anyone who passed `interact()`. It needs a kill-credit/owner set per corpse plus squad loot modes (`squadSetLootMode` is a stub, `cell/cell_methods/organization.rs:140-143`, see #472).
- [ ] **CAT-D-02** (High → Med) Partly fixed by #515. Still missing: a looter-alive check, an `INT_NormalLoot` recheck, and failing closed when either position is unreadable (`loot.rs:149`).
- [ ] **CAT-D-06** (Med) `requestAmmoChange` accepts any ammo type on a weapon-def cache miss (`cell/cell_methods/inventory/bandolier/ammo_change.rs:78-84,128`). Fix is in PR #602 (needs a rebase). → #448
- [ ] **CAT-D-05** (design requirement) `useItem.target_id` is forwarded unvalidated (`cell/cell_methods/inventory/item_ops.rs:155`) but nothing uses it yet. The first content action that reads it must check the target exists and is in the same space and within range.

Resolved / dropped:
- [x] **CAT-D-01** bandolier ammo keyed by type_id, fixed by #520 (#445).
- [x] **CAT-D-04** dropped. `sgw_inventory.bound` is never true (grant INSERT hardcodes false). A future bound gate needs a client-vs-content source flag, because content `RemoveItem` reuses the same message.
- [x] **CAT-D-07** dropped. Moving *into* 16 is blocked by `item_allows_container` (`grant/validation.rs:27-57`). Moving *out of* 16 is a real bug, tracked in #464.
- [x] **CAT-D-08** won't fix. Split copies `bound` under the advisory lock, and bound is always false.
- [x] **CAT-D-09** dropped. The .def declares `INT16 quantity` (`SGWInventoryManager.def:144-148`).

## Acceptance criteria
- A take from a corpse by a player without loot rights logs `reason="loot_not_owner"` and moves nothing. Owners follow kill credit (solo) or the squad loot mode.
- A dead looter or a missing position refuses the take (`reason="loot_looter_dead"` / `loot_position_unknown`).

## Test type
Unit (loot rights predicate), live-DB (no item granted on refused take), negative-log.

## Docs to update
CAT-D status banner; `docs/gameplay/` loot page if loot rules become player-visible.

## Client impact
Free.

## Domain advisor
`items-systems-advisor`; `social-systems-engineer` for squad loot modes.

## Needs a human for
An owner decision on loot rules (FFA after N seconds? round-robin?) before D-03 is `ready-for-agent`.
```

---

## #464 — [security-audit] CAT-E — Vendor (6 findings)

- Verdict: REWRITE
- Priority: P1 (two unlimited naquadah/durability exploits reachable today with a scripted client: E-01 free repair/recharge, and the new free-buyback-through-MoveItem dupe)
- Labels: no change. Consider `ready-for-agent` for the buyback-source fix once rewritten, because the fix and test shape are clear.
- Summary: None of the six CAT-E findings is fixed. E-01 is unchanged: leaving out the trailing `vendor_template_id` still selects the free repair/recharge path, and live-DB tests treat that as intended. E-02: `vendor_entity` has one writer and is never cleared, and `vendor_context` has no distance check. E-04/E-05/E-06 (buyback rows not tied to a vendor, never expire, silent abort at 12) are open. E-05 now breaks the button-feedback rule, and once 12 items are in buyback, selling stops for good. E-03 is latent. **New:** MoveItem checks only the *target* container. A player can sell an item (it goes to container 16 and they are paid) and then `moveItem` it back to the bag for free.
- Evidence:
  - E-01: `cell/cell_methods/player/vendor/mod.rs:78-91` (`None => None` with the comment "signals free repair") and `:146-157`. The base free branches are `base/world_entry/methods/vendor/repair.rs:122,156` and `vendor/recharge.rs:32,109`. Tests pin free behaviour at `repair.rs:398` and `recharge.rs:333`.
  - E-02: the only write is `cell/interactions/vendor.rs:20`. `vendor/session.rs:22-35` `vendor_context` checks only that the vendor still exists.
  - E-03 latent: `cell/cell_methods/inventory/item_ops.rs:179-190` is a stub. `vendor/repair.rs:22-104` has no naquadah debit (`:53`).
  - E-04/E-06: `vendor/buyback/mod.rs:84-89` filters `container_id=16 AND flags>0` only. The `vendor_template_id` parameter is used only for tracing (`:32,38`). `vendor/sell/mod.rs:219-222` writes `flags = unit_price`.
  - E-05: `vendor/sell/mod.rs:128-139` rolls back with a server WARN only and no client packet.
  - NEW: `base/world_entry/methods/inventory/move_/mod.rs:162-164` selects the source by `character_id AND item_id` with no container check. Only the target goes through `item_allows_container` (`:241`). No `INV_BUYBACK` reference exists in the inventory move path. Checked statically, not reproduced live.
- Related/duplicates: #463 (D-07 is the other half of the buyback finding), #465 (CAT-F-01 has the same "interaction target never cleared" shape)

### Action text

**Comment:**
> Re-verified on `main` @ 059d6038. None of the six findings is fixed. There is also a new one, found while re-checking CAT-D-07: `MoveInventoryItem` only validates the target container, so a sold item sitting in buyback (16) can be moved straight back into the bag with no payment (`base/world_entry/methods/inventory/move_/mod.rs:162-164`). Together with E-01 (leave out the trailing template id and repair is free), that makes two scriptable economy exploits, so priority is raised. E-05 also breaks the rule that every button press gets visible feedback: selling fails silently once buyback holds 12 items, and because buyback rows never expire it stays broken. The body is rewritten below.

#### New body

```markdown
Part of the server-authority audit (#459). Findings: [`docs/security-audit/2026-05-31-server-authority/findings/CAT-E-vendor.md`](../blob/main/docs/security-audit/2026-05-31-server-authority/findings/CAT-E-vendor.md).

## Problem
The vendor stack prices things on the server and runs inside transactions. Two wire-level holes still let a scripted client create value: leaving out a field selects a free code path, and a sold item can be moved back out of buyback. On top of that, the vendor session never expires and buyback has no lifecycle.

## Status (re-verified on main @ 059d6038, 2026-09-25)
- [ ] **NEW (High)** Free buyback through MoveItem. `MoveInventoryItem` selects the source by `character_id AND item_id` only (`base/world_entry/methods/inventory/move_/mod.rs:162-164`) and validates only the target container (`:241`). Sell → `moveItem(item, 1, slot, -1)` gives the item back with the sale price kept. Fix: refuse a move whose source container is 16 (buyback), or any container that is not player-movable.
- [ ] **CAT-E-01** (High) When the trailing `vendor_template_id` is missing, `repairItems`/`rechargeItems` take the free path (`cell/cell_methods/player/vendor/mod.rs:78-91,146-157` → `base/world_entry/methods/vendor/repair.rs:122,156`, `recharge.rs:32,109`). Unless the owner confirms that free repair is a real 2009 flow (then gate it on that flow's own server-side state), require the template id as Purchase/Sell/Buyback already do, and update the live-DB tests that pin free behaviour (`repair.rs:398`, `recharge.rs:333`).
- [ ] **CAT-E-02** (Med) `vendor_entity` is never cleared (only writer `cell/interactions/vendor.rs:20`). `vendor_context` (`vendor/session.rs:22-35`) has no distance, space or alive check. Re-check `MAX_INTERACT_DISTANCE` on each op and clear the field on death, zone change and moving out of range.
- [ ] **CAT-E-05** (Low → UX bug) With buyback full (12), a sale is rolled back with a server-side WARN only (`vendor/sell/mod.rs:128-139`). The client gets no feedback. Evict the oldest buyback row (the original behaviour), or send an error.
- [ ] **CAT-E-04 / CAT-E-06** (Low) Buyback rows are tied to neither vendor nor time. The price lives in `flags` (`vendor/sell/mod.rs:219-222`) and the buyback query ignores `vendor_template_id` (`vendor/buyback/mod.rs:32-38,84-89`). Add vendor + sold-at columns (seed schema in `db/sgw/`) and expire on logout or after a time window.
- [ ] **CAT-E-03** (design requirement) Single-item `RepairItem` with a client-supplied `repair_ratio` is a stub (`cell/cell_methods/inventory/item_ops.rs:179-190`). The base handler has no debit (`vendor/repair.rs:53`). Do not wire it up without a server-computed cost.

## Acceptance criteria
- `moveItem` from container 16 is refused with `reason="move_from_buyback"`. The live-DB row stays in 16 and naquadah is unchanged.
- `repairItems` with no template id either fails (`reason="vendor_template_missing"`) or is allowed only through a documented free-repair source.
- A vendor op from more than `MAX_INTERACT_DISTANCE` away is refused and clears `vendor_entity`.
- Selling with buyback full gives the client visible feedback on the first press.

## Test type
Live-DB regression guards (the buyback dupe and the free repair must each fail when the fix is reverted), negative-log, and a wire-format test for the template-id-required arm.

## Docs to update
CAT-E status banner; `docs/gameplay/` vendor page; the schema docs if buyback columns are added.

## Client impact
Free.

## Domain advisor
`items-systems-advisor`, then `server-authority-enforcer`.

## Needs a human for
An owner answer on whether the 2009 game had a free-repair path (for E-01).
```

---

## #465 — [security-audit] CAT-F — Crafting / R&D (8 findings)

- Verdict: REWRITE
- Priority: P2
- Labels: no change
- Summary: None is fixed. Two are live in working `trainAbility` code. F-01: you can train anywhere, because there is no trainer interaction or proximity gate. F-08: the base-side UPDATE doesn't re-check level or archetype. The other six (F-02..F-07) target the crafting activity handlers (methods 95-100), which are still stubs that log UNIMPLEMENTED and return true (`cell/cell_methods/player/crafting.rs:23-86`). PR #427 added crafting-state persistence and #521 added GM grants only. The six should be folded into #567 (crafting activity handlers) as security acceptance criteria. This issue then shrinks to the two live trainAbility items.
- Evidence:
  - F-01: `cell/cell_methods/player/vendor/train.rs:31-199` runs six checks with no trainer or distance check and sends `CellToBaseMsg::TrainAbility` at `:177`. A ready anchor exists in `last_interaction_target` (set at `cell/interactions/interact.rs:208`). It is never cleared on trainer close (`service/base_messages/ability_granted.rs:66-86`).
  - F-08: `base/world_entry/methods/progression/mod.rs:497`. The UPDATE (~`:553-560`) guards only `training_points > 0` and not-known. Level is checked only on the cell side (`train.rs:123`).
  - F-02..F-07: stubs at `crafting.rs:23-43` (ASP), `:45-57` (craft), `:59-67` (research/RE, which don't parse args), `:69-81` (alloy), `:83-86` (respec). There is no busy or induction state anywhere in `cell/`. #567 covers most functional gates. It does not yet list item-owner checks, `FOR UPDATE` plus a single transaction, idempotent DB guards for ASP spend, a busy gate (`@mustBeIdle`), rolls computed only on the server, or the precedence bug in the Python alloy check (`Crafter.py:469/473`, `x != y is None`, which means the original never checked the current-tier item type).
- Related/duplicates: #567 (fold F-02..F-07), #723 (paradigm gate for F-02), #53 (closed epic), #55 (closed, trainer NPC)

### Action text

**Comment on #465:**
> Re-verified on `main` @ 059d6038. The crafting activity handlers (95-100) are still stubs, so F-02..F-07 are design requirements. They have been moved into #567 as security acceptance criteria (see the comment there). This issue now tracks only the two findings that are live in the working `trainAbility` path: F-01 (train from anywhere) and F-08 (the base doesn't re-validate level). Rewritten below.

#### New body

```markdown
Part of the server-authority audit (#459). Findings: [`docs/security-audit/2026-05-31-server-authority/findings/CAT-F-crafting.md`](../blob/main/docs/security-audit/2026-05-31-server-authority/findings/CAT-F-crafting.md).

## Problem
`trainAbility` (cell method 77) validates the ability tree well, but it dropped the 2009 trainer requirement. Any client can train any ability it qualifies for from anywhere on the map. The base-side debit also trusts the cell's copy of the player's level.

## Status (re-verified on main @ 059d6038, 2026-09-25)
- [ ] **CAT-F-01** (Med) No trainer-interaction or distance gate (`cell/cell_methods/player/vendor/train.rs:31-199`). Use `last_interaction_target` (set at `cell/interactions/interact.rs:208`). Require that it is a trainer whose `template_trainer_lists` contains the ability, and re-check `MAX_INTERACT_DISTANCE` at train time. The distance re-check is mandatory because the field is never cleared (`service/base_messages/ability_granted.rs:66-86`).
- [ ] **CAT-F-08** (Low) The base UPDATE (`base/world_entry/methods/progression/mod.rs:497`, ~`:553-560`) guards only `training_points > 0` and not-already-known. Add a level (and archetype-tree) predicate to the SQL guard.

Moved to #567 (crafting handlers are still stubs, `cell/cell_methods/player/crafting.rs:23-86`):
- [x] CAT-F-02 ASP spend, F-03 craft, F-04 research/RE, F-05 alloying, F-06 accept-before-validate, F-07 busy/induction gate.

## Acceptance criteria
- `trainAbility` with no open trainer, the wrong trainer, or a trainer out of range logs `reason="train_no_trainer"` / `train_out_of_range` and debits nothing. The player gets the existing error feedback.
- A base-side train for a character below the ability's level affects 0 rows and logs it.

## Test type
Unit (trainer gate), live-DB (0 rows affected on a level mismatch), negative-log.

## Docs to update
CAT-F status banner; `docs/gameplay/` abilities/trainer page.

## Client impact
Free.

## Domain advisor
`combat-systems-advisor` (abilities); `server-authority-enforcer` review.

## Needs a human for
Nothing.
```

**Also post on #567:**
> Security acceptance criteria folded in from CAT-F (#465, audit file `docs/security-audit/2026-05-31-server-authority/findings/CAT-F-crafting.md`). The handlers are still stubs at `crates/services/src/cell/cell_methods/player/crafting.rs:23-86`. Each phase must:
>
> 1. Resolve every client-named item_id with `WHERE character_id = <caller> ... FOR UPDATE`, then consume the inputs and grant the outputs in **one** transaction (F-03/F-04/F-05).
> 2. Make ASP spend idempotent in SQL (`WHERE applied_science_points >= 1 AND NOT discipline_ids @> ...`) so a replayed packet can't double-spend (F-02; the paradigm gate depends on #723).
> 3. Compute every research/RE/alloy outcome on the server only, with no client seed or result. Use half-open `gen_range(0..n)`, because Python `randint` is inclusive (F-04).
> 4. Do **not** port the precedence bug in `Crafter.py:469/473` (`x != y is None`). Check the current-tier item's type (F-05).
> 5. Finish all validation before setting busy, arming the timer or sending `onCraftingStarted`. Refuse new craft RPCs while busy (the `@mustBeIdle` equivalent) (F-06/F-07).
> 6. Respec charges naquadah and clears disciplines atomically.

---

## #466 — [security-audit] CAT-G — Mail (8 findings)

- Verdict: CLOSE (not planned). Folded into #72.
- Priority: P3 (latent; nothing to exploit until mail send/take/COD is written)
- Labels: n/a
- Summary: Unchanged since the audit. `sendMailMessage`, `takeCash`, `takeItem`, `payCOD` and `returnMailMessage` are all still `UNIMPLEMENTED` stubs that return true (`cell/cell_methods/mail.rs:32-95`). Only headers, body, archive and delete run, and those are correctly scoped by `character_id`. So G-01..G-06 are design requirements for the mail implementation (#72), not live bugs. The two live Low items are defense-in-depth fixes of one line each: G-07 (read_time UPDATE without `character_id`, `base/world_entry/methods/mail/mod.rs:193`, behind a SELECT that is scoped) and G-08 (`ToText` filled with the reader's name, `:203-210`). They fit #72 as well. The audit file on main keeps the permanent record. Note that #72's 2026-05-27 triage comment claims "Send mail, Attach item, Take item, Take cash" work. That is **false** on main (they are stubs). #72 needs that correction whichever batch owns it.
- Evidence: `cell/cell_methods/mail.rs:32-35,52-57,68-95` (stubs); `base/world_entry/methods/mail/mod.rs:157` (body SELECT scoped by character_id), `:193` (UPDATE by mail_id only), `:203-210` (ToText = reader name); PR #586 (BM branch, open and CONFLICTING) contains a `send_mail_to_player` helper for the BM cascade that is not on main.
- Related/duplicates: #72 (mail implementation), #571 / PR #586 (BM mail cascade)

### Action text

**Comment on #466 (then close as not planned):**
> Re-verified on `main` @ 059d6038. Mail send, take-cash, take-item, pay-COD and return are still `UNIMPLEMENTED` stubs (`crates/services/src/cell/cell_methods/mail.rs:32-95`). Every CAT-G finding except G-07 and G-08 is therefore a requirement for the implementation, not a live bug. All eight have been moved into #72 as acceptance criteria (see the comment there). The per-finding record stays in `docs/security-audit/2026-05-31-server-authority/findings/CAT-G-mail.md` on main. Closing so mail security is tracked in the same place as mail work.

**Comment to post on #72:**
> Correction to the 2026-05-27 triage above: on `main` @ 059d6038, `sendMailMessage`, `takeCashFromMailMessage`, `takeItemFromMailMessage`, `payCODForMailMessage` and `returnMailMessage` are **stubs** (`crates/services/src/cell/cell_methods/mail.rs:32-95`). Only headers, body, archive and delete are implemented.
>
> Security acceptance criteria folded in from CAT-G (#466, `docs/security-audit/2026-05-31-server-authority/findings/CAT-G-mail.md`):
>
> - G-01 send: the recipient resolves server-side. Attached items are checked with `WHERE character_id = <caller> ... FOR UPDATE` and must be tradeable. Cash and COD must be ≥ 0 and are debited in the same transaction that inserts the mail. A failure sends `sendMailResult` instead of silently returning "handled".
> - G-02/G-03/G-04 take: one transaction per take, keyed `mail_id AND character_id = <caller>`. Zero the cash or clear the attachment with `rows_affected == 1` as the guard, so a replayed take or a cash+item race cannot pay out twice. Check inventory space before clearing the attachment.
> - G-05 COD: the amount comes from the stored mail row, never from the client. Debit the recipient and credit the sender atomically before releasing the attachment.
> - G-06 return: re-address to the stored original sender and move attachments inside one transaction. Never take a sender from the client.
> - G-07: add `AND character_id = $3` to the read_time UPDATE (`crates/services/src/base/world_entry/methods/mail/mod.rs:193`).
> - G-08: fill `onMailRead.ToText` from the stored recipient, not the reader's name (`mod.rs:203-210`).
> - Tests: live-DB double-take and COD-replay guards that fail when the `rows_affected` guard is removed.

---

## #467 — [security-audit] CAT-H — Trade (P2P) (10 findings)

- Verdict: REWRITE (reduce to the one residual). CLOSE would also be defensible, see the note below.
- Priority: P2
- Labels: no change
- Summary: The audit's premise, "Rust has no trade implementation", stopped being true about eight hours after filing. PR #438 (merged 2026-06-01, closes #54) implemented trade with most of the audit's guardrails. The session gauntlet covers not-self, both sides are players, same space, `MAX_INTERACT_DISTANCE`, and not already trading. Proposal versions must increase monotonically, items are deduplicated, the range is re-checked on every update and lock, and the session is cancelled on disconnect. The base commit is atomic: one transaction, per-player advisory locks, `FOR UPDATE` on both player rows and every offered item, ownership re-validated, a container whitelist, a `bound` refusal, slot reservation, negative cash rejected, rollback on any failure. H-01..H-06 and H-08..H-10 are fixed. What remains is H-07 in a smaller form: nothing locks the offered items *during* the session. The commit re-validates, so there is no dupe, but after the partner confirms the offerer can still consume or split the offered stack. The commit then either cancels or transfers the smaller stack (bait-and-switch). There is also a small gap: `begin_trading` has no alive check.
- Evidence:
  - `cell/cell_methods/player/trade/state.rs:27-130` (`begin_trading` gauntlet), `:140-200` (`apply_proposal` version +1 check and dedup), `:125-152` (`partners_in_range`); `trade/handlers.rs:238-252,309-323` (range re-check with auto-cancel), `:326+` (lock version check); `trade/mod.rs:43-49` (`cancel_trade_on_disconnect`, test `cell/service/base_messages/tests/trade_disconnect.rs`).
  - `base/world_entry/methods/trade/execute/mod.rs:1-40,139-150` (negative cash), `execute/swap.rs:66-72` (advisory locks), `:306-336` (`FOR UPDATE` item rows, `bound` refusal, `TRADEABLE_CONTAINERS`).
  - H-07 residual: no inventory mutation path looks at `trade_partner_entity_id` (grep in `base/world_entry/methods/inventory`, `cell/cell_methods/inventory` and `vendor` finds nothing).
  - PR #438 merged 2026-06-01 (`merged-prs.tsv`).
- Related/duplicates: #54 (closed), #463/#464 (inventory paths that would need the lock check)

### Action text

**Comment:**
> Re-verified on `main` @ 059d6038. Trade was implemented by #438 (merged 2026-06-01, a few hours after this audit was filed), and it covers nine of the ten findings. The cell session checks self, player, same space, range, busy, version and dedup, and cancels on disconnect. The base commit runs in one transaction with advisory and `FOR UPDATE` locks, re-validates ownership, whitelists containers, refuses bound items and rolls back on failure. The one remaining gap is H-07, in a smaller form. Offered items are not locked while the trade is open, so the offerer can use or split a stack after the partner confirms. The commit's re-validation prevents a dupe, but the partner can receive less than they saw. Rewritten to track only that. (If the maintainer prefers, close this as completed by #438 and open a one-line ticket for the H-07 residual instead.)

#### New body

```markdown
Part of the server-authority audit (#459). Findings: [`docs/security-audit/2026-05-31-server-authority/findings/CAT-H-trade.md`](../blob/main/docs/security-audit/2026-05-31-server-authority/findings/CAT-H-trade.md). Trade shipped in #438.

## Problem
While a trade is open, the offered items are not locked against the offerer's other inventory operations (use, split, move, sell, destroy). The base commit re-validates ownership and containers under `FOR UPDATE` (`base/world_entry/methods/trade/execute/swap.rs:306-336`), so an item that disappears only cancels the trade. But a stack the offerer reduces after the partner confirms is transferred at its new, smaller size. That is a bait-and-switch.

## Status (re-verified on main @ 059d6038, 2026-09-25)
- [x] CAT-H-01..H-06, H-08..H-10 fixed by #438 (session gauntlet `cell/cell_methods/player/trade/state.rs:27-152`; atomic swap `base/world_entry/methods/trade/execute/`).
- [ ] **CAT-H-07** (Med → Low) Offered items are not locked during the session. No inventory path looks at `trade_partner_entity_id`.
- [ ] (small) `begin_trading` has no alive check on either party (`state.rs:27-130`).

## Acceptance criteria
Either (a) inventory mutations on an item in an open trade proposal are refused with `reason="item_in_trade"`, or (b) any inventory change to an offered item resets both lock states and re-sends `onTradeState`, as a proposal update already does. In addition, the commit transfers exactly the stack size both sides confirmed, or it cancels.

## Test type
Unit (lock reset on inventory change), live-DB (commit with a reduced stack cancels instead of transferring).

## Docs to update
CAT-H status banner; `docs/gameplay/` trade page if the behaviour becomes player-visible.

## Client impact
Free.

## Domain advisor
`social-systems-engineer`, `items-systems-advisor`.

## Needs a human for
Nothing.
```

---

## #468 — [security-audit] CAT-I — Black Market / Auction (6 findings)

- Verdict: CLOSE (not planned). Folded into #571 / PR #586.
- Priority: P3 (latent on main)
- Labels: n/a
- Summary: On main the whole `SGWBlackMarketManager` surface is still the parse-and-log stub (`cell/cell_methods/black_market.rs`). It still decodes `auction_length` as an i32 (I-02 wire note). The real implementation is open PR #586 (`feat/571-black-market-phase1`). It is CONFLICTING and was last updated 2026-06-22. The CAT-I file's 2026-07-25 banner (on main) already checked that branch. I-01, I-03, I-04 and I-06 are addressed there. Still open there: I-02 has no listing fee and no per-player cap; I-05 search has no LIMIT (`search.rs:45` `fetch_all`, confirmed on the branch head 6dc1b6c6); `BMStart/StopWatchingItem` are stubs; `next_min_bid` is a guessed formula. Every finding is either latent on main or a review item for #586, so this issue duplicates #571. Move the residuals into #571's acceptance criteria and close.
- Evidence: `cell/cell_methods/black_market.rs:14-80` (stub on main); no `base/black_market/` on main; `gh pr view 586`: OPEN, not draft, mergeable=CONFLICTING, updated 2026-06-22; `git show origin/feat/571-black-market-phase1:crates/services/src/base/black_market/search.rs` line 45 `.fetch_all(pool)` with no LIMIT; banner at `docs/security-audit/.../CAT-I-black-market.md:3-44`.
- Related/duplicates: #571 (BM implementation), PR #586, #72 (the mail cascade the BM expiry sweep depends on), #477 (transport replay, the CAT-I-03 caveat)

### Action text

**Comment on #468 (then close as not planned):**
> Re-verified on `main` @ 059d6038. The Black Market is still the parse-and-log stub on `main`. The implementation is PR #586, which is open and needs a rebase. The CAT-I status banner on `main` (2026-07-25) shows that branch already closes I-01, I-03, I-04 and I-06. The remaining items have been moved into #571 as acceptance criteria (see the comment there). The per-finding record stays in `docs/security-audit/2026-05-31-server-authority/findings/CAT-I-black-market.md`. Closing to avoid tracking the same work twice.

**Comment to post on #571:**
> Security acceptance criteria folded in from CAT-I (#468). Re-verify all of these when PR #586 is rebased:
>
> - I-05: `BMSearch` must cap its result set (SQL `LIMIT` plus server-side pagination; the client's `clientKey` is only a cursor hint). On the branch head, `base/black_market/search.rs:45` is an unbounded `fetch_all`.
> - I-02: deduct the listing fee inside the create transaction, and cap active listings per player. Keep the duration as the server-side enum (`wire.rs:48-57`).
> - I-03: confirm `next_min_bid` against the client (the 5% increment is a guess, `wire.rs:59-67`) so the server floor matches the client UI.
> - `BMStartWatchingItem` / `BMStopWatchingItem` are still stubs. Implement or refuse them with feedback.
> - Transport replay of bids depends on #477. The `BID_TOO_LOW` floor covers the same-amount replay.
> - The BM stub on `main` decodes `auction_length` as i32. The client sends 1 byte (CAT-I-02 wire note). #586 replaces it, so don't fix the stub separately.

---

## #459 — [security-audit] Server-authority / anti-cheat / anti-replay full sweep — 177 findings across 15 categories

- Verdict: REWRITE (a slim live index; drop the stale copy of the executive summary)
- Priority: P2
- Labels: remove `documentation`, keep `security`
- Summary: The umbrella's content is stale in three ways. (1) Every link points at the `worktree-server-authority-audit` branch, but the audit is now on main under `docs/security-audit/` with a README index and a status-banner convention. (2) Three of the five P0s are closed: #475 (access_level into cell dispatch, PR #512), #478 (movement validation, PR #522) and #479 (dialog-open gate, PR #513). #476 (TLS) and #477 (IV/replay) are open, and both are only partly addressed, by opt-in TLS and Mercury v2, which has not been validated against a live client. (3) Several P1 items are done: C-03 (#514), D-01 (#520), D-02 partly (#515), CAT-O-01 (address book, see #474). The "P2 guard-rails for stubbed handlers" framing is also out of date, because trade (#438) shipped and mail/BM/crafting are moving into their implementation issues. It is still worth having one GitHub-side index of which category and P0 issues remain open. That is only true if the body stays short and doesn't repeat the doc. Otherwise close it in favour of `docs/security-audit/README.md`.
- Evidence: `docs/security-audit/README.md` and `docs/security-audit/2026-05-31-server-authority/UMBRELLA.md` on main; `gh issue view` states: #475 CLOSED 2026-06-12, #478 CLOSED 2026-06-19, #479 CLOSED 2026-06-12, #476/#477 OPEN; merged PRs #512, #513, #514, #515, #520, #522, #438, #738, #698 (`merged-prs.tsv`); this batch's verdicts for #460-#468.
- Related/duplicates: #460-#479; stray duplicates of audit findings: #443 and #63 (= B-01, fixed by #522), #165 (= D-01, fixed by #520)

### Action text

**Comment:**
> Status refresh (2026-09-25, re-verified on `main` @ 059d6038). The audit now lives on `main` at `docs/security-audit/2026-05-31-server-authority/` (index: `docs/security-audit/README.md`), so the branch links in this body are replaced. P0s: #475, #478 and #479 are closed. #476 (TLS) and #477 (IV and inbound replay) remain, and both have partial opt-in mitigations only. Category status after re-verification: CAT-C and CAT-E hold the live P1 exploits, namely respawn/callForAid on a live player, casting while stunned, free repair, and a newly found free-buyback-through-MoveItem dupe. Trade (CAT-H) shipped in #438 and has one residual. Mail (CAT-G) and Black Market (CAT-I) were folded into #72 and #571. The crafting stubs (CAT-F-02..07) were folded into #567. The body below becomes a short index only. The per-finding detail stays in the docs.

#### New body

```markdown
Umbrella index for the 2026-05-31 server-authority / anti-cheat / anti-replay audit (177 findings, 15 categories).

**Source of truth:** [`docs/security-audit/2026-05-31-server-authority/`](../tree/main/docs/security-audit/2026-05-31-server-authority). Findings are point-in-time; status is layered on as dated banners per [`docs/security-audit/README.md`](../blob/main/docs/security-audit/README.md). This issue only tracks which GitHub issues are still open.

## P0 foundational
- [x] #475 — access_level plumbed into cell dispatch + GM gate (#512)
- [ ] #476 — SOAP auth over TLS (TLS listener is opt-in; plain HTTP still default)
- [ ] #477 — per-packet IV + inbound replay dedup (Mercury v2 opt-in and not live-validated; no inbound dedup)
- [x] #478 — server-side movement validation on 0x03 (#522)
- [x] #479 — DialogButtonChoice open-dialog gate (#513; offered-dialog set in #770)

## Categories (status as of 2026-09-25)
- [ ] #460 CAT-A Auth / session: hardening leftovers (ticket TTL at consume, rate limit, reserved names)
- [ ] #461 CAT-B Movement: B-01/04/06/09 fixed; ring caller gate, unstuck, replay remain
- [ ] #462 CAT-C Combat: **P1**, respawn/callForAid on a live player, casting while stunned, LoS, setTargetID
- [ ] #463 CAT-D Inventory: D-01 fixed; loot ownership (needs design), ammo-change fail-open (PR #602)
- [ ] #464 CAT-E Vendor: **P1**, free repair via a missing field, free buyback via MoveItem
- [ ] #465 CAT-F Crafting: trainAbility trainer gate; stub requirements moved to #567
- [x] #466 CAT-G Mail: moved to #72 (handlers are stubs)
- [ ] #467 CAT-H Trade: shipped in #438; one residual (offered items not locked)
- [x] #468 CAT-I Black Market: moved to #571 / PR #586
- [ ] #469 CAT-J Mission / dialog
- [ ] #470 CAT-K Minigame
- [ ] #471 CAT-L Chat / contact
- [ ] #472 CAT-M Organization / squad / duel
- [ ] #473 CAT-N GM / debug
- [ ] #474 CAT-O World / space / gate / ring

(Tick a category when its issue closes. Check the #469-#474 boxes against their own triage.)

## Systemic rules for new handlers (from the audit, still binding)
1. Any wire surface that is exposed but stubbed must refuse with feedback, or carry its validation contract in its implementation issue before the body is written.
2. Response messages with no correlation id (dialog, duel, org invite, strike team) need pending state held per session on the server.
3. Anything the client names (item, target, respawner, region, ring, gate) is resolved against state the server holds for *this* caller.
```

---

## Batch summary

| # | verdict | priority | one-line reason |
|---|---|---|---|
| 459 | REWRITE | P2 | Links point at the audit branch (docs are on main now); 3/5 P0s closed; slim it to a status index |
| 460 | REWRITE | P2 | TLS and Mercury v2 landed but are opt-in; IP binding is warn-only (#738); dev mode off by default; TTL, rate-limit, reserved-name and replay items still open |
| 461 | REWRITE | P2 | B-01/04/06/09 fixed (#522, region containment gate); ring caller gate (B-03), unstuck stub, replay remain |
| 462 | REWRITE | P1 | C-03/C-10 fixed; respawn/callForAid on a live player (heal + any-respawner teleport) and casting while stunned (now live) still open |
| 463 | REWRITE | P2 | D-01 fixed (#520); D-04/07/09 wrong premise; loot ownership (D-03) needs design; D-06 fix in PR #602 needs a rebase |
| 464 | REWRITE | P1 | Nothing fixed; free repair via a missing field (E-01) plus NEW free-buyback-through-MoveItem dupe |
| 465 | REWRITE | P2 | trainAbility trainer gate (F-01) and base level recheck (F-08) are live; F-02..07 are latent stubs, fold into #567 |
| 466 | CLOSE (not planned) | P3 | Mail send/take/COD are still stubs; fold all into #72 (and correct #72's false "send works" triage) |
| 467 | REWRITE | P2 | Trade shipped in #438 and closes 9/10; only offered-item locking during the session (bait-and-switch) remains |
| 468 | CLOSE (not planned) | P3 | BM is a stub on main; implementation is PR #586 (CONFLICTING); fold the residuals (search LIMIT, fee/cap) into #571 |
