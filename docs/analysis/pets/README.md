# Pets Restoration

> Type: how-to. Audience: the Claude Code coordinator, packet workers and the owner.
> Updated: 2026-09-27. Companions: [evidence audit](audit.md), [work packets](work-packets.md), [session resume](handoffs/session-resume.md), research ([code map](research/code-map.md), [content inventory](research/content-inventory.md), [client static RE](research/client-static-re.md)), tracking issue [#570](https://github.com/SandboxServers/Cimmeria/issues/570), security criterion CAT-C-11 / #462.

## Purpose

This campaign restores **player pets**: summoned `SGWPet` companions that follow their owner, fight, and take orders from the pet bar. Tracking issue is #570. The client (2009) is complete: it has the `GamePet` entity class (client index 5), the pet window and bar, stances and party-pet targeting. The original server never implemented pets; the legacy Python has stubs only. So the server side is new work that follows the client contract, and **no client patch is needed**.

The first playable slice is **Goa'uld Servant Lord → Summon Straegis (2826)** (owner decision D-PT13): a player casts it, a Straegis Fighter appears after the 6 s warmup, follows the owner, defends them, uses its kit, obeys stance changes from the pet bar, and gives kill credit to the owner. The Jaffa (1643), Prime (1645) and Lo'taur (1644) follow once the core is proven, along with the owner-side pet buffs.

The campaign ships **in-game UAT tooling** alongside the feature:

- a `.pet` GM console command family;
- a `.giveability` GM command;
- a pet trainer NPC in the Castle Cellblock stasis-room debug hub;
- a UAT checklist in the [session resume](handoffs/session-resume.md).

Out of scope:

- Scientist turrets: the client ships no turret model (PT-12 is blocked).
- Pet persistence beyond the session, unless the owner decides otherwise (D-PT01).
- Pet PvP.
- Pet leveling curves: pets take the owner's level at summon.
- Ashrak as a pet: `Summon Ashrak` 2825 is in no ability tree.

## What was found

Against `main` @ `95366c59`. The [audit](audit.md) has the evidence for each row.

| Area | State on `main` | Packets |
|---|---|---|
| Pet entity | None. `class_id` 0x05 is never produced, and nothing uses `PetOwnerId` or `ENTITYFLAG_Pet`. A cell entity with `class_id = 0x05` is enough for the client to build a `GamePet`. | PT-01 |
| Wire | Pet client methods `onPetAbilityList` / `onPetStanceList` / `onPetStanceUpdate` are unimplemented. Their derived wire indices are 29/30/31; `pet-restoration.md` says "idx 0/1/2", which is the client registration order. `pet-wire-formats.md` has the two INT8 fields wrong. | PT-E1, PT-01 |
| Owner binding | Unknown how the client decides an entity is *its* pet (the `Unit.Pet1..4` slots). The server sends no BigWorld property stream for NPCs, so `ownerID` cannot ride the usual path; `onEntityProperty(GENERICPROPERTY_PetOwnerId=5)` is the leading candidate. | PT-E1 (blocks the pet-bar half of PT-01) |
| Commands | CM 88/89/90 are stubs that parse the right widths but check nothing (CAT-C-11). The shipped mouse UI routes **every** pet click, ability or "command", through CM 88. CM 89 has no Lua call site. The small pet bar sends the stance *slot index*, not the stance id (an original-client bug). | PT-04 |
| Summon | No summon path. The summon abilities have **no effects**, no effect script exists, and nothing in any seed or cooked data says which creature a summon spawns. Effect scripts only run inside `apply_damage_to_target`, behind the #444 hostile-target gate that a self-cast summon fails. | PT-S, PT-03 |
| Content | No pet templates. The original pet display names survive as text monikers: 8087 "Jaffa Soldier", 28892 "Jaffa Prime", 28891 "Lo'Taur Servant", 28894 Straegis. The Praxis Jaffa body and kit (template 160) already spawn and fight in Harset. Summon VFX event sets 1121/1122 and 855/1120 exist but are unwired. No ability carries the `PetCommand` flag, so the command row is empty as shipped. | PT-S, PT-11 |
| AI | Follow AI exists (NA24, `npc_ai/follow.rs`). Missing: teleport-to-owner, stances, defend-owner, owner-relative leash, and NPC-vs-NPC engagement. `begin_leash` would strand a pet at its summon point. | PT-05 |
| Kill credit | `grant_xp = true` unconditionally. A pet kill sends XP to the pet id (lost), a mob killing a pet sends `GrantXP` to the mob, and pet kills never advance KillCount missions. | PT-06 |
| Lifecycle | 11 owner-teardown paths (logout, death, respawn, gate travel, ring, space transfer, instanced-space teardown and others). `destroy_entity` has no `tx`. | PT-01 (self-healing), PT-02 |
| Class filters | `class_id == 0x04` is hard-coded in `all_npc_entity_ids`, `ai_driven_npc_entity_ids` and `npc_ids_in_space_of`. The AI and movement ticks must admit 0x05; AoE, cone, respawn and assist must keep excluding it. | PT-01 |
| UAT tooling | No GM give-ability command. A trainer only offers the player's own tree nodes, and the debug trainer (template 25, list 1) offers every node. | PT-07 |
| Leash / poll / onOwnerRespawn | **No client footprint** (the client static RE confirms it). These are server-only, so they are greenfield design values, not constants to recover. | D-PT07, D-PT08 |

## Decisions

| ID | Status | Decision | Reason |
|---|---|---|---|
| D-PT00 | **APPROVED** (owner kickoff via cimmeria-19, 2026-09-27) | Autonomous run: one worktree and one test database per worker, squash-merge each PR after green CI (rebase and re-test first if its CI predates `main`), and `/release` on the last merged PR. | The owner's campaign instructions. |
| D-PT11 | **APPROVED** (owner kickoff) | UAT tooling is part of the campaign: GM `.pet` commands, `.giveability`, a pet trainer NPC in the stasis-room debug hub (templates 350-369, spawns 450-469), and a UAT checklist in the handoff. | The owner's campaign instructions. |
| D-PT01 | **APPROVED** (owner, via cimmeria-19, 2026-09-27) | Pets are **ephemeral per session**. A pet despawns on logout, zone change and owner death, and the player re-summons it. No new table. | No `pets` table ever existed, no `SGWPet` property is persistent, and the design is ability-driven summoning with a "chosen pet" (Heed Our Calling). Persistence can be added later without rework (PT-10). |
| D-PT02 | **APPROVED** (owner, via cimmeria-19, 2026-09-27) | `transferXP = 1.0`: the owner gets 100% of the kill XP (and kill credit) for anything the pet kills. Pets do not level on their own; a pet takes the owner's level at summon. | `.def` default 1.0. `NoPetLeveling` exists as a flag, but no pet XP table exists. |
| D-PT03 | **SUPERSEDED by D-PT13** | Pet roster and looks. Summon Jaffa (1643) = Praxis Jaffa body and kit (clone of template 160, name 8087) first. Then Prime (1645) = Praxis Jaffa Lieutenant look (159, name 28892); Lo'taur (1644) = Goa'uld body with the `AR_G_Underlings` servant dress (name 28891); Straegis (2826) = Straegis Fighter (78, name 28894). Turrets wait for a model. | [Content inventory §3](research/content-inventory.md). Only the Jaffa look is already proven in game. |
| D-PT13 | **APPROVED** (owner, via cimmeria-19, 2026-09-27) | Roster order: **Straegis (2826) is the first pet**, as the Straegis Fighter look (a pet clone of template 78). Then, per D-PT03's looks: Jaffa (1643, Praxis Jaffa), Prime (1645, Lieutenant), Lo'taur (1644, Goa'uld servant dress). Turrets stay deferred. | The owner chose Straegis because its templates, models and abilities already exist. Caveats: the Straegis body has never been spawned by this server, text 28894 is empty (so the name may come from 27377 "Summoned Straegis Fighter"), and 78 has no ability set, so PT-S builds its kit from the Straegis mob abilities. |
| D-PT14 | **APPROVED** (owner rule via cimmeria-19, 2026-09-27) | Every PR gets a Copilot review before merge. The coordinator requests it, fixes each valid comment or replies why it doesn't apply, re-requests after a substantive fix, and squash-merges only when CI is also green. | Owner rule for all campaigns. |
| D-PT15 | **APPROVED** (owner rule via cimmeria-19, 2026-09-27) | Telemetry is a first-class acceptance requirement: pets must be debuggable from SigNoz alone (which path ran, why it was refused, before/after values, correlating ids). Every packet follows `instrumentation-discipline.md`, `negative-logging-convention.md` and `observability.md` (targets `pets.lifecycle`, `pets.command`, `pets.ai`, `pets.credit`). Merged and open packets are retrofitted. | Owner rule for all campaigns. |
| D-PT04 | PROPOSED | One active pet per owner. Summoning again replaces the current pet (despawn, then spawn). | "Chosen pet" wording (2852). The turret "If <1 Turret" effect text implies a cap of 1, raised to 2 by the Dual Turrets capstone (later, with turrets). |
| D-PT05 | PROPOSED | Ship stances and pet abilities. **Do not invent pet commands**: no seed ability gets the `PetCommand` flag, so the command row stays empty, as in the 2009 data. | No ability carries `PetCommand` in the seed or the cooked client data, and the shipped mouse UI never used the command path. |
| D-PT06 | PROPOSED | The pet takes the owner's faction (`ENTITYFLAG_PetUseOwnFaction` semantics). Players cannot damage pets (the existing #444 gate does this once the pet is non-hostile). Hostile NPCs fight a pet once it has threat on them; pet threat also puts the owner in combat. | The binary faction model: anything not faction 10 is friendly to players. |
| D-PT07 | PROPOSED | Follow band 2-5 u. Teleport to the owner when more than **40 u** away, or on a different floor band, rate-limited to once per **5 s**. Checked on the NPC AI tick. | Greenfield: the client has no footprint for these values. 40 u sits above the NPC perception radius, so an unrouted pet does not trail far behind. |
| D-PT08 | PROPOSED | Owner death despawns the pet (`onOwnerDeath`). Respawn and relog need a re-summon. A pet that dies becomes a corpse and is despawned after 10 s. | Simplest consistent rule. The `DespawnOnOwnerLeash` flag family points to owner-coupled despawn. |
| D-PT09 | PROPOSED | Stances. **Passive**: follows and never engages, even when hit. **Defensive** (default): engages whatever damages the owner or the pet. **Aggressive**: also engages hostile NPCs within 15 u of the pet, and the owner's current target once the owner is in combat. Per-template `NoPassive` / `NoDefensive` / `NoAggressive` flags filter the stance list sent to the client. | `EPetStance` values and the per-template stance flags. The engagement rules are greenfield. |
| D-PT10 | PROPOSED | The summon warmup is the ability's warmup (6 s for Goa'uld summons). `SpeedPet` scales it by the owner's `speedPet` stat (111), the same way SpeedGrenade and SpeedDeploy work. With the stat at 0 there is no change. | Heed Our Calling "Summons Chosen Pet Instantly" is an `AlwaysPersist` "Pet Summon Speed increase" effect. |
| D-PT12 | PROPOSED | Summoning pulls Goa'uld gameplay forward into UAT. UAT uses a **Goa'uld character in Castle Cellblock** (the Goa'uld start zone) through the debug-hub pet trainer. | D-AT09 left Goa'uld UAT unscheduled. |

Under the autonomous-run authorization, rows marked PROPOSED are adopted at their defaults unless the owner objects. The owner answered D-PT01, D-PT02 and D-PT03 on 2026-09-27 (D-PT13) and raised no objection to the other defaults. A change is recorded as a new row, never by editing an old one.

## Coordinator launch prompt

You are the Claude Code coordinator for the pets campaign. Implement [work-packets.md](work-packets.md) as small reviewed PRs.

1. Record `git rev-parse origin/main` and check that the audit's file references still hold. Check for a live peer on this campaign (`pets/*` branches, `~/.claude/sessions/*.json`); if one exists, message it and stand down. Crafting (cimmeria-af) and guilds (cimmeria-fa) run in parallel; message them before touching `entity_templates.sql`, `spawnlist.sql` or the entity/AoI creation paths.
2. **Wave 0:** dispatch PT-E1 (RE), PT-01 (foundation) and PT-S (seed) in parallel worktrees with disjoint ownership. PT-01 must not merge its wire indices or owner binding until PT-E1 confirms them.
3. **Wave 1:** once PT-01 is on `main`, dispatch PT-02, PT-03, PT-04, PT-05, PT-06 and PT-07 in parallel. Merge them in the order the contended-file list gives.
4. **Wave 2:** PT-08 (owner abilities on pets) and PT-11 (the rest of the Servant Lord roster), then PT-13 close-out with `/release`.
5. Every worker follows the pets worker rules (`PETS-WORKER-RULES.md` in the shared campaign folder): the build lane, per-worktree test DB, crate-scoped checks, the pre-PR checklist, and the doc-map updates.
6. Keep [handoffs/session-resume.md](handoffs/session-resume.md) current after each merge.
