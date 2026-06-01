---
name: project-gm-commands-audit-2026-05-31
description: CAT-N audit — GM/debug/cheat commands trust posture as of 2026-05-31
metadata:
  type: project
---

## CAT-N GM-commands trust posture — 2026-05-31

**Why**: User commissioned a per-category server-authority audit;
CAT-N is the largest single risk surface (~125 distinct wire
events). Findings written to
`.scratch/audit/findings/CAT-N-gm-commands.md`.

**How to apply**: When future work touches any `gm*` handler, the
SGWGmPlayer entity class enablement, or anything that plumbs
caller identity into the cell layer — reference this audit. The
core systemic finding (no access_level in cell dispatch) is the
prerequisite for safely implementing any of the GM cell methods.

### Key state

- ZERO `gm*` cell methods are implemented in Rust. Every one
  falls through to the warn! arm at
  `crates/services/src/cell/dispatch/router.rs:101`.
- `class_id` is hardcoded to SGWPlayer (0x02) regardless of
  access_level (see TODO at
  `crates/services/src/base/world_entry/play_character.rs:89-94`).
  Legitimate GMs CANNOT use GM commands today.
- Three GM-shaped methods ARE in the regular SGWPlayer flat-index
  table (so reachable today even without the SGWGmPlayer class):
  `onWorldInstanceReset` (CM 92), `resetMyAbilities` (CM 72),
  and the combat/heal debug toggles (CM 2/3/6 on
  SGWAbilityManager/SGWCombatant). All are stub handlers today.
- Two parallel GM dispatch paths exist: (a) the chat slash-command
  registry in `crates/commands` which DOES gate on access_level,
  and (b) the wire cell-method dispatch which does NOT. The
  in-process `crates/game/src/commands/gm_cmds.rs` handlers use
  path (a); future implementers MUST NOT confuse the two.

### What I filed

40 numbered findings:

- CAT-N-01: WORLD_INSTANCE_RESET reachable on regular player wire
- CAT-N-02: resetMyAbilities reachable on regular player wire (free respec)
- CAT-N-03: systemic — no access_level in cell dispatch (CRITICAL)
- CAT-N-04: class hardcode + future flip without auth plumbing
- CAT-N-05: SetHideGM exposed — if bHideGM ever becomes a privilege gate
- CAT-N-06..16, 18..40: per-handler trust violations across
  Set/Give/Show/Kill/Spawn/Goto/Load/Debug families. All share the
  CAT-N-03 root cause but each is documented with its specific
  wire shape, severity, and remediation.
- CAT-N-30: TOGGLE_COMBAT_DEBUG family — exposed at SGWAbilityManager
  CMs 2/3/6, stub today, info-disclosure when implemented.
- CAT-N-39: SET_AUTO_CYCLE inherits CAT-C's missing perception
  check on the target.

### What I didn't file (with rationale in the file's Not Filed section)

- Per-mission GM variants (gmMissionAssign/Advance/Reset/Complete/...)
  — same shape as CAT-N-21 (flag mutation), not double-filed.
- Per-debug shapes (DebugAbilityOnMob, BehaviorsOnMob, PathsOnMob,
  MobData) — sibling shapes of CAT-N-31 (DebugEvents).
- LogOff, Disconnect, Unstuck — owned by CAT-A / CAT-B.
- Petition, Who, BroadcastMinimapPing — CAT-L (not GM despite name).
- SetCallback's PYTHON arg — not directly filed because (a) the
  Rust cell decoder doesn't handle PYTHON, (b) it's a CAT-A-style
  RCE concern about the wire type itself.

### Severity calibration used

- Critical: anyone-becomes-GM (CAT-N-03 systemic; CAT-N-05 if
  bHideGM becomes a privilege bit).
- High: anyone mutates authoritative inventory/state of self or
  others — SetGodMode, SetHealth, GiveItem, Kill, Spawn,
  WorldInstanceReset, GotoXYZ, SetSpeed, SetLevel.
- Medium: info disclosure or self-only power inflation (SetFlag,
  SetMobAttribute, SetMobAbilitySet, ShowIP, ShowInventory,
  Invisible, XRayEyes, GiveStargateAddress, SetNoXP, SetNoAggro,
  AddBehaviorEventSet, GiveRespawner/Gearset/Inventory/Blueprint,
  SetFaction, RemoveItem, Respec).
- Low: log noise / DoS shape / corner-case (PerfStats trust,
  SetMovementType enum exposure, gmUsers, ShowMobCount,
  TestLOS/ToggleCombatLOS, PrintStats, RequestReload caller check).

### Linked memory

- [[reference-gm-command-wire-spec]] — authoritative wire shapes
- [[reference-gm-auth-plumbing-gap]] — what needs to change to fix
- [[reference-cell-method-entity-id-authority]] — related: the
  entity_id substitution in cell_arms; CAT-N-29 relies on this
  being correct.
