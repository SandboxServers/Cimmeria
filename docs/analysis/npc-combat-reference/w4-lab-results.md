# NPC combat reference: W4 lab results

Type: reference. Audience: engineers and the campaign owner. Last updated: 2026-11-27.
Companions: [evidence-registry.md](evidence-registry.md), [uat-checklist.md](uat-checklist.md), [README.md](README.md), [RESUME.md](RESUME.md).

Attested results of the W4 live-client lab runs NC-40..44. Server `086fc2710`, run date 2026-11-27, local DB `sgw_nc_uat`, character `Nctester`, world `DebugArea`. The lab driver reads chat and the combat log; it could not inspect screenshots, and the lab has no audio. Animation, muzzle effects, VFX and sound are therefore NOT verified for any NPC below, and nothing here is ORIGINAL CLIENT VERIFIED.

## NC-40 NID Guard (template 24, ability 559, effect 5310): PASS with notes

- Server evidence (combat.log): NID Guard (entity 100430) casts ability 559 "Automatic Weapon Auto Attack" about every 2 s. Every cast dispatches RangedPhysicalDamage with effect 5310 "AW Auto Attack Damage" (37 casts, no 621, no 592).
- `base_health_damage=10` and `focus_damage=100` per hit (D-NC12 tooltip values, RECONSTRUCTED).
- The existing Focus-pierce spillover adds health bleed once Focus is gone (client saw 20, then 43 per hit). This is the pre-existing QR/Focus mechanic, not the effect payload. The player died after 37 casts (Target killed, ability 559).
- Client: NID Guard rendered, on screen, damaging the player. Screenshot `lab_screenshot-20261127-213801.529.png` (driver could not inspect it; animation and muzzle effect NOT verified).
- Not checked: 20 s respawn after a kill (NC-40 respawn row), animation wording.
- Setup gotchas: the world name is `DebugArea`, not 1300; `lab_ensure_in_world` create needs alignment sgu or praxis; `.aggro on` worked (the account is GM-capable).

## NC-43 Prisoner Retrieval Unit (template 4, entity 100431, ability 221, effect 264): PASS with notes

- Server evidence (combat.log): 23 matching lines for attacker 100431 ability 221; none for ability 592.
- Client: "Energy Shock hits/glances/crits you for 16 Health damage" every step (health 760 to 456 in about 25 s).
- Anomaly: the client shows the attacker name blank ("'s Energy Shock"); `client_entity_find` by name returned nothing; entity 100431 has no name. Known cosmetic (K26), not a combat failure.
- Animation and beam not described by the driver: presentation NOT verified. Screenshot `lab_screenshot-20261127-214007.156.png`.

## NC-42 ScavDog (template 76, entity 100432, ability 1433 Mob Strike, set 6): PASS with notes

- Server evidence (combat.log): 180 lines for entity 100432 with ability 1433 "Mob Strike" (first cast 03:45:42, cooldown 2.0 s); zero lines with ability 592.
- Client: "Mob Strike hits/glances you for 33-56 Health"; health 760 to 223 in about 15 s once in reach. The attacker name is blank in the client combat line (same display anomaly as the PRU).
- The dog did not move; it fired from about 2.5 m. Animation NOT verified (no visual read). Screenshot `lab_screenshot-20261127-214646.986.png`.
- Lab lesson: `/gmsethealth` answers "target not found" and `server_console_list` says `not_gm` for the lab character. Health refill used death at the NID Guard and Release in PlayerDefeatWin (respawner 130, Debug Area Arrival; 0:21 auto-respawn countdown). The death and respawn flow works.

## NC-41 Praxis Jaffa Guard 100433 and Lieutenant 100434 (templates 1380/1381 lineup actors; set 4 = 584 + 710; effect 736): PASS with notes

- Server evidence (combat.log): 71 `ability_launched` rows for the two Jaffa: ability 584 (Staff Auto Attack) x30, ability 710 (Staff Melee AA) x41; zero rows with ability 592. Effect 736 appears 117 times.
- Client: "Staff Auto Attack hits you for 101/108/81/62..." and "Staff Melee AA ... 43/33", with the Jaffa names shown (the blank-name anomaly is specific to PRU and ScavDog).
- Both Jaffa walked into melee range even from 15 m and 8 m, so the NC-U2 "ranged blast at 15 m" row is only partly observed: Staff Auto Attack (584) landed at 15 m; the Guard closed to 1 m by +10 s. At 2 m both abilities appeared (the D-NC4 melee preference on cooldown was not separately isolated).
- The hostility field read "0" on both yet they attacked (aggro on).
- Presentation (staff animation, effect) NOT verified: no screenshot mid-fight, the driver reads chat only. Damage per hit and total (760 to 0 in about 10 s at 2 m) feels high; the effect 736 payload is RECONSTRUCTED (D-NC2). Flagged for owner tuning, not a defect.

## NC-44 telemetry rows: PASS

- 0 `ability_launched` rows with ability 592 anywhere in `logs/combat.log` for the whole W4 session (NID, PRU, ScavDog and Jaffa all fired their own ability sets).
- 0 `combat_profile_unresolved` or `combat_profile_placeholder` rows for templates 24, 76, 4, 159, 160 in `logs/server.log`.
- 21 `combat_profile_placeholder` rows remain, all for templates with firearm evidence (WP_SMG_1A and similar); 266 `combat_profile_unresolved` rows for templates without an ability set or weapon evidence (they hold fire, by D-NC1). These are the V2 backlog.

## Not exercised or not verified

- NC-40: 20 s respawn of the NID Guard.
- T2 Z9 lethal squad (18 s).
- All animation, VFX and sound rows (not-verified-in-client or UNRESOLVED).
- GM health commands fail ("target not found"), so health refill used death plus Release.

## Open findings from W4

1. Blank attacker name for PRU and ScavDog in the client combat log (known cosmetic: templates 4 and 76 ship no name, unified-uat K26; not a combat defect).
2. Jaffa damage feels high; effect 736 is RECONSTRUCTED, owner tuning.
3. The items in "Not exercised or not verified" above.
