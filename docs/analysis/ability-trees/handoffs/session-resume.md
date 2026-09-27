# Ability Trees: Session Resume

> Type: how-to. Audience: the owner (UAT) and any later session.
> Updated: 2026-09-27. Companions: [launch prompt and decisions](../README.md), [work packets](../work-packets.md), [audit](../audit.md).

## State: campaign complete, awaiting owner UAT (AT-06)

Every packet is merged. `/release` goes on this close-out PR (D-AT05).

| Packet | PR | What it delivers |
|---|---|---|
| Plan | #805, #818 | Ledger, decisions, and the AT-10 addition |
| AT-01 | #813 | Schema, the shared `AbilityTreeCatalog`, the single `evaluate_train` predicate |
| AT-E1 | #809 | Client evidence for the trainer UI (`findings/ability-trainer-ui.md`) |
| AT-07 | #812 | Level cap 50, and 1 training point at level 1 plus 1 per level |
| AT-03 | #820 | Archetype-wide spend gate, one-statement purchase, the points refresh. Fixes a live bug: the cell never loaded a player's level, so everyone trained as level 1 |
| AT-05b | #807 | The 439-node FINAL v2 seed, its generator and validator, and live-DB seed guards, including reachability |
| AT-10 | #828 | Charged abilities fire when the warmup ends, with interrupts and a cooldown refund |
| AT-04 | #827 | Trainer authority gates, rejection feedback, and a same-space check on interact range |
| AT-02 | #832 | `onAbilityTreeInfo` built from the one shared catalog; the hard-coded trees are deleted |
| AT-08 | #834 | Respec (cell method 72), atomic refund, and feedback when a stale action-bar button is pressed |

The code lives in the post-#825 crates:

- `ability_tree/` is in `crates/cell-catalog`.
- Train and respec are in `crates/cell-methods` (`vendor/`).
- The trainer pin is in `crates/cell-interactions`.
- Warmup is in `crates/cell-combat` (`use_ability/warmup/`).
- Progression, train and respec on the base side are in `crates/base-methods`.
- `onAbilityTreeInfo` is in `crates/wire`.

## Owner UAT (AT-06)

Run the ten steps in [work-packets.md § AT-06](../work-packets.md#at-06-owner-uat-colo-after-the-release) on the colo after the release. For each showcase archetype, create a character, then use the Interaction Debug NPC (template 25). Also watch for these, which the tests cannot show:

- **Error text.** Does `onErrorCode` show any text? The code sends one on every rejected purchase, respec or stale-button press, and always re-sends the trainer window, which is the feedback we know works. If no text appears, only the window refresh is visible (AT-E1 Q2).
- **Respec with the Ability window open.** Does the window drop the removed abilities (AT-E1 open question 2)?
- **Charged abilities.** Does the charge bar appear (warmup timer type 1), and is the cooldown timer zeroed on an interrupt?
- **Level 50.** Does the XP bar stay sane? At the cap it is sent `MaxExp == Exp`, never 0.

## Open owner decisions

1. **Should a stun interrupt a warmup?** The worker recommends not yet. A stun here is only `BSF_MOVEMENT_LOCK`, which ring transport and death also set, so a real fix needs a trigger driven by the stun effect itself (worknotes/at10.md).
2. **Should an interrupted warmup carry a relaunch lockout?** The worker recommends not yet. Relaunching only sends extra packets to witnesses; it is not an exploit. If telemetry shows abuse, a silent server-side throttle of about 250 ms would contain it.
3. **A client Lua patch to clear action-bar buttons after a respec.** The server has no hotbar (AT-08), so stale buttons remain. They now answer error 167 when pressed.
4. **Trainer list order (D-AT07, "offer the whole tree").** `onTrainerOpen` still lists abilities in `trainer_abilities` order. The client joins the two lists by id, so nothing is broken. Make the list follow `catalog.tree()` if trainers should offer the whole tree in tree order.
5. **D-AT10 and D-AT11** are still PROPOSED in the README:
   - the respec price stays at 1000 naquadah;
   - existing characters keep every ability they know, and their spend starts at 0.

## Known gaps, carried forward

- A dead trainer still teaches. Trainer re-sends after a rejection are not rate-limited (AT-04).
- An ability that is both bought from the trainer and granted by the equipped weapon drops out of the next known-abilities update after a respec, until the weapon is re-equipped. It still fires (AT-08).
- Two doc conflicts need fixing in their own PR (AT-10):
  - the timer-type numbering in `findings/combat-wire-formats.md` (`AbilityWarmup = 2`) loses to `enumerations.xml` (`AbilityWarmup = 1`);
  - `AF_CHANNEL_ALLOWS_MOVEMENT = 16384` is actually `SpeedPet`'s value in `enumerations.xml:51`.
- `crates/entity/src/cell_entity/entity_struct.rs` is over the 700-line hard cap and should be split.
- Under threaded `cargo test`, `LogCapture` tests sometimes leak between tests. They pass under nextest, which CI uses (AT-04).

## Housekeeping

Campaign worktrees still present: `at-coord` and `at08`. Before removing either, delete its `external` junction with `[System.IO.Directory]::Delete(path, $false)` or `cmd /c rmdir <worktree>\external`. Never delete the junction recursively.
