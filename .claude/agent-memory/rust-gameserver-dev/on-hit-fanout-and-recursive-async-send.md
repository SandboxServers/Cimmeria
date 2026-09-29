---
name: on-hit-fanout-and-recursive-async-send
description: An effect that damages OTHER entities cannot be an EffectScript (no wire, no death); fan out in damage_apply, and box the recursion with a named dyn Send future
metadata:
  type: project
---

Learned on AM-10 (Explosive splash, 2026-09-28).

- `EffectScript::on_apply` gets a sync `EffectContext` (`&mut SpaceManager`, no `tx`). It can write any entity's stats but cannot send `onEffectResults` / `onStatUpdate`, call `resolve_death` or `generate_threat`. `damage_apply` flushes and death-sweeps only the hit's own target. So anything hitting secondaries (splash, chain lightning) belongs in `cell-combat` `damage_apply`, applied per secondary through the same per-target body (`apply_hit(.., HitKind)`), like cone secondaries.
- Recursion `apply_hit` -> `apply_splash` -> `apply_hit`: `Box::pin(apply_hit(..))` cast to `dyn Future + Send` inside the callee still fails ("future is not Send") because the opaque types form a cycle. Fix: make the middle function a plain `fn` returning a named `Pin<Box<dyn Future<Output = ()> + Send + 'a>>` that boxes an inner `async fn`.
- No-chaining guard: the splash kind runs no on-hit effect. Reverting it overflows the stack in tests (two NPCs splash each other forever), which is a clear failure.
- `last_aoe_deaths` is drained by `credit_single_target` on every cast and by `credit_ground_deaths` (since AM-10); before that, a cone kill whose primary survived was credited to a later cast.
- `collect_ground_targets` (dispatch) is the reusable "hostiles within r of a point" collector (`pub(super)`), with the area rule built in. Area collectors do no LOS; add `occluder_probe` yourself with the fire-LOS policy (no occluder / Unknown never refuses).

Related: [[ability-launch-fire-split]], [[kill-credit-seams-and-loot-ownership]].
