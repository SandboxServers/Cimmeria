# Ammo Restoration

> Type: how-to. Audience: the Claude Code coordinator and implementing engineers.
> Updated: 2026-09-28. Tracking issue: [#1026](https://github.com/SandboxServers/Cimmeria/issues/1026). Companions: [audit](audit.md), [work packets](work-packets.md), [session resume](handoffs/session-resume.md), [documentation index](../../readme.md).

## Purpose

Ammo today is clip-only: a bandolier slot holds `clip_size` rounds, reload refills it to full from nothing, and the selected `cur_ammo_type` is persisted but never read by the damage path. Of the weapons in `db/resources/Items/Seed/items.sql`, 595 allow only `Bullet_Default` and 19 only `Dart_Default` — the client's ammo-type picker (`Bandolier.lua`, `getAmmoTypes`) never has a second option to show. This campaign restores **special ammo as a finite, lootable resource**, while default ammo (`Bullet_Default` / `Dart_Default`) keeps today's free reloads.

Out of scope for this campaign:

- Org (Team/Command) vault storage of ammo items — ammo stacks are ordinary inventory items, so the Bank campaign's existing vault rules already cover them; no new work here.
- A per-weapon ammo-type UI change on the client — the picker already exists (`Bandolier.lua`, 23 ammo icons, a 6-button picker, `requestAmmoChange`). This campaign is data plus server logic.
- Dagger ammo types (`Dagger_Default`/`Dagger_Metallic`/`Dagger_Poison`/`Dagger_Electrical`/`Dagger_Disease`/`Dagger_Plasma`). The issue's 17 toggle abilities list bullets and darts only; no dagger toggle ability is named. Daggers are melee weapons in the current seed (`max_melee_range` populated, `max_ranged_range` 0), so they are not bandolier ammo in the sense this campaign restores. Flagged as an open question below rather than silently dropped.

## Decisions (@Cadacious, 2026-09-28)

| ID | Decision |
|---|---|
| D-AM01 | **Reserve lives in the bags.** Special ammo is stackable inventory items, one item per ammo type. They show in the bags and in the normal corpse loot window, and reload of a special type draws from them. Not a hidden per-type pool. **AM-01 confirmed this is a restoration-team design choice, not a recovered mechanic**: the client schema has no per-type count, no ammo-item category, and no reserve stat anywhere — `knownAmmoTypes` is a discovery/unlock flag array, not a quantity (HIGH confidence in the absence; [ammo-system.md § Q1](../../reverse-engineering/findings/ammo-system.md#q1--the-original-reserve-model)). Record it as completing an incomplete original design, not as restoring what SGW had. |
| D-AM02 | **Special ammo is finite, default stays free.** `Bullet_Default` / `Dart_Default` keep today's free reloads. Special types are finite, drop from NPCs, and switch the damage modifier. |
| D-AM03 | **Its own campaign.** It does not block the Castle pre-Romney reward chest; Hollow Point is added to that chest once this lands. |
| D-AM04 | **Scope: all four families**, in this order: Hollow Point and Armor Piercing first, then Incendiary / EMP / Explosive, then the dart toggles. |
| D-AM05 | **Counted in rounds, never punish the player.** Clips and bag stacks both count individual rounds, not magazines. A reload takes only `clip_size - current_ammo` rounds from the selected type's stack; a partial reload keeps the rounds already in the clip; a short stack loads what is there. Switching ammo type returns the unfired special rounds to their bag stack, or leaves them in the weapon (never deletes them) if the bags are full. |
| D-AM06 | **The debug hub loot crate is the test rig.** Loot table 3 ("Debug hub loot crate"), opened through `open_loot` without being killed (#1031), must hand out, at probability 1: a full stack of every bullet special ammo type (Hollow Point, Armor Piercing, Incendiary, EMP, Explosive); at least one pistol and one SMG whose `ammo_types` include all of those types. Extend with darts when that packet lands. Debug-hub only, never live content. |
| D-AM07 | **The server applies the ammo modifier directly (option a).** Any shot fired with a special type loaded applies that type's `ammo_modifiers` row (damage, penetration, damage type, on-hit effect) on the server. There is no cast and no cooldown, and toggle abilities 715/719/… are not launched: they stay as seeded, and `toggle_ability_id` records only where the numbers came from. Answers open question 1. |
| D-AM08 | **Dagger ammo types go in a later wave**, not permanently out of scope. After Wave 2, a packet designs `Dagger_*` coatings (Metallic/Poison/Electrical/Disease/Plasma) as ammo-like items for melee daggers. It is RECONSTRUCTION with no toggle-ability evidence, and it is not scheduled until Wave 2 closes. Answers open question 2. |
| D-AM09 | **`/gmsetinfiniteammo` only frees the reserve.** Reloads of special ammo draw nothing from the bags, but the clip still empties and still needs a reload, so testers exercise the normal reload path. It is backed by `bInfiniteAmmo`. Answers open question 3. |
| D-AM10 | **Standard Pistol (27 ids) and Standard SMG (25 ids) accept all five bullet special types**, per [audit.md § 6](audit.md#6-weapon-families-considered-for-widening). AM-F widens exactly these. Answers open question 4. |

## Open questions

**Resolved by AM-01** ([docs/reverse-engineering/findings/ammo-system.md](../../reverse-engineering/findings/ammo-system.md), PR [#1040](https://github.com/SandboxServers/Cimmeria/pull/1040)):

- *Items vs. pool* — answered. No reserve model of any kind exists in the client schema; D-AM01 is a restoration-team design choice (see the D-AM01 row above).
- *New item-id feasibility* — de-risked but not proven. The cooked-data element fetch is a plain numeric-key lookup with no static id-range check (`versionInfoRequest → onVersionInfo → InvalidKeys → elementDataRequest → resourceFragment`), and PR #405 proved the mechanism end-to-end for **existing** ids (2893/4735). Nobody has proven it for a **wholly new** id (MEDIUM-HIGH confidence it will work; [ammo-system.md § Q4](../../reverse-engineering/findings/ammo-system.md#q4--new-item-ids-feasibility)). AM-07 now runs an early **spike** — push one new ammo item's cooked entry and confirm it renders in a live client — before batch-authoring the rest. See [work-packets.md § AM-07](work-packets.md#am-07-client-push-narrowed-scope-early-spike).
- *Whether widening a weapon's `ammo_types` needs a client patch* — answered, no. `getAmmoTypes`, `getCurrentAmmoType` and `requestAmmoChange` all resolve through the same live, server-populated `SGWPlayer+0x8c → +0x24` container cache, not `CookedDataItems.pak` (HIGH confidence; [ammo-system.md § Q2](../../reverse-engineering/findings/ammo-system.md#q2--where-the-clients-allowed-ammo-type-list-comes-from)). AM-F's widening is DB-only; AM-07's client push covers only the 15 new ammo item *definitions* (icon/name/stack), never the weapons.
- *Client-side gates* — none found on reload or the ammo picker (HIGH confidence). One adjacent caution for UAT: the bandolier active-slot swap (F1-F4) has a *different*, confirmed client-side no-op suppression on stale cached state; a tester who swaps weapon slots immediately around an ammo-type pick could see a stale-looking result even though `requestAmmoChange` itself has no such gate ([ammo-system.md § Q5](../../reverse-engineering/findings/ammo-system.md#q5--client-side-gates-on-reload-or-ammo-picking)).

**Questions 1-4 below were answered on 2026-09-28; see D-AM07 to D-AM10. Question 5 is informational.**

1. **How toggle abilities 715/719 (and their siblings) connect to `requestAmmoChange`.** AM-01 confirmed these are architecturally independent systems in the client — no automatic link was found, and `GENERICPROPERTY_AmmoTypeId` (propId 3) drives only the UI icon ([ammo-system.md § Q3](../../reverse-engineering/findings/ammo-system.md#q3--how-ammo-type-and-toggle-abilities-connect)). Acceptance criterion 4 is therefore a design decision, not a recovery. **Recommendation: option (a) — the server applies the ammo type's damage modifier directly and automatically whenever a shot fires with that type loaded, with no cast, no cooldown interaction, and no change to the toggle abilities themselves.** The abilities (715, 719, …) stay exactly as already seeded, untouched by this campaign; `ammo_modifiers.toggle_ability_id` (AM-F's contract) is provenance-only — it records which ability's description/effect text the reconstructed multiplier numbers came from, not a live dependency. This matches D-AM02's "switch the damage modifier" language literally, needs no integration with the ability-cast pipeline, and avoids the unscoped question of whether picking ammo should also show a buff icon. The alternative, option (b) — `requestAmmoChange` server-launches the matching toggle ability — was considered and rejected for this recommendation because it entangles ammo switching with cooldowns (719 has a 30 s cooldown) in a way nothing in the client or the issue asks for, and ability 715's `effect_ids` is empty regardless, so the damage numbers must be reconstructed either way. **Needs owner sign-off before AM-04 ships.**
2. **Dagger ammo types.** Are `Dagger_*` in scope for a later wave, or permanently out of scope because no toggle ability exists for them? Needs an owner answer before any packet touches them; nothing in this ledger schedules them.
3. **`bInfiniteAmmo` semantics for `/gmsetinfiniteammo`.** The property exists (`SGWAbilityManager.def`, `CELL_PUBLIC INT8`) but nothing reads it today. AM-06 proposes it bypasses the reserve draw during reload; confirm this is the intended scope (does it also mean unlimited clip, i.e. skip the reload gate entirely?).
4. **Which pistol and SMG families to widen.** This plan proposes the `description = 'Standard Pistol'` (27 item ids) and `description = 'Standard SMG'` (25 item ids) families — see [audit.md § Weapon families](audit.md#6-weapon-families-considered-for-widening) for the reasoning and the full id lists. Confirm before AM-F ships, since every later packet's debug-crate and loot rows cite specific ids from this choice.
5. **`/gmgiveammo`'s wire byte layout is unrecovered, and it may never have had a working server receiver.** AM-01 confirmed `Event_NetOut_GiveAmmo` is a registered, client-emittable NetOut event, but **no `.def` entry exists anywhere under `entities/defs/` for a `GiveAmmo` cell/base method** — the developer-side receiver is either lost or was never finished (HIGH confidence in the absence; [ammo-system.md § Q1](../../reverse-engineering/findings/ammo-system.md#what-gmgiveammo-ammoid-quantity-sends)). AM-06 therefore builds `/gmgiveammo` as a GM-gated `.`-console command instead of wiring the native opcode, per the project's existing split for commands with no native binding (`docs/agents/rules-and-gotchas.md`; PR #518/#523 precedent). If a later session recovers the byte layout, wiring the real native opcode is a clean follow-up, not a redesign — see [work-packets.md § AM-06](work-packets.md#am-06-gm-tooling-revised-console-first).

## Packet status

| Packet | Status | Owner (advisor) | Depends on |
|---|---|---|---|
| AM-F | Ready | `items-systems-advisor` design, `rust-gameserver-dev` writer, `database-persistence` schema review | Plan (this PR) |
| AM-01 | **Done** (PR [#1040](https://github.com/SandboxServers/Cimmeria/pull/1040)) | `game-archaeology-specialist` | none — ran ahead of AM-F, findings folded into this ledger |
| AM-02 Reserve | BlockedDependency (AM-F) | `rust-gameserver-dev`, review `items-systems-advisor` + `server-authority-enforcer` | AM-F |
| AM-03 Validation | BlockedDependency (AM-F) | `rust-gameserver-dev`, review `items-systems-advisor` | AM-F |
| AM-04 Damage framework | BlockedDependency (AM-F) | `rust-gameserver-dev`, review `combat-systems-advisor` | AM-F |
| AM-05 Loot and crates | BlockedDependency (AM-F) | `rust-gameserver-dev`, review `items-systems-advisor` | AM-F |
| AM-06 GM tooling | BlockedDependency (AM-F) | `rust-gameserver-dev`, review `server-authority-enforcer` | AM-F |
| AM-07 Client push | BlockedDependency (AM-F) | `rust-gameserver-dev` | AM-F only — no longer gated on AM-01, which is done; runs a spike-first internal order |
| AM-08 Incendiary | BlockedDependency (AM-04) | `rust-gameserver-dev`, review `combat-systems-advisor` | AM-04 |
| AM-09 EMP | BlockedDependency (AM-04) | `rust-gameserver-dev`, review `combat-systems-advisor` | AM-04 |
| AM-10 Explosive | BlockedDependency (AM-04) | `rust-gameserver-dev`, review `combat-systems-advisor` | AM-04 |
| AM-11a/b/c Darts | BlockedDependency (AM-04) | `rust-gameserver-dev`, review `combat-systems-advisor` | AM-04 |
| AM-12 Close-out | BlockedDependency (all above) | coordinator + `documentation-writer` | Every packet above |

Full scope, contract and test-type table: [work-packets.md](work-packets.md).

## Parallelization plan

The full design, file-ownership matrix and dependency graph are in [work-packets.md](work-packets.md). Summary:

- **AM-F is the only serial gate.** It is kept deliberately small: reserved item ids, the `ammo_item_types` mapping table, the `AmmoReserve` and `AmmoModifier` Rust contracts, pre-split hotspot files, the `ammo.finite_special` feature flag (default off), and the telemetry catalog. Nothing in AM-F turns on player-visible behavior.
- **Wave 1 is six packets in parallel** once AM-F merges (AM-02 through AM-07) — AM-01 already shipped ahead of AM-F as a read-only research packet, so it is not a Wave-1 occupant any more. None of the six is gated on RE findings any more: every packet goes through the `ammo_item_types` mapping table rather than a hardcoded item id, and AM-01 resolved the one real risk (new-item-id feasibility) to "proceed, but spike first" rather than "wait." AM-07 carries that spike internally.
- **Wave 2 is one packet per remaining ammo family** (Incendiary, EMP, Explosive, and darts split into three by effect kind: crowd-control, tech-disable, buff/heal), gated only on AM-04 (the damage framework), not on each other.
- **Wave 3 is the close-out**, which flips `ammo.finite_special` on, runs the unified UAT, and updates the status docs once.
- Maximum useful parallelism is **6 agents in Wave 1** (AM-02 through AM-07), dropping to **6 in Wave 2** (AM-08, AM-09, AM-10, AM-11a, AM-11b, AM-11c), 1 in AM-F and 1 in AM-12. Critical path: AM-F → AM-04 → (AM-08..AM-11c, longest of which gates AM-12) → AM-12 — four packets deep.

## UAT checklist outline

Steps land in [handoffs/session-resume.md](handoffs/session-resume.md#uat-checklist) as packets ship real behavior; this is the outline AM-12 fills in with SigNoz queries, matching the acceptance criteria in #1026:

1. Bandolier picker shows a special type only when the weapon's `ammo_types` allows it; `requestAmmoChange` to a disallowed type is refused.
2. Reload with a special type selected draws from the bag stack, not from nothing; an empty stack refuses the reload with visible feedback and leaves the clip as it was.
3. Default-ammo reload is unchanged (regression).
4. Partial reload and ammo-switch round-trip the exact round count (D-AM05 arithmetic, both directions).
5. Hollow Point and Armor Piercing visibly change damage and penetration through their toggle ability's effect.
6. NPC loot and the corpse window drop and stack special ammo.
7. The debug-hub crate (table 3) hands out a full stack of every bullet special plus a pistol and an SMG that list all five (D-AM06).
8. The Castle pre-Romney chest includes Hollow Point.
9. `/gmgiveammo` and `/gmsetinfiniteammo` work for testers.

## Known issues

- The damage path (`RangedPhysicalDamage`/`MeleeDamage` effect scripts, `crates/cell-world/src/cell/effects/scripts.rs`) reads only static `FocusDamage`/`HealthDamage` params from the ability/effect definition today — it has no notion of the attacker's active ammo type. AM-04 adds the read; see [audit.md § Damage path](audit.md#3-the-damage-path-ignores-ammo-type).
- `requestAmmoChange`'s cache-miss fall-open (#448 / PR #602, open) is a pre-existing hazard this campaign's AM-03 packet absorbs rather than leaving as a parallel, uncoordinated fix. See [work-packets.md § AM-03](work-packets.md#am-03-validation).
- Client Lua (`Bandolier.lua`, `getAmmoTypes`) is cited from the issue and from `docs/gameplay/weapon-ammo-reload.md`'s existing line references; this worktree has no local client copy to re-verify against (`game/sgw/` is a placeholder — see CLAUDE.md repo invariants). AM-01's Ghidra trace independently corroborates the mechanism (the `SGWPlayer+0x8c → +0x24` container cache) without needing the Lua source, so this is now cross-confirmed rather than a single-source citation.
- `getAmmoTypes`/`getCurrentAmmoType`'s exact lookup key (container id vs. item id vs. both) is ambiguous in AM-01's decompile — only one of two converted Lua arguments visibly reaches the native call, which may be a real one-argument signature or a decompiler artifact. Not load-bearing for this plan (the server already knows the answer server-side via `InvItem.ammoTypes`), but worth a disassembly-level or x64dbg trace if a future packet needs the exact client-side semantics.
- `Event_NetOut_GiveAmmo`'s exact wire byte layout (arg types/order for `ammoId`/`quantity`) is unrecovered, and `ammoId` is only MEDIUM-HIGH confidence to be an `EAmmoType` rather than an item id (inferred from naming convention against `GiveItem`'s `designId`, not a confirmed byte read). AM-06 does not depend on this — it builds the GM tool as a `.`-console command instead (open question 5 above).
