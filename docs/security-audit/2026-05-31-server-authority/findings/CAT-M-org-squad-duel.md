# CAT-M — Organization / Squad / Duel

**Overall trust posture: pre-implementation.** None of the eighteen wire
surfaces in this category are implemented server-side. The cell-method
handlers in `cell/cell_methods/organization.rs` (indices 8–19) and in
`cell/cell_methods/player/social.rs` (`ORG_CREATION`=94, `SEND_DUEL_RESPONSE`=102,
`DUEL_FORFEIT`=103) consume the args bytes, emit `tracing::info!("UNIMPLEMENTED: ...")`,
and return `true` (success). The base-method dispatch in `base/dispatch.rs`
has no arm for `organizationInvite` / `organizationKick` / `organizationRankChange`
/ `sendDuelChallenge` — all four BaseMethods marked `<Exposed/>` on
`SGWPlayer.def` and `OrganizationMember.def` land in the catch-all `warn!`
and silently return `Ok(())`.

This means the **entire category is the same shape as CAT-G (mail) and
CAT-I (black market): a fully-defined client wire surface with zero
server-side validation**. The exploits below are not "the server is
doing the wrong thing" — they are "the server is doing nothing, and on
the day someone wires it up, the trust posture is wide open by default
unless every one of these is on the implementer's checklist."

Wire shapes are extracted from `entities/defs/interfaces/OrganizationMember.def`
and `entities/defs/SGWPlayer.def` and cross-referenced against Ghidra
`Event_NetOut_*` RTTI strings (which confirm the client emits these
exact classes). The .def is authoritative for arg ordering because the
2009 BigWorld build of the SGW client and server both consume it via
the entity codegen.

The findings ordering: organization creation/lifecycle first (CAT-M-01
through CAT-M-04), then rank/permission destructive ops (-05 through
-08), then guild bank (-09), then text fields (-10), then squad (-11),
then duel (-12 through -15), then PvP (-16).

---

### CAT-M-01 — `organizationInvite` (base): no server-side org membership/permission check, but no base handler at all

**Severity**: High
**Class**: Trust violation by omission (caller permission, target state)
**Wire surface**: `Event_NetOut_OrganizationInvite` → BaseMethod `organizationInvite(INT32 aOrganizationId, WSTRING aPlayerName)`
**Demonstrable / Likely-theoretical**: Likely-theoretical (no current handler — the trust violation will land on implementation)

**Trust violation**
Client supplies `(aOrganizationId, aPlayerName)`. The handler must verify:
(a) the calling session is actually a member of `aOrganizationId`,
(b) the caller's rank in that org has `EORG_PERM_Invite` (value 2 in `enumerations.xml:1911`),
(c) `aPlayerName` resolves to a real player not already in an org of the same type,
(d) the target player isn't on an ignore/block list for this caller.
Today the server has no `organizationInvite` base-method arm at all — it
hits the catch-all in `base/dispatch.rs:333-347` and returns `Ok` with a
warn log, so the client UI proceeds as if the invite went out but no
state mutated. When the handler is wired up, the implementer must not
trust `aOrganizationId` (the caller might not be in it) or assume the
client's UI gated the invite-permission check.

**Evidence**
- Ghidra: `0x019be900` `Event_NetOut_OrganizationInvite` (RTTI string); registered at `register_NetOut_OrganizationInvite` callsites in the OrganizationMember NetOut bundle.
- Wire shape from `entities/defs/interfaces/OrganizationMember.def:421-425` (BaseMethods, `<Exposed/>`).
- Permission bit from `entities/defs/enumerations.xml:1911` (`EORG_PERM_Invite`).
- Client behavioral log: n/a
- Cross-ref to Rust (no handler today): `crates/services/src/base/dispatch.rs:333` (catch-all `warn!` arm).

**Attack scenario**
1. Attacker is in org A (low-rank, no invite perm) but sends `organizationInvite(orgId = B, playerName = "victim")` for org B he doesn't belong to.
2. Server-side org-state mutation must reject: not-a-member or not-permitted.
3. Without that check, on first implementation the attacker fills any org's roster with garbage invites.

**Suggested remediation (one line)**
Server-side validate `caller_session.org_id == aOrganizationId && caller_rank_permissions & EORG_PERM_Invite != 0` before issuing any invite-state mutation.

**Would benefit from x64dbg trace?**
No — wire shape is fully decoded from the .def + the RTTI string confirms the emit.

---

### CAT-M-02 — `organizationInviteByType`: client picks org type, can implicit-create

**Severity**: Medium
**Class**: Privilege escalation via implicit-create + trust on caller-supplied type
**Wire surface**: `Event_NetOut_OrganizationInviteByType` → BaseMethod `organizationInviteByType(UINT8 aOrganizationType, WSTRING aPlayerName)`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
This method is described in the .def comment as: *"If the member is not
in a group of that type it will be created automatically"*
(`OrganizationMember.def:427`). That means the implicit-create path
runs without an `OrganizationCreation`-shaped flow — no name uniqueness
check, no founder fee, no validation that the requesting player is
eligible to create an org of the given type. The type field is `UINT8`
with no range validation in the def — only `EORG_TYPE_Squad=0`,
`EORG_TYPE_Team=1`, `EORG_TYPE_Command=2` are defined
(`enumerations.xml:1942-1944`). A client supplying type=255 must be
rejected, not implicit-created as type-255.

**Evidence**
- Ghidra: `0x019be920` `Event_NetOut_OrganizationInviteByType`.
- Wire shape from `entities/defs/interfaces/OrganizationMember.def:428-432`.
- Enum values from `entities/defs/enumerations.xml:1939-1946`.
- Cross-ref to Rust (no handler today): catch-all in `base/dispatch.rs`.

**Attack scenario**
1. Attacker emits `organizationInviteByType(type=2 [Command], playerName=bot)` repeatedly with rotating bot names.
2. Each invocation implicit-creates a new Command-rank organization owned by the attacker, bypassing any creation fee or cooldown.
3. Resource exhaustion on the org table plus possible naming/role abuse.

**Suggested remediation (one line)**
Range-check `aOrganizationType ∈ {0,1,2}` and route implicit-create through the same flow as `onOrganizationCreation` (fee + name uniqueness + faction check).

**Would benefit from x64dbg trace?**
No — surface is fully described in the .def.

---

### CAT-M-03 — `onOrganizationCreation` (cell method 94): name-only, no founder fee/faction/uniqueness validation

**Severity**: High
**Class**: Missing economic + integrity checks on a creation flow
**Wire surface**: `onOrganizationCreation(WSTRING aOrganizationName)` (cell method index 94)
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
Client supplies the new organization's name only. The server must
authoritatively enforce: (1) founder fee deducted from the player's
naqahdah balance (or refused if insufficient — pattern from CAT-G mail
COD attachment limits); (2) name uniqueness across all orgs of that
type; (3) name length + Unicode whitelist (the field is `WSTRING` with
no bound on the wire); (4) faction binding (cross-faction guilds, if
disallowed in design, must be enforced server-side, not by UI). The
current handler at `cell_methods/player/social.rs:62` is a one-line
log stub that does none of these.

**Evidence**
- Ghidra: `0x0195fb88` `Event_NetOut_OrganizationCreation` (RTTI string).
- Wire shape from `entities/defs/SGWPlayer.def:877-880`.
- Cross-ref to Rust: `crates/services/src/cell/cell_methods/player/social.rs:61-64`.

**Attack scenario**
1. Attacker sends `onOrganizationCreation("Х" * 100000)` (huge WSTRING).
2. With no length cap, server-side org-name allocation explodes; org list serialization to other clients leaks the giant string into every roster query.
3. With no fee, attacker enumerates thousands of orgs at zero cost to squat names or DoS the org table.

**Suggested remediation (one line)**
On implementation: deduct founder fee + length-cap WSTRING server-side (mirror `MAIL_*` rank constants for analogous content limits in `enumerations.xml:1055-1062`) + unique-constraint on `(org_type, name)` in the DB row insert.

**Would benefit from x64dbg trace?**
No — wire shape is fully defined and current handler is a stub.

---

### CAT-M-04 — `organizationLeave`: org_id from client, no membership check

**Severity**: Low
**Class**: Trust on client-supplied org_id
**Wire surface**: `organizationLeave(INT32 aOrganizationId)` — cell method `LEAVE`=9 on the OrganizationMember interface
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
The org_id the player is "leaving" comes from the wire. The server's
session-tracked org membership (`SGWPlayer.organizationMember.records`
in the Python reference; not yet in Rust) is the authority. A handler
that takes `org_id` at face value would let a client mass-leave any
org_id (deleting the *server's record* that this player is in that
org, even if they aren't, breaking invariants like the leader-elect
flow if `aOrganizationId` happens to match an org with a recent
leadership transfer race). The current handler at
`cell_methods/organization.rs:41-47` logs and returns.

**Evidence**
- Ghidra: `0x019be970` `Event_NetOut_OrganizationLeave`.
- Wire shape from `entities/defs/interfaces/OrganizationMember.def:286-289`.
- Cross-ref to Rust: `crates/services/src/cell/cell_methods/organization.rs:41-47`.

**Attack scenario**
1. Attacker sends `organizationLeave(aOrganizationId = victim's org)`.
2. Naïve handler dereferences and removes the row keyed by `(session.player_eid, aOrganizationId)`.
3. If the player happens to *be* in that org for some other type slot, or if the join-pending flag races a leave, the org's roster invariant is broken.

**Suggested remediation (one line)**
Cross-check `aOrganizationId` against the session-tracked membership list before any mutation; reject silently if mismatch.

**Would benefit from x64dbg trace?**
No.

---

### CAT-M-05 — `organizationKick` (base): no rank-comparison invariant

**Severity**: High
**Class**: Privilege escalation via missing rank-compare
**Wire surface**: `Event_NetOut_OrganizationKick` → BaseMethod `organizationKick(INT32 aOrganizationId, WSTRING aPlayerName)`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
The kick operation requires two server-side comparisons that the wire
shape forces the server to compute itself: (a) caller has
`EORG_PERM_Eject` (value 16, `enumerations.xml:1914`), and (b)
target's rank is **strictly lower** than caller's rank. Without the
strict-lower check, an Officer with Eject perm could kick the Leader.
The error enum `EDB_ERROR_Player_rank_too_low = -20072`
(`enumerations.xml:1954`) signals that the original Python reference
*did* enforce this — when implementing the Rust handler, the same
invariant must reappear or the leader can be self-evicted. No base
handler today.

**Evidence**
- Ghidra: `0x019be990` `Event_NetOut_OrganizationKick`.
- Wire shape from `entities/defs/interfaces/OrganizationMember.def:435-439`.
- Permission bit `EORG_PERM_Eject` from `enumerations.xml:1914`.
- Rank ordering from `enumerations.xml:1892-1905` (`EORG_RANK_Initiate`=1 .. `EORG_RANK_Leader`=8).
- Cross-ref to Rust (no handler today): catch-all in `base/dispatch.rs:333`.

**Attack scenario**
1. Officer (rank 6) emits `organizationKick(orgId, "Leader's name")`.
2. With only `EORG_PERM_Eject` checked but no rank-compare, the kick succeeds.
3. Leader is removed from their own guild; remaining-leader-promotion logic may promote the attacker.

**Suggested remediation (one line)**
Require `caller_rank > target_rank` AND `caller_perms & EORG_PERM_Eject != 0` before any roster mutation; return the existing `EDB_ERROR_Player_rank_too_low` on mismatch.

**Would benefit from x64dbg trace?**
No.

---

### CAT-M-06 — `organizationRankChange` (base): can promote above own rank if not enforced

**Severity**: High
**Class**: Privilege escalation via missing rank-cap
**Wire surface**: `Event_NetOut_OrganizationRankChange` → BaseMethod `organizationRankChange(INT32 aOrganizationId, WSTRING aPlayerName, UINT8 aRank)`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
Three invariants the server must enforce for rank change: (a) caller
has `EORG_PERM_Promote` (=4) or `EORG_PERM_Demote` (=8) depending on
delta direction; (b) the requested `aRank` is strictly less than the
caller's own rank (else any officer could promote anyone to Leader);
(c) the target's *current* rank is also strictly less than the
caller's rank (you can't reshuffle equals or seniors). `aRank` is
`UINT8` on the wire — no enum guard. Without a range check on the
enum tokens (`EORG_RANK_None=0 .. EORG_RANK_Leader=8`,
`enumerations.xml:1892-1905`), clients can send `aRank = 255` and
either crash a switch-on-rank somewhere or store an out-of-band rank
that breaks UI rendering. No base handler today.

**Evidence**
- Ghidra: `0x019be9b0` `Event_NetOut_OrganizationRankChange`.
- Wire shape from `entities/defs/interfaces/OrganizationMember.def:442-447`.
- Cross-ref to Rust (no handler today): catch-all in `base/dispatch.rs:333`.

**Attack scenario**
1. Senior Member (rank 3) with `EORG_PERM_Promote` emits `organizationRankChange(orgId, "alt account", aRank=8 [Leader])`.
2. Without rank-cap, alt becomes Leader.
3. Original Leader kicked next.

**Suggested remediation (one line)**
Require `aRank ∈ [1,8]` AND `aRank < caller_rank` AND `target_current_rank < caller_rank` before any rank mutation.

**Would benefit from x64dbg trace?**
No.

---

### CAT-M-07 — `organizationSetRankPermissions`: arbitrary permissions bitfield

**Severity**: High
**Class**: Privilege escalation via mask-not-validated
**Wire surface**: `organizationSetRankPermissions(INT32 aOrganizationId, INT32 aRank, INT32 aPermissions)` — cell method `SET_RANK_PERMISSIONS`=16
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
`aPermissions` is a raw `INT32` on the wire that becomes the rank's
permission bitfield. The valid mask is defined in
`EOrganizationPermission` (`enumerations.xml:1907-1937`) — top defined
bit is `EORG_PERM_AllianceCmds = 33554432` (0x2000000). Any handler
that stores the client value as-is would let a client set bits that
aren't in the union (potential UB if those bits collide with future
flags) or set high-impact bits like `EORG_PERM_AlterPerms` (=8388608)
on a low rank, which would then let that low rank further escalate.
The caller must (a) themselves have `EORG_PERM_AlterPerms`, (b) the
target rank must be strictly lower than caller's rank, (c)
`aPermissions` must be a subset of the union of defined bits, (d) the
caller cannot grant permissions they themselves do not hold
(otherwise an officer with AlterPerms but without WithdrawBank can
grant WithdrawBank to a subordinate). Current handler stub:
`cell_methods/organization.rs:112-126`.

**Evidence**
- Ghidra: `0x019bea3c` `Event_NetOut_OrganizationSetRankPermissions`.
- Wire shape from `entities/defs/interfaces/OrganizationMember.def:390-395`.
- Permission mask values from `enumerations.xml:1907-1937`.
- Cross-ref to Rust: `crates/services/src/cell/cell_methods/organization.rs:112-126`.

**Attack scenario**
1. Officer with `EORG_PERM_AlterPerms` sends `organizationSetRankPermissions(orgId, rank=1 [Initiate], permissions=0xFFFFFFFF)`.
2. Initiate rank now has every permission, including `EORG_PERM_TransferLeader = 16777216` and `EORG_PERM_WithdrawCash`.
3. Officer's alt characters at Initiate rank now have full control.

**Suggested remediation (one line)**
Mask `aPermissions` against the union of defined `EOrganizationPermission` bits, require caller's permission set is a superset of `aPermissions`, require `caller_rank > aRank`.

**Would benefit from x64dbg trace?**
No.

---

### CAT-M-08 — `organizationSetRankName`: text-length not capped, no permission check

**Severity**: Medium
**Class**: Missing input cap + missing permission check
**Wire surface**: `organizationSetRankName(INT32 aOrganizationId, INT32 aRank, WSTRING aName)` — cell method `SET_RANK_NAME`=17
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
`aName` is unbounded WSTRING. Caller permission required: `EORG_PERM_RankNames` (=128, `enumerations.xml:1917`). Rank value must be in `[1,8]`. Without a server-side length cap, this is a roster-wide DoS vector — every member's UI will render the long string each time the org rank list updates. Current handler stub: `cell_methods/organization.rs:127-138`.

**Evidence**
- Ghidra: `0x019bea68` `Event_NetOut_OrganizationSetRankName`.
- Wire shape from `entities/defs/interfaces/OrganizationMember.def:397-402`.
- Permission bit `EORG_PERM_RankNames` from `enumerations.xml:1917`.
- Cross-ref to Rust: `crates/services/src/cell/cell_methods/organization.rs:127-138`.

**Attack scenario**
1. Officer with `EORG_PERM_RankNames` (or anyone, if the perm isn't checked) sends `organizationSetRankName(orgId, rank=8, aName="X" * 65535)`.
2. Server stores the giant string; broadcasts it via `onOrganizationRankNameUpdate` to every org member.
3. Client UI hangs rendering 65k-char rank label on every roster open.

**Suggested remediation (one line)**
Length-cap `aName` server-side (e.g. 32 chars matching typical UI box) and require `caller_perms & EORG_PERM_RankNames != 0 && caller_rank > aRank`.

**Would benefit from x64dbg trace?**
No.

---

### CAT-M-09 — `organizationTransferCash`: client picks amount + direction with no rank-bound withdraw cap

**Severity**: Critical
**Class**: Guild bank dupe / privilege escalation
**Wire surface**: `organizationTransferCash(INT32 aOrganizationId, INT32 aCash)` — cell method `TRANSFER_CASH`=19
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
`aCash` is `INT32` — a *signed* 32-bit integer. The handler must:
(a) reject `aCash <= 0` (signed-overflow trick: aCash=INT_MIN with a
naïve subtraction wraps into a deposit); (b) require `caller_perms`
contains `EORG_PERM_DepositCash` (=262144) for positive transfers and
`EORG_PERM_WithdrawCash` (=524288) for negative ones — these are
distinct bits in `enumerations.xml:1928-1929`; (c) atomically debit
caller's wallet and credit org bank (or vice versa) in a single DB
transaction with rollback on disconnect; (d) per-rank withdraw
**cap** — most MMO guild-bank designs gate withdraw amount by rank.
The .def has no per-rank cap field, so the implementer must derive it
from a server config or store it in the org row. Current handler
stub: `cell_methods/organization.rs:147-159` parses
`org_id, cash = i32::from_le_bytes(...)` and logs.

**Evidence**
- Ghidra: `0x019bea90` `Event_NetOut_OrganizationTransferCash`.
- Wire shape from `entities/defs/interfaces/OrganizationMember.def:409-413`.
- Permission bits from `enumerations.xml:1928-1929`.
- Cross-ref to Rust: `crates/services/src/cell/cell_methods/organization.rs:147-159`.

**Attack scenario**
1. Member (no `EORG_PERM_WithdrawCash`) sends `organizationTransferCash(orgId, aCash=-1000000)` interpreting the negative as a withdraw.
2. Naïve handler: `player.cash -= aCash` → player.cash *increases* by 1M; org bank decreases by 1M (or wraps if no withdraw-perm check).
3. Dupe of guild bank into personal naqahdah.

Atomicity scenario:
1. Member with `EORG_PERM_DepositCash` sends `organizationTransferCash(orgId, aCash=1000)`; player wallet debited; client disconnects mid-transaction.
2. If org-bank credit hasn't committed and disconnect rolls back wrong-side, 1000 cash is destroyed (refund-amount-not-credited) or duplicated (debit not committed but credit was).

**Suggested remediation (one line)**
`aCash > 0` only (direction inferred from a separate verb if both directions needed), `caller_perms & deposit_or_withdraw_bit != 0`, single-transaction wallet↔bank mutation with `with_item_lock`-style row pinning, per-rank daily withdraw cap stored server-side.

**Would benefit from x64dbg trace?**
Yes — once implemented, a live trace through the i32 sign-extension path would confirm the wallet wraparound behavior under negative input.

---

### CAT-M-10 — `organizationMOTD` / `organizationNote` / `organizationOfficerNote`: unbounded WSTRING + permission unchecked

**Severity**: Medium
**Class**: Missing input cap + missing permission check
**Wire surface**:
- `organizationMOTD(INT32 aOrganizationId, WSTRING aMOTD)` — cell `MOTD`=13
- `organizationNote(INT32 aOrganizationId, WSTRING aNote)` — cell `NOTE`=14
- `organizationOfficerNote(INT32 aOrganizationId, WSTRING aName, WSTRING aNote)` — cell `OFFICER_NOTE`=15

**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
All three text-mutation cell methods accept unbounded WSTRINGs.
`organizationMOTD` requires `EORG_PERM_MOTD` (=1024,
`enumerations.xml:1920`); `organizationNote` requires
`EORG_PERM_RosterNotes` (=32); `organizationOfficerNote` requires
`EORG_PERM_OfficerNotes` (=64). All three rebroadcast via the
matching `on*Update` cell method to every member's client. The
current handlers at `cell_methods/organization.rs:91-111` only log
`org_id` from the prefix bytes — the actual WSTRING contents are not
even parsed. The OFFICER_NOTE case is the worst — it takes a
`WSTRING aName` field which, if used to look up which member's
officer-note to edit, would let a non-officer edit anyone's officer
note by spoofing the name (the actual target authority is the
member-id resolved from the name, and that resolution must enforce
caller's officer-note-edit permission).

**Evidence**
- Ghidra: `0x019be9d4` (`MOTD`), `0x019be9f4` (`Note`), `0x019bea14` (`OfficerNote`).
- Wire shapes from `entities/defs/interfaces/OrganizationMember.def:371-388`.
- Permission bits from `enumerations.xml:1915-1920`.
- Cross-ref to Rust: `crates/services/src/cell/cell_methods/organization.rs:91-111`.

**Attack scenario**
1. Initiate with no `EORG_PERM_MOTD` sends `organizationMOTD(orgId, "X" * 65535)`.
2. Naïve handler stores + rebroadcasts to all online members. Every member's UI hangs on render.
3. Repeat for `organizationOfficerNote` with name = victim's name to vandalize their HR note.

**Suggested remediation (one line)**
Each handler: validate caller's permission bit, length-cap the WSTRING (e.g. 256 chars for MOTD, 128 for note), resolve target name → member_id server-side and re-check permission against that target's rank for the officer-note path.

**Would benefit from x64dbg trace?**
No.

---

### CAT-M-11 — `squadSetLootMode`: no leader check, no enum range check

**Severity**: Medium
**Class**: Missing caller authority + missing enum range check
**Wire surface**: `squadSetLootMode(INT32 aLootMode)` — cell method `SQUAD_SET_LOOT_MODE`=18
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
The .def at `OrganizationMember.def:404-407` is striking: the wire
shape carries **no `aOrganizationId`** — only the loot mode value.
That means the server must resolve "the squad this player is in"
entirely from session state. The handler at
`cell_methods/organization.rs:140-146` just parses
`loot_mode = i32::from_le_bytes(...)` and logs. When implemented,
the implementer must:
(a) look up the player's squad org (type=Squad, =0 in
`enumerations.xml:1942`) from session;
(b) verify caller is the squad leader (`EORG_RANK_Leader = 8` in this
context, or however squad-leader is encoded for the EORG_TYPE_Squad
case);
(c) range-check `aLootMode` against the valid enum (the .def comment
says it's `aLootType` on the client side at
`OrganizationMember.def:172-175`, broadcast via `onSquadLootType`, but
the actual enum tokens for valid loot modes are not in
`enumerations.xml` — they may live in a different schema file and
must be enumerated before implementing). Without (c) the client can
send a negative or out-of-bound value that stores in the squad row
and breaks the loot-distribution code on the next mob kill.

**Evidence**
- Ghidra: `0x019beab8` `Event_NetOut_SquadSetLootMode`.
- Wire shape from `entities/defs/interfaces/OrganizationMember.def:404-407`.
- Note: no aOrganizationId on the wire — server-side session lookup mandatory.
- Cross-ref to Rust: `crates/services/src/cell/cell_methods/organization.rs:140-146`.

**Attack scenario**
1. Squad member (not leader) sends `squadSetLootMode(aLootMode=99)`.
2. Naïve handler stores 99 as the squad's loot mode.
3. Next mob kill, loot-distribution code switches on a value outside its match arms → either falls through to a default (e.g. FFA, which the non-leader prefers) or panics.

**Suggested remediation (one line)**
Resolve squad from session, require caller is squad-leader, range-check `aLootMode` against the defined loot-mode enum.

**Would benefit from x64dbg trace?**
Yes — to enumerate the valid loot-mode values that the SGW client UI actually sends, since the enum isn't in `enumerations.xml`.

---

### CAT-M-12 — `sendDuelChallenge` (base): no online/same-space/cooldown check, target by name

**Severity**: High
**Class**: Missing precondition checks on a state-machine transition
**Wire surface**: `Event_NetOut_DuelChallenge` → BaseMethod `sendDuelChallenge(WSTRING aPlayerName, INT8 aSquadDuel)`
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
The duel state machine transition from "no duel" → "pending challenge"
needs server-side gating on:
(a) target player exists and is online;
(b) target is in the same space (cross-space duels would teleport-glitch the duel marker);
(c) target is within a sane challenge-range (movement-physics-advisor's domain — server-side position distance check);
(d) caller is not already in a duel or pending-challenge state;
(e) target is not already in a duel;
(f) caller's PvP flag and target's PvP flag are compatible per the design;
(g) cooldown on issuing challenges (anti-spam — without it, a script can spam-challenge every nearby player every tick).
The `aSquadDuel` flag elevates the challenge to "my entire squad vs their entire squad" — that fan-out makes the precondition checks more important, not less. No base handler today.

**Evidence**
- Ghidra: `0x019b4448` `Event_NetOut_DuelChallenge`; `register_NetOut_DuelChallenge` at `0x00cbee10`; SlashCmd entry at `Event_SlashCmd_DuelChallenge__vfunc_2 @ 0x005b3b10` (client UI emit).
- Wire shape from `entities/defs/SGWPlayer.def:509-513`.
- Cross-ref to Rust (no handler today): catch-all in `base/dispatch.rs:333`.

**Attack scenario**
1. Script: loop {emit `sendDuelChallenge("victim", aSquadDuel=1)`; sleep 50ms; emit `duelForfeit`}.
2. With no anti-spam, victim's UI receives a duel-challenge prompt 20×/sec forever.
3. Or: emit `sendDuelChallenge(...)` cross-space against a player in a safe zone, dragging them into a duel state where the PvP flag forced-on causes them to take damage from PvP-flagged players.

**Suggested remediation (one line)**
Resolve target name → entity_id server-side, validate online/same-space/range/state-machine compatibility/anti-spam cooldown; route to a dedicated duel-state-machine module rather than the catch-all.

**Would benefit from x64dbg trace?**
Yes — to confirm the challenge-acceptance UI prompt the target sees can be replayed verbatim by spoofing `onDuelChallenge` client-method indices.

---

### CAT-M-13 — `sendDuelResponse`: no validation that a challenge is pending for this session

**Severity**: High
**Class**: Replay / state-machine bypass
**Wire surface**: `sendDuelResponse(INT8 aResponse)` — cell method `SEND_DUEL_RESPONSE`=102
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
This is the response side of the duel state machine. The wire shape
carries **only `aResponse` (INT8)** — no challenger id, no challenge
nonce. That means the server must hold the pending-challenge state
per-session and consume it on response. If the handler dispatches
without checking "is there a pending challenge for me right now",
then:
(a) a client that never received a challenge can spam
`sendDuelResponse(1)` to force-start duels against arbitrary players;
(b) once a duel ends, a delayed `sendDuelResponse` packet could
re-trigger the duel-start path; (c) without a nonce, the response
can't be cross-checked against *which* challenge is being answered if
multiple pile up. The fan-out shape `Event_NetIn_onDuelChallenge` has
`MAILBOX + ARRAY<MAILBOX>` (`SGWPlayer.def:1372-1375`) — the server
holds the challenger info, but it must verify the response correlates
to a known-pending challenge. Current handler stub:
`cell_methods/player/social.rs:90-96`.

**Evidence**
- Ghidra: `0x0195fb58` `Event_NetOut_DuelResponse` (sic — RTTI string says DuelResponse but the cell method on the .def is `sendDuelResponse`); `register_NetOut_DuelResponse` callsites confirm.
- Wire shape from `entities/defs/SGWPlayer.def:975-978`.
- Cross-ref to Rust: `crates/services/src/cell/cell_methods/player/social.rs:90-96`.
- This is the same exploit shape as the [[reference-dialog-choice-exploit-shape]] (no open-dialog tracking → replay forgeable).

**Attack scenario**
1. Without ever receiving a duel challenge, send `sendDuelResponse(aResponse=1)`.
2. Server has no `pending_duel_challenge` map → naïve handler may default-accept or, worse, look up "current duel partner" from a stale field.
3. If the server stores "last challenger" as a side-effect of receiving a challenge but doesn't clear it on timeout, a delayed accept reactivates a stale challenge.

**Suggested remediation (one line)**
Per-session `pending_duel_challenge: Option<{challenger_eid, expires_at, nonce}>`, populated on receipt of a challenge, consumed (cleared) on response or timeout, and required to be `Some` for the response to do anything.

**Would benefit from x64dbg trace?**
No — wire and Python reference are clear about the missing state.

---

### CAT-M-14 — `duelForfeit`: no caller-is-in-duel check, no participant check

**Severity**: Medium
**Class**: Missing participant check on a state-machine transition
**Wire surface**: `duelForfeit()` — cell method `DUEL_FORFEIT`=103 (no args)
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
The wire carries zero arguments — caller identity is from
`session.player_eid` (good — already enforced by
[[reference-cell-method-entity-id-authority]]). The server must
verify the caller is actually a participant of a currently-active
duel. Without this:
(a) any player can `duelForfeit` while not in a duel, triggering the
"end of duel" cleanup path (clearing PvP flags, removing the duel
marker, awarding the "winner" — depending on how the duel end is
designed, this could grant credit to a third party);
(b) in a squad duel (squad vs squad), a non-participant squad-member
forfeit could end the duel for everyone on their side.
Current handler stub: `cell_methods/player/social.rs:98-101`.

**Evidence**
- Ghidra: `0x019b4478` `Event_NetOut_DuelForfeit`; `register_NetOut_DuelForfeit` at `0x00cbefc0`; SlashCmd entry at `Event_SlashCmd_DuelForfeit__vfunc_2 @ 0x005b4010`.
- Wire shape from `entities/defs/SGWPlayer.def:1015-1017`.
- Cross-ref to Rust: `crates/services/src/cell/cell_methods/player/social.rs:98-101`.

**Attack scenario**
1. Player A (not in a duel) emits `duelForfeit`.
2. Naïve handler runs end-of-duel cleanup against a default/stale duel record, possibly clearing PvP flags or awarding XP to a phantom opponent.
3. Or: in a squad duel of A vs B, an unrelated player C (not in either squad) emits `duelForfeit` and naïvely ends A's squad's side.

**Suggested remediation (one line)**
Resolve `session.player_eid` → active duel record via the `duelEntities` array on `SGWDuelMarker.def`; reject silently if the player is not listed as a participant in any active duel.

**Would benefit from x64dbg trace?**
No.

---

### CAT-M-15 — Disconnect during duel: no auto-forfeit path

**Severity**: Medium
**Class**: State-machine disconnect-race exploit
**Wire surface**: Implicit — the lack of a disconnect handler that knows about duel state
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
This is the analogue of the [[project-trade-handlers-unimplemented]]
disconnect-timing window for duels. The duel state lives across two
players' sessions plus an `SGWDuelMarker` entity. When one
participant disconnects mid-duel:
(a) if no auto-forfeit fires, the other participant is stuck in a
duel state forever (PvP flag forced on, can't loot or interact);
(b) if death-penalty would otherwise apply to the disconnector, they
escape it by force-quitting;
(c) the `SGWDuelMarker` and its `duelEntities` array
(`SGWDuelMarker.def:15-18`) leak — the duel record never decrements
to zero participants.
The current logoff path in `base/dispatch.rs:235-327` (`LOG_OFF`)
sends `DisconnectEntity` and `DestroyEntity` messages to the cell, but
the cell side has no duel-aware handler to convert "participant
disconnected" into "remaining-participant wins by forfeit + clear
duel state". This needs to land at the same time as the duel
state-machine in CAT-M-12/-13/-14.

**Evidence**
- Wire shape (the disconnect path that needs to do this) from `base/dispatch.rs:259-275`.
- Duel state container from `entities/defs/SGWDuelMarker.def:15-18` (`duelEntities: ARRAY<of>MAILBOX`).
- No current Rust handler — finding is about the gap in the disconnect path.

**Attack scenario**
1. Attacker challenges victim to a duel; duel starts.
2. Attacker is losing, force-disconnects via `LogOff(disconnect=1)`.
3. Without an auto-forfeit: attacker avoids death penalty; victim remains stuck in duel-flagged state until manual GM cleanup.

**Suggested remediation (one line)**
On `DisconnectEntity` for a player listed in any `SGWDuelMarker.duelEntities`, fire an auto-forfeit with the remaining participant(s) as winner before destroying the entity.

**Would benefit from x64dbg trace?**
No.

---

### CAT-M-16 — `pvpOrganizationLeaveResponse`: no pending-leave-request correlation

**Severity**: Medium
**Class**: Replay / state-machine bypass
**Wire surface**: `pvpOrganizationLeaveResponse(INT32 aOrganizationId, UINT8 aResponse)` — cell method `PVP_LEAVE_RESPONSE`=12
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
This is the response leg of `onPvPOrganizationLeaveRequest`
(`OrganizationMember.def:123-126`). The server prompts the client
"changing your PvP flag would force you to leave org X, accept?"; the
client responds with `aResponse`. If the server doesn't track the
*pending* leave request per session and per org, then:
(a) a client can spam `pvpOrganizationLeaveResponse(orgId, 1)`
without ever having received a leave prompt — naïve handler may
execute the "leave on PvP flip" path against any org_id the player
happens to be in;
(b) timing race: a delayed accept after the prompt timed out should
not still process. The session must hold a `pending_pvp_leave: Option<{org_id, expires_at}>`
analogous to the duel-challenge state in CAT-M-13.
Current handler stub: `cell_methods/organization.rs:78-90`.

**Evidence**
- Ghidra: `0x019bfbec` / `0x019cb5d8` `Event_NetOut_PvPOrganizationLeaveResponse` (two RTTI string sites, NetOut typed payload + EventHandler binding).
- Wire shape from `entities/defs/interfaces/OrganizationMember.def:336-340`.
- Cross-ref to Rust: `crates/services/src/cell/cell_methods/organization.rs:78-90`.

**Attack scenario**
1. Player is in PvP org A (joined when PvP flag was on). They want to leave A without flipping PvP first.
2. Spam `pvpOrganizationLeaveResponse(orgId=A, aResponse=1)` without ever toggling PvP.
3. Naïve handler treats the response as "yes, leave A and flip PvP off" — player leaves A while never having received a server-side leave-prompt.

**Suggested remediation (one line)**
Per-session `pending_pvp_leave: Option<{org_id, expires_at}>`; populate on emitting `onPvPOrganizationLeaveRequest` to that client; consume + clear on response; require `Some` with matching `org_id` for the response to act.

**Would benefit from x64dbg trace?**
No.

---

### CAT-M-17 — `strikeTeamResponse`: same shape as PvP leave response — no pending-request correlation

**Severity**: Low
**Class**: Replay / state-machine bypass
**Wire surface**: `strikeTeamResponse(INT32 aOrganizationId, UINT8 aResponse)` — cell method `STRIKE_TEAM_RESPONSE`=11
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
The exact same shape as CAT-M-16. The server prompts via
`onStrikeTeamUpdate` (`OrganizationMember.def:323-326`), the client
responds with `aResponse`. Without a pending-strike-team-prompt map
on the session, a client can force-accept or force-decline a
strike-team transition that was never offered. Lower severity than
the PvP leave because the side effect (strike-team toggle) is narrower
than full-org-leave, but the missing-correlation shape is identical
and warrants flagging now so it's not skipped when the handler is
wired up. Current handler stub: `cell_methods/organization.rs:65-77`.

**Evidence**
- Wire shape from `entities/defs/interfaces/OrganizationMember.def:329-333`.
- Cross-ref to Rust: `crates/services/src/cell/cell_methods/organization.rs:65-77`.

**Attack scenario**
1. Member of org A (not currently being asked about strike-team) sends `strikeTeamResponse(orgId=A, aResponse=1)`.
2. Naïve handler toggles strike-team flag on A.
3. Org-wide PvP state shifts without leader consent.

**Suggested remediation (one line)**
Same pattern as CAT-M-16 — per-session pending-strike-team-prompt map, populate on emit, consume on response.

**Would benefit from x64dbg trace?**
No.

---

### CAT-M-18 — `organizationInviteResponse`: no per-session pending-invite correlation

**Severity**: Medium
**Class**: Replay / state-machine bypass + request-id forgery
**Wire surface**: `organizationInviteResponse(INT32 aRequestID, UINT8 aResponse)` — cell method `INVITE_RESPONSE`=8
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
The client passes `aRequestID` (server-generated invite token from
`onOrganizationInvite` at `OrganizationMember.def:64-70`, INT32 field
`aRequestID`). The handler must:
(a) look up the request_id in a per-session pending-invites map;
(b) verify the invite hasn't expired (TTL — without one, an invite
forwarded by mail or out-of-band can be replayed forever);
(c) confirm the invite is destined for this session's player.
Without (c), a client could harvest a request_id from a packet
capture and force-accept *anyone else's* invite. Without (a)/(b), the
client can fabricate request_id = 1, 2, 3 ... and hope to brute-force
match a real outstanding invite ID. Request IDs are INT32 — 2^32
space, but if they're assigned monotonically from 0 the search space
is much smaller and brute-force becomes practical against a noisy
server. Current handler stub: `cell_methods/organization.rs:28-40`.

**Evidence**
- Ghidra: `0x019be948` `Event_NetOut_OrganizationInviteResponse`.
- Wire shape from `entities/defs/interfaces/OrganizationMember.def:267-271`.
- Cross-ref to Rust: `crates/services/src/cell/cell_methods/organization.rs:28-40`.

**Attack scenario**
1. Attacker harvests a victim's `aRequestID` from a packet capture or by guessing the monotonic counter range.
2. Sends `organizationInviteResponse(aRequestID=harvested, aResponse=1)` from attacker's session.
3. Naïve handler that keys only on `request_id` (not on session+request_id) accepts the invite on attacker's behalf, adding attacker to the org the request was destined for.

**Suggested remediation (one line)**
Per-session `pending_invites: HashMap<i32, {org_id, inviter_eid, expires_at}>`; reject if `request_id` not in this session's map; use a CSPRNG for `request_id` so guessing is infeasible.

**Would benefit from x64dbg trace?**
Yes — to inspect how the SGW client builds `aRequestID` and whether the same value is reused across sessions.

---

## Not Filed

- **`BroadcastMinimapPing`** — categorized as Squad in CAT-L (chat) by the surface inventory; deferred to that audit. The handler stub at `cell_methods/organization.rs:48-64` accepts an arbitrary org_id + vector3 with no membership or rate-limit check, but this overlaps `sendPlayerCommunication` and other broadcast-shape exploits already filed under CAT-L.
- **`ReloadOrganizations`** — listed in CAT-M scope but the Ghidra surface inventory shows no `Event_NetOut_ReloadOrganizations` RTTI string today. Strings search confirms 0 hits. This is likely a GM/debug-only command that's part of CAT-N's `RequestReload` family; route any future implementation through the GM-auth plumbing gap [[reference-gm-auth-plumbing-gap]] rather than the OrganizationMember dispatch.
- **`onOrganizationCreationResult` / `launchOrganizationCreation`** — server→client messages (NetIn / cell→client direction), not client→server NetOut. Out of scope for an audit of client-supplied trust.
- **`awardSquadXP`** — server→client (cell method on `SGWPlayer.def:572-578`, NOT `<Exposed/>`). The Python `<Exposed/>` discriminator means the wire surface does not accept client invocation; this is a cell-internal RPC.
- **`startSquadDuel` / `duelChallenge` / `duelResponse` / `duelEntityDefeat` / `startDuel` / `registerDuelMarker` / `onDuelDefeat` / `duelAbort` / `onDuelEntitiesSet` / `onDuelEntitiesRemove` / `onDuelEntitiesClear`** — all non-`<Exposed/>` cell methods on `SGWPlayer.def:980-1013` and `1417-1426`. These are server-side inter-entity RPCs (e.g. duel marker → player) and not client-callable. Out of scope.
- **`onSquadMemberRingTransport*`** — server→client notifications, not client emits.
- **OrganizationCreation founder-fee value** — out of scope to specify, since `enumerations.xml` has no token for the fee amount. Implementer must source from server config or content data; the AUDIT item is "there must BE a fee," which is captured in CAT-M-03.
- **Strike-team / PvP-leave timer fields** (`strikeTeamTimers`, `pendingPvPTimers`, `pendingGroups`, `pendingJoins`, `pendingInvitesByType` — all `CELL_PRIVATE` properties on `OrganizationMember.def:26-57`) — these are the *correct* state for the pending-request maps recommended in CAT-M-16/-17/-18. Not a finding; surfaced here so the implementer knows the per-session state is already declared in the entity model.
- **`PythonProperty records` on OrganizationMember** — the `records` property at `OrganizationMember.def:12-16` is `PYTHON` typed, `CELL_PRIVATE`. The Python reference stored the player's organization-membership tuples here. Server-authority-critical, but not a wire-supplied field — it's authoritative server state. Not a wire-surface finding; flagged for the implementer to confirm it lives in the DB and not in the client.
