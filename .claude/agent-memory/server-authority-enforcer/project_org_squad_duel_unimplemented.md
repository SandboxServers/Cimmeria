---
name: project-org-squad-duel-unimplemented
description: Organization / Squad / Duel wire surface is fully stubbed; CAT-M audit findings as of 2026-05-31
metadata:
  type: project
---

# CAT-M (Organization / Squad / Duel) — trust posture 2026-05-31

**Status:** All 18 in-scope client wire surfaces are unimplemented.

**Why:** Implementation has not started on guild/squad/duel systems. The
cell-method handlers in `cell/cell_methods/organization.rs` (indices
8–19) and `cell/cell_methods/player/social.rs` (`ORG_CREATION`=94,
`SEND_DUEL_RESPONSE`=102, `DUEL_FORFEIT`=103) are all log-and-return-
true stubs. The four BaseMethods (`organizationInvite`,
`organizationInviteByType`, `organizationKick`, `organizationRankChange`,
`sendDuelChallenge`) have no base-dispatch arm at all — they hit the
catch-all `warn!` in `base/dispatch.rs:333-347` and silently return Ok.

**How to apply:** When auditing or implementing org/squad/duel, treat
the entire vertical as a single landing point — the same way [[project-
black-market-unimplemented]] and [[project-trade-handlers-unimplemented]]
landed. The 18 findings in `.scratch/audit/findings/CAT-M-org-squad-duel.md`
form the checklist:

- Org creation (CAT-M-03): founder fee, name uniqueness, length cap, faction binding
- Org invite/leave (CAT-M-01, -02, -04, -18): caller permission, target state, request-id correlation
- Destructive ops on roster (CAT-M-05, -06, -07, -08): rank-compare, rank-cap, perm-mask, perm-subset invariants
- Guild bank (CAT-M-09): signed-i32 dupe, atomicity, per-rank withdraw cap — **critical severity**
- Text fields (CAT-M-10): MOTD/Note/OfficerNote permission + length cap
- Squad loot (CAT-M-11): squad-leader check, enum-range check
- Duel state machine (CAT-M-12, -13, -14, -15): online/space/range/cooldown gates,
  pending-challenge correlation, participant check on forfeit, disconnect auto-forfeit
- PvP / strike-team responses (CAT-M-16, -17): pending-request correlation

**Pattern callouts:**

1. **Pending-request-correlation** is the recurring shape for invite-response,
   pvp-leave-response, strike-team-response, duel-response. The
   `OrganizationMember.def:26-57` properties (`strikeTeamTimers`,
   `pendingPvPTimers`, `pendingGroups`, `pendingJoins`, `pendingInvitesByType`)
   are exactly the per-session state needed. They already exist in the entity
   model — implementer just has to wire them into the response handlers.
2. **Permission bits in `enumerations.xml:1907-1937`** (`EOrganizationPermission`,
   UINT32 union, ~22 defined bits) are the canonical mask for every
   roster-mutation gate. Caller-perm bit + target-rank < caller-rank +
   perm-subset constraint are the three invariants on destructive ops.
3. **Wire shapes live in `entities/defs/interfaces/OrganizationMember.def`**
   (BaseMethods at lines 421-447, CellMethods at lines 179-413).
   `entities/defs/SGWPlayer.def` adds `sendDuelChallenge` (line 509),
   `onOrganizationCreation` (line 877), `sendDuelResponse` (line 975),
   `duelForfeit` (line 1015).

**Critical implementation gotcha:** `organizationTransferCash`'s `aCash`
field is signed `INT32`. The implementer **must** reject `aCash <= 0`
or a negative deposit becomes a withdraw under naïve subtraction. The
Python reference enforced this; the Rust reimplementation must too.

Links: [[reference-org-squad-duel-wire-spec]] for wire-shape anchors,
[[reference-cell-method-entity-id-authority]] for the framing-layer
entity-id guarantee, [[reference-dialog-choice-exploit-shape]] for the
analogous "no pending-prompt tracking" exploit shape that CAT-M-13/-16/
-17/-18 all share.
