---
name: damage-apply-miss-gate-and-seeded-rolls
description: Since AB-06 a QR miss lands nothing in damage_apply; tests with a fixed effect_seq can silently roll a miss — pick the seq with seq_rolling
metadata:
  type: project
---

Since AB-06 (2026-10-03, D-AB07) `damage_apply` gates every QR-rolled effect on
`RC_MISS`: no NVP damage, no script, no pulse registration. Damage scripts
(`RangedPhysicalDamage`, `MeleePhysicalDamage`, `RangedEnergyDamage`,
`MeleeDamage`) are their effect's only damage and run with the direct damage,
NVPs scaled by cover × ammo × splash (`damage_apply/effect_scripts.rs`).
`EF_DONT_USE_QR` is 16 and read in `damage_apply/qr_gate.rs`.

**Why it matters for tests:** the roll is deterministic per
`(attacker, ability_id, effect_seq)` (`abilities/rng.rs`). A test that passes
`effect_seq = 1` may be on a miss: NPC 2 → player 1 with ability 579 at seq 1
is a miss, which the old code hid because scripts ignored misses.

**How to apply:** in `damage_apply` tests, choose the seq with
`single_damage_path_tests::seq_rolling(&mgr, (attacker, target), ability, want_miss)`
instead of a literal, and assert the result code (byte 16 of the
`onEffectResults` args) when the outcome matters. A NVP miss at very negative
QR deals 0 anyway (`1 + qr < 0`), so force misses by seed at the real QR, not
by inflating the defender's stats. See [[testing-patterns-index]].
