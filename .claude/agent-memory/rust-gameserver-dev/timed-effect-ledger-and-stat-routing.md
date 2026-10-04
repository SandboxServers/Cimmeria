---
name: timed-effect-ledger-and-stat-routing
description: AB-04 timed effect ledger shape (per-(effect,invoker) entries, TimedStacking), why the script writes it, and the routing traps that decide which stat effects can be bound
metadata:
  type: project
---

Since AB-04 (2026-10-03) `CellEntity::stat_buffs` (`StatBuffLedger`) holds
`TimedEffect` entries keyed `(effect_id, invoker_id)`, each with all its stat
deltas, `expires_at: Option<Instant>` (None = held), flags and the ability's
`moniker_ids`. Stacking is a spec field: `TimedStat` uses `PerSource`, the
stimpack `StatBuff` keeps decision 28's `ReplaceSameStat`.

**Why the script writes the ledger, not the seam:** scripts already run where
effects land (`fire_beneficial`, `damage_apply` after-hit scripts — miss-gated
by `plan_hit_effects` — and content `apply_effect`), before the death sweep, so
an `EF_ClearOnDeath` debuff on a killed target is stripped. The seams only call
`flush_stat_buff_timers` so the icon goes out in the same resolution.

**Routing traps for binding a stat effect (generator `families/stat.py`):**
- Binding a non-beneficial effect on a Self ability flips `ability_is_beneficial`
  and the cast lands on the wire target (a hostile). Combat Sprint 2002 stays unbound.
- A beneficial effect beside a non-beneficial effect that does something takes
  the hostile path too.
- "Secondary Target" halves are TCM_Single and would double on the primary
  (Hunker Down 1746 + 1747 = +200).
- Regen stats: `regen.rs` reads FOCUS_REGEN as points/s, so "+50%" = 50/s until AB-05.

**Monikers:** `abilities.moniker_ids` are broad shared CRCs (1470900795 is on
most combat abilities); the seed has no effect-moniker column. Removing by an
ability moniker would strip unrelated buffs.

**How to apply:** later packets (AB-05/08/09/10) call
`SpaceManager::apply_timed_effect` / `remove_timed_effects` /
`strip_timed_effects`; never key a new ledger by stat. See
[[beneficial-cast-resolution-and-abilitydef-fields]].
