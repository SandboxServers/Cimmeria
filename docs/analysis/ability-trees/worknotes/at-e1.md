---
title: "AT-E1 Worknote — Trainer UI Client Evidence"
type: reference
audience: ability-trees campaign coordinator
last_updated: 2026-09-25
---

# AT-E1 worknote — one-screen summary

Full evidence: `docs/reverse-engineering/findings/ability-trainer-ui.md`.

## Answers at a glance

| # | Question | Status | Answer |
|---|---|---|---|
| 1 | Tree/trainer join | **RESOLVED** | Hidden, not greyed, confirmed at byte level (`getTrainableInfo` writes no field when the id isn't in the trainer map). Order comes from the **tree** (`onAbilityTreeInfo`), not the trainer list. No cap beyond Lua's `MAX_BUTTONS = 30`. |
| 2 | `onErrorCode` rendering | **UNRESOLVED** | No Lua consumer exists anywhere in the client (full-tree grep, zero hits). Whether a native C++ listener renders it is unknown — not traced under the tightened budget. Treat server-side sends as correct-but-unverified presentation. |
| 3 | Points counter refresh | **RESOLVED** | Yes — `onEntityProperty(TrainingPoints)` → `Events.PropertyUpdated` → `AbilityMod.onPropertyUpdated` → `refreshTrainingPoints()`. Fires regardless of window visibility. |
| 4 | Level 50 cap | **RESOLVED** | No client-side level/XP table anywhere. `ExpBar.lua` is a pure passthrough of two server-pushed properties (`onExpUpdate`/`onMaxExpUpdate`). No client patch needed. |
| 5 | Respec | **RESOLVED (send) / PARTIAL (receive)** | `respecAbilities()` sends a bare, zero-payload cell method 72 `resetMyAbilities` call, confirmed by decompile. Client does **not** clear the hotbar itself — no evidence of any hotbar-cleanup subscriber. |

## D-AT08 recommended error-code mapping

| `TrainReject` reason | Code | Fit |
|---|---|---|
| Wrong archetype | `CONDITION_FEEDBACK_NotSpecifiedArchetype` (6) | Exact |
| Missing prerequisite | `CONDITION_FEEDBACK_EntityDoesNotHaveAbility` (167) | Exact |
| Not at / too far from trainer | `CONDITION_FEEDBACK_OutsideDistanceCheck` (43) | Close |
| Level too low | `CONDITION_FEEDBACK_LevelGreaterThanOrEqual` (9) | Close |
| Not enough training points | `CONDITION_FEEDBACK_StatValueLessThan` (35) | **No exact code exists** — fallback, reused not recovered |
| Branch-spend gate | `CONDITION_FEEDBACK_StatValueLessThan` (35) | **No exact code exists** — same fallback |
| Duplicate purchase | *(no code — stays silent, as already drafted)* | By design |

The 2009 `EConditionHandlerFeedback` enum has zero tokens for the trainer's own economy (training points, branch spend) — both are new gates the pack introduces. Any code chosen for those two rows is a documented reuse, not a recovered original value; say so in the AT-04 implementation PR.

## Q4 — level 50, for the coordinator

The client has **no** hard-coded level cap or XP table anywhere checked (`ExpBar.lua`, `Character.lua`, `UnitFrames.lua`, `Greet.lua` all just print `unitLevel()`/`getExperience()`/`getMaxExperience()` verbatim). This is purely a server-side change (AT-07). The one thing to get right: `ExpBar.lua` divides `curExp / (1.0 * maxExp)` with **no zero-guard**, so at the level-50 sentinel the server must send a non-zero `onMaxExpUpdate` value (recommend `MaxExp == Exp`, i.e. bar reads 100% and stops moving) — sending `0` would produce a NaN/inf progress bar.

## Q5 — respec, for the coordinator

`respecAbilities()` → cell method 72 `resetMyAbilities`, **no arguments**, confirmed by Ghidra decompile of the native Lua binding (shim `0x00aa2d80`, inner sender `0x00aeacd0`). This matches AT-08's plan as written.

**Correction to the work-packets.md assumption**: AT-08's scope line says to "Remove refunded abilities from the hotbar the way AT-E1 question 5 says the client expects." The evidence says the opposite — **the client does not expect anything and has no hotbar-cleanup mechanism**. A full-tree grep for `Events.AbilityUpdate` subscribers found exactly one (`Ability.lua`); `ActionButtons.lua` never subscribes to any ability-removal or property-update signal. A respec will leave stale action-bar bindings in place until the player manually clears them, or until a doomed `useAbility` press fails silently (consistent with the project's existing silent-rejection pattern elsewhere). AT-08 should treat hotbar cleanup as **new work with no client hook to lean on**, not as something the client already handles once the server sends the right update — or explicitly accept the stale-binding UX gap as out of scope.

## What's still open

1. Whether a **native** (non-Lua) listener renders `onErrorCode` — not traced. Needs a live-client, non-freezing breakpoint trace or a manual CME subscriber-list walk.
2. Whether `onKnownAbilitiesUpdate`'s add/remove diffing (the bridge from the flat wire array to per-id `Events.AbilityUpdate` calls) handles **removal** the same way it handles addition. Matters directly for whether the Ability window refreshes correctly after a respec.
3. Whether `getAbilityInfo(id)` still resolves for an ability the player no longer knows (likely yes, since ability defs are static content) — would explain why a stale hotbar button keeps rendering normally.
