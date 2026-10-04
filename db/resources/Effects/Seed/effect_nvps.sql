--
-- TOC entry 3255 (class 0 OID 64199)
-- Dependencies: 306
-- Data for Name: effect_nvps; Type: TABLE DATA; Schema: resources; Owner: -
--

INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (1, 659, 'HealPercentage', '35.00');

INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (2, 641, 'HealthDamage', '10');

INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (3, 641, 'FocusDamage', '100');

INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (4, 654, 'HealthDamage', '15');

INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (5, 654, 'FocusDamage', '150');

INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (6, 3091, 'HealthDamage', '15');

INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (7, 3091, 'FocusDamage', '150');

INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (8, 3834, 'HealthDamage', '25');

INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (9, 3834, 'FocusDamage', '250');

INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (10, 646, 'HealthDamage', '25');

INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (11, 646, 'FocusDamage', '250');

INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (12, 4467, 'HealthDamage', '15');

INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (13, 4467, 'FocusDamage', '150');

INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (14, 264, 'HealthDamage', '16');

INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (15, 264, 'FocusDamage', '80');

INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (16, 621, 'HealthDamage', '20');

INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (17, 621, 'FocusDamage', '200');

-- Strike (594) / effect 656: shipped via PR #493 (MeleePhysicalDamage).
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (18, 656, 'HealthDamage', '10');

INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (19, 656, 'FocusDamage', '100');

-- Health Heal (1646) / effect 2008: shipped via PR #496.
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (100, 2008, 'HealPercentage', '10.00');

-- Wires effect 1383 (Medical Attention: Recuperation — ability 1218,
-- 75% Health heal over 25 seconds) to the existing `HealHealth` script.
-- The effect already has `pulse_count = 25` and `pulse_duration = 1`
-- on its row in effects.sql.
--
-- Pulse-count accounting (verified against
-- `crates/cell-combat/src/cell/effects/pulsing.rs:118-119`):
--   - `damage_apply` fires the initial pulse synchronously
--     (counts as pulse 1 of 25 — NOT an extra pulse)
--   - `register_active_effect` schedules `remaining = pulse_count - 1`
--     follow-up pulses (24 in this case) at `pulse_duration` intervals
--   - Total: 1 initial + 24 follow-ups = 25 pulses × 3% = 75% of max HP
--     over 25 seconds. Matches the effect's `effect_desc` exactly.
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (200, 1383, 'HealPercentage', '3.00');

-- Pets PT-08 (ids 350-359): magnitudes for the owner abilities that act on
-- pets. The 2009 rows shipped no NVPs, so each value is the one in the
-- effect's own description (crates/cell-world/src/cell/effects/pet_scripts.rs):
--   4220 Holy Warrior "Toggled: +100 Accuracy -100 Defense" (PetStatBuff)
--   4121 To The Death "Accuracy +400" for its 60 s pulse_duration (PetStatBuff)
--   350  Lord's Concentration (new server-only row, D-PT17) +50 interruptRes (PetStatBuff)
--   3211 Repair Turret: Percentage "Heal 20% of target's Health pool" (the ability
--        tooltip says 15%; the effect row is what executes) (HealPetHealth)
--   3230 Repair Turret: Regenerate "+5% 15 Ticks": 15 pulses x 5% (HealPetHealth)
--   3350 Repair Turret: Full "+100% Health": 10 pulses x 10% (HealPetHealth)
--   4968 Heed Our Calling "Pet Summon Speed increase": speedPet +100, so a
--        SpeedPet summon's warmup scales to 0 ("Summons Chosen Pet Instantly",
--        D-PT10) (PetSummonSpeed)
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (350, 4220, 'Accuracy', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (351, 4220, 'Defense', '-100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (352, 4121, 'Accuracy', '400');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (353, 350, 'InterruptResistance', '50');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (354, 3211, 'HealPercentage', '20.00');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (355, 3230, 'HealPercentage', '5.00');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (356, 3350, 'HealPercentage', '10.00');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (357, 4968, 'SpeedPet', '100');

-- Native consumables (items_event_sets event 5, cell::content::consumable_use;
-- ids 400-462). The 2009 rows shipped no NVPs, so each value is the number
-- in the effect's own effect_desc, and each effect's script_name is set in
-- effects.sql:
--   HealAmount (HealHealth / HealFocus): the flat heal, "Heals 500 health."
--     -> 500. Health: 712 (Health Slappack TC1), 3125, 3249-3257.
--     Focus: 3062, 3239-3241, 3243-3248.
--   <Stat> (StatBuff): the stimpack magnitude, "+7 Coordination" -> 7, for
--     the effect's 3600 s pulse_duration. Mark III 3949-3954, Mark V
--     3955-3966, Mark VII 3967-3978, Mark X 3979-3990. "Intellect" moves the
--     INTELLIGENCE stat.
-- The Stealth (3221) and Energy (3227) boosts and the Disguise boosts stay
-- unwired: nothing on the server reads those stats (see
-- docs/content/consumable-via-onitemuse-pattern.md).
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (400, 712, 'HealAmount', '500');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (401, 3125, 'HealAmount', '162');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (402, 3249, 'HealAmount', '172');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (403, 3250, 'HealAmount', '182');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (404, 3251, 'HealAmount', '221');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (405, 3252, 'HealAmount', '232');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (406, 3253, 'HealAmount', '244');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (407, 3254, 'HealAmount', '278');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (408, 3255, 'HealAmount', '290');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (409, 3256, 'HealAmount', '339');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (410, 3257, 'HealAmount', '353');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (411, 3062, 'HealAmount', '384');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (412, 3239, 'HealAmount', '454');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (413, 3240, 'HealAmount', '524');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (414, 3241, 'HealAmount', '683');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (415, 3243, 'HealAmount', '764');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (416, 3244, 'HealAmount', '844');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (417, 3245, 'HealAmount', '1005');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (418, 3246, 'HealAmount', '1005');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (419, 3247, 'HealAmount', '1322');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (420, 3248, 'HealAmount', '1420');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (421, 3949, 'Morale', '5');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (422, 3950, 'Coordination', '5');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (423, 3951, 'Engagement', '5');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (424, 3952, 'Fortitude', '5');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (425, 3953, 'Intellect', '5');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (426, 3954, 'Perception', '5');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (427, 3955, 'Engagement', '3');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (428, 3956, 'Coordination', '7');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (429, 3957, 'Engagement', '7');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (430, 3958, 'Perception', '3');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (431, 3959, 'Fortitude', '7');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (432, 3960, 'Coordination', '3');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (433, 3961, 'Intellect', '7');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (434, 3962, 'Morale', '3');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (435, 3963, 'Morale', '7');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (436, 3964, 'Fortitude', '3');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (437, 3965, 'Perception', '7');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (438, 3966, 'Intellect', '3');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (439, 3967, 'Coordination', '7');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (440, 3968, 'Engagement', '7');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (441, 3969, 'Perception', '7');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (442, 3970, 'Engagement', '7');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (443, 3971, 'Fortitude', '7');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (444, 3972, 'Coordination', '7');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (445, 3973, 'Intellect', '7');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (446, 3974, 'Morale', '7');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (447, 3975, 'Fortitude', '7');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (448, 3976, 'Morale', '7');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (449, 3977, 'Perception', '7');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (450, 3978, 'Intellect', '7');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (451, 3979, 'Coordination', '10');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (452, 3980, 'Engagement', '10');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (453, 3981, 'Engagement', '10');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (454, 3982, 'Perception', '10');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (455, 3983, 'Fortitude', '10');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (456, 3984, 'Coordination', '10');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (457, 3985, 'Intellect', '10');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (458, 3986, 'Morale', '10');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (459, 3987, 'Morale', '10');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (460, 3988, 'Fortitude', '10');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (461, 3989, 'Perception', '10');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (462, 3990, 'Intellect', '10');
-- Deployables Phase 0 (ids 380-389): 5066 "Damage", the pulse of 1012
-- Deployable: Microwave Emitter. Its description is "Medium Radius AE /
-- Secondary -100F": 100 Focus and no Health, written the way the "-100F
-- -10H" rows above (641, 656) are. Its script (effects.sql) is the
-- Focus-first RangedPhysicalDamage, as for 641, so a target's Focus goes
-- first and the overflow bleeds into Health.
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (380, 5066, 'FocusDamage', '100');

-- ability-mechanics generated heal begin
-- GENERATED by tools/ability_mechanics/effect_nvps_from_desc.py (family 'heal', nvp_id 20000-20999).
-- Do not edit by hand: change the parser and regenerate. Rows outside these
-- markers are hand-authored and the tool never touches them.
-- RECONSTRUCTION: the 2009 rows shipped no NVP for these effects. Each value
-- is read from the effect's own effect_desc, quoted below (\n = line break);
-- it is not recovered server data.
-- The same run sets each effect's script_name in effects.sql.
-- RECONSTRUCTION 788 Field Medic I Heal (ability 742), effect_desc "+10% Health"
--   from "+10% Health" -> HealHealth, HealPercentage 10.00
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (20000, 788, 'HealPercentage', '10.00');
-- RECONSTRUCTION 789 Field Medic II Heal (ability 743), effect_desc "+20% Health"
--   from "+20% Health" -> HealHealth, HealPercentage 20.00
--   note: the ability tooltip says "Heals 10% of the player's Health pool"; the effect row is what executes (as for 3211)
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (20001, 789, 'HealPercentage', '20.00');
-- RECONSTRUCTION 834 Restore: Concentration Focus Heal (ability 788), effect_desc "+10% Focus"
--   from "+10% Focus" -> HealFocus, HealPercentage 10.00
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (20002, 834, 'HealPercentage', '10.00');
-- RECONSTRUCTION 835 Focus Heal (ability 789), effect_desc "Heals 20% of target's Focus pool"
--   from "Heals 20% of target's Focus pool" -> HealFocus, HealPercentage 20.00
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (20003, 835, 'HealPercentage', '20.00');
-- RECONSTRUCTION 939 Rally User Focus heal (ability 869), effect_desc "Single Target\n35% Focus Heal"
--   from "35% Focus Heal" -> HealFocus, HealPercentage 35.00
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (20004, 939, 'HealPercentage', '35.00');
-- RECONSTRUCTION 1040 Heal (ability 946), effect_desc "Single Target\nTarget +35% Focus"
--   from "Target +35% Focus" -> HealFocus, HealPercentage 35.00
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (20005, 1040, 'HealPercentage', '35.00');
-- RECONSTRUCTION 1044 Direct Heal (ability 948), effect_desc "Single Target\nTarget +10% Health"
--   from "Target +10% Health" -> HealHealth, HealPercentage 10.00
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (20006, 1044, 'HealPercentage', '10.00');
-- RECONSTRUCTION 1877 Focus Heal (ability 1554), effect_desc "Heals 35% of players Focus pool"
--   from "Heals 35% of players Focus pool" -> HealFocus, HealPercentage 35.00
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (20007, 1877, 'HealPercentage', '35.00');
-- RECONSTRUCTION 1878 Focus Heal (ability 1555), effect_desc "Heals 35% of players Focus pool"
--   from "Heals 35% of players Focus pool" -> HealFocus, HealPercentage 35.00
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (20008, 1878, 'HealPercentage', '35.00');
-- RECONSTRUCTION 2009 Focus Heal (ability 1647), effect_desc "Heals 35% of players Focus pool"
--   from "Heals 35% of players Focus pool" -> HealFocus, HealPercentage 35.00
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (20009, 2009, 'HealPercentage', '35.00');
-- RECONSTRUCTION 2014 Focus Heal (ability 1651), effect_desc "Heals 35% of target's Focus pool"
--   from "Heals 35% of target's Focus pool" -> HealFocus, HealPercentage 35.00
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (20010, 2014, 'HealPercentage', '35.00');
-- RECONSTRUCTION 4085 Lord's Vitae Heal (ability 2823), effect_desc "Heals 10% of target's Health pool\nChanneled: 1 Second interval\nEnergy -25"
--   from "Heals 10% of target's Health pool" -> HealHealth, HealPercentage 10.00
--   note: "Energy -25" is a cost; no ability cost is modelled (B-04)
--   note: 10% per pulse, 20 pulses of 1 s (200% in total)
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (20011, 4085, 'HealPercentage', '10.00');
-- ability-mechanics generated heal end

-- ability-mechanics generated damage begin
-- GENERATED by tools/ability_mechanics/effect_nvps_from_desc.py (family 'damage', nvp_id 21000-22999).
-- Do not edit by hand: change the parser and regenerate. Rows outside these
-- markers are hand-authored and the tool never touches them.
-- RECONSTRUCTION: the 2009 rows shipped no NVP for these effects. Each value
-- is read from the effect's own effect_desc, quoted below (\n = line break);
-- it is not recovered server data.
-- No script is bound: the server's NVP path reads these rows.
-- RECONSTRUCTION 660 Quick Burst Damage (ability 598), effect_desc "Single Target\n-200F / -20H"
--   from "-200F / -20H", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21000, 660, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21001, 660, 'HealthDamage', '20');
-- RECONSTRUCTION 674 Area Burst Single Target Damage (ability 612), effect_desc "Single Target Damage\n-200F / -20H"
--   from "-200F / -20H", FocusDamage 200, HealthDamage 20
--   note: the ability tooltip says "-300F / -30H"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21002, 674, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21003, 674, 'HealthDamage', '20');
-- RECONSTRUCTION 694 Longburst Damage: Secondary (ability 632), effect_desc "Cone: Beam\nF-200 H-20"
--   from "F-200 H-20", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21004, 694, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21005, 694, 'HealthDamage', '20');
-- RECONSTRUCTION 704 Default Damage (ability 641), effect_desc "Single Target\nTarget -100F / -10H"
--   from "Target -100F / -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21006, 704, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21007, 704, 'HealthDamage', '10');
-- RECONSTRUCTION 715 Wound Damage (ability 651), effect_desc "Melee Damage\n-100F -10H"
--   from "-100F -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21008, 715, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21009, 715, 'HealthDamage', '10');
-- RECONSTRUCTION 719 Shotgun Target Damage (ability 654), effect_desc "Single Target Damage\n-200F / -20H"
--   from "-200F / -20H", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21010, 719, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21011, 719, 'HealthDamage', '20');
-- RECONSTRUCTION 720 Rifle Auto Attack Damage (ability 655), effect_desc "Single Target\nTarget -300F / -30H"
--   from "Target -300F / -30H", FocusDamage 300, HealthDamage 30
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21012, 720, 'FocusDamage', '300');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21013, 720, 'HealthDamage', '30');
-- RECONSTRUCTION 722 Interrupting Shot Damage (ability 657), effect_desc "Single Target\n-100F / -10H"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21014, 722, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21015, 722, 'HealthDamage', '10');
-- RECONSTRUCTION 724 Direct Damage (ability 658), effect_desc "Wide Cone\n-100F / -10H"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21016, 724, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21017, 724, 'HealthDamage', '10');
-- RECONSTRUCTION 732 Pistol Half Mag Damage (ability 706), effect_desc "Single Target Channeled: 7 ticks\n-100F\n-10H"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
--   note: per tick, 7 ticks of 0.8 s
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21018, 732, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21019, 732, 'HealthDamage', '10');
-- RECONSTRUCTION 739 Direct Damage (ability 713), effect_desc "Single Target\n-200F / -20H"
--   from "-200F / -20H", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21020, 739, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21021, 739, 'HealthDamage', '20');
-- RECONSTRUCTION 742 Wounding Shot Damage (ability 716), effect_desc "Single Target\n-100F\n-10H"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21022, 742, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21023, 742, 'HealthDamage', '10');
-- RECONSTRUCTION 744 SnareShot Damage (ability 717), effect_desc "Single Target\n-100F / -10H"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21024, 744, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21025, 744, 'HealthDamage', '10');
-- RECONSTRUCTION 751 Spray and Pray Target DoT (ability 722), effect_desc "Single Target\n-50F / -5H DoT: 10 Ticks"
--   from "-50F / -5H DoT: 10 Ticks", FocusDamage 50, HealthDamage 5
--   note: per tick, 10 ticks of 1 s
--   note: the ability tooltip says "-100F / -10H"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21026, 751, 'FocusDamage', '50');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21027, 751, 'HealthDamage', '5');
-- RECONSTRUCTION 752 Spray and Pray Cone Damage (ability 722), effect_desc "Medium Cone\n-100F / -10H"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21028, 752, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21029, 752, 'HealthDamage', '10');
-- RECONSTRUCTION 818 Disruption Shot Damage (ability 774), effect_desc "Single Target\n-150F / -15H"
--   from "-150F / -15H", FocusDamage 150, HealthDamage 15
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21030, 818, 'FocusDamage', '150');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21031, 818, 'HealthDamage', '15');
-- RECONSTRUCTION 853 Cover Denial AE Damage (ability 808), effect_desc "Medium Radius AE\n-300F / -30H"
--   from "-300F / -30H", FocusDamage 300, HealthDamage 30
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21032, 853, 'FocusDamage', '300');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21033, 853, 'HealthDamage', '30');
-- RECONSTRUCTION 864 Sustained Sweep Targeted Damage (ability 818), effect_desc "Single Target Channeled Damage\n-100F / -10H per Pulse\n-5 Ammo per Pulse"
--   from "-100F / -10H per Pulse", FocusDamage 100, HealthDamage 10
--   note: "-5 Ammo per Pulse" is a cost; no ability cost is modelled (B-04)
--   note: per tick on a single-shot row: one application deals one tick (channel ticks are not modelled)
--   note: the ability tooltip says "-50F / -5H per pulse"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21034, 864, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21035, 864, 'HealthDamage', '10');
-- RECONSTRUCTION 865 Sustained Sweep Cone Damage (ability 818), effect_desc "Medium Cone Damage\n-100F / -10H per Pulse"
--   from "-100F / -10H per Pulse", FocusDamage 100, HealthDamage 10
--   note: per tick on a single-shot row: one application deals one tick (channel ticks are not modelled)
--   note: the ability tooltip says "-50F / -5H per pulse"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21036, 865, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21037, 865, 'HealthDamage', '10');
-- RECONSTRUCTION 866 Steady Aim Single Target Damage (ability 819), effect_desc "Single Target Channeled: 20 Ticks\n-150F / -10H per Tick\n5 Ammo per Tick"
--   from "-150F / -10H per Tick", FocusDamage 150, HealthDamage 10
--   note: "5 Ammo per Tick" is a cost; no ability cost is modelled (B-04)
--   note: per tick, 20 ticks of 0.5 s
--   note: the ability tooltip says "-1500F / -150H"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21038, 866, 'FocusDamage', '150');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21039, 866, 'HealthDamage', '10');
-- RECONSTRUCTION 904 Repeated Fire Channeled Damage (ability 848), effect_desc "Single Target Channeled\nTarget -500F / -50H"
--   from "Target -500F / -50H", FocusDamage 500, HealthDamage 50
--   note: per tick on a single-shot row: one application deals one tick (channel ticks are not modelled)
--   note: the ability tooltip says "-300F / -30H"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21040, 904, 'FocusDamage', '500');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21041, 904, 'HealthDamage', '50');
-- RECONSTRUCTION 906 Scattershot Cone Damage (ability 850), effect_desc "Medium Narrow Cone\n-200F\n-20H"
--   from "-200F / -20H", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21042, 906, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21043, 906, 'HealthDamage', '20');
-- RECONSTRUCTION 908 High Explosive Mortar Damage (ability 852), effect_desc "Medium Radius AE\n-800F / -80H"
--   from "-800F / -80H", FocusDamage 800, HealthDamage 80
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21044, 908, 'FocusDamage', '800');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21045, 908, 'HealthDamage', '80');
-- RECONSTRUCTION 911 Fire Zone Cone Damage (ability 853), effect_desc "Medium Cone\n-500F / -50H"
--   from "-500F / -50H", FocusDamage 500, HealthDamage 50
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21046, 911, 'FocusDamage', '500');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21047, 911, 'HealthDamage', '50');
-- RECONSTRUCTION 916 Aimed Shot: Leg Damage (ability 855), effect_desc "Single Target\nTarget -500F / -50H"
--   from "Target -500F / -50H", FocusDamage 500, HealthDamage 50
--   note: the ability tooltip says "-200F / -20H"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21048, 916, 'FocusDamage', '500');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21049, 916, 'HealthDamage', '50');
-- RECONSTRUCTION 919 Takedown Damage (ability 856), effect_desc "Single Target Melee Damage\n-100F / -10H"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21050, 919, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21051, 919, 'HealthDamage', '10');
-- RECONSTRUCTION 927 Offensive Grenade Damage (ability 861), effect_desc "Short Radius AE\n-800 F / -80 H"
--   from "-800 F / -80 H", FocusDamage 800, HealthDamage 80
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21052, 927, 'FocusDamage', '800');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21053, 927, 'HealthDamage', '80');
-- RECONSTRUCTION 930 Direct Damage (ability 863), effect_desc "Single Target\n-100F / -10H"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21054, 930, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21055, 930, 'HealthDamage', '10');
-- RECONSTRUCTION 943 Sap Damage (ability 873), effect_desc "Single Target\n-100F / -10H"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21056, 943, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21057, 943, 'HealthDamage', '10');
-- RECONSTRUCTION 975 Direct Damage (ability 891), effect_desc "Single Target\n-100F"
--   from "-100F", FocusDamage 100
--   note: the ability tooltip says "-100F / -10H"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21058, 975, 'FocusDamage', '100');
-- RECONSTRUCTION 1103 Pistol Half Magazine Damage (ability 993), effect_desc "Single Target Channeled: 7 ticks\n-100F\n-10H"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
--   note: per tick, 7 ticks of 0.8 s
--   note: the ability tooltip says "-700F / -70H"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21059, 1103, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21060, 1103, 'HealthDamage', '10');
-- RECONSTRUCTION 1106 Pistol Full Magazine Effect (ability 997), effect_desc "-100F\n-10H\n50 Ammo"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
--   note: "50 Ammo" is a cost; no ability cost is modelled (B-04)
--   note: the ability tooltip says "-1500F / -150H"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21061, 1106, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21062, 1106, 'HealthDamage', '10');
-- RECONSTRUCTION 1124 Shotgun Cone Damage (ability 654), effect_desc "Medium Narrow Cone\n-200F / -20H"
--   from "-200F / -20H", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21063, 1124, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21064, 1124, 'HealthDamage', '20');
-- RECONSTRUCTION 1194 Area Burst Cone Damage (ability 612), effect_desc "Cone Damage\n-200F / -20H"
--   from "-200F / -20H", FocusDamage 200, HealthDamage 20
--   note: the ability tooltip says "-300F / -30H"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21065, 1194, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21066, 1194, 'HealthDamage', '20');
-- RECONSTRUCTION 1196 Direct Damage (ability 658), effect_desc "Single Target\n-100F / -10H"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21067, 1196, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21068, 1196, 'HealthDamage', '10');
-- RECONSTRUCTION 1199 Anti-Personnel Mortar AE Damage (ability 724), effect_desc "Short Radius AE\n- 800F / - 80H"
--   from "- 800F / - 80H", FocusDamage 800, HealthDamage 80
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21069, 1199, 'FocusDamage', '800');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21070, 1199, 'HealthDamage', '80');
-- RECONSTRUCTION 1216 Cover Fire Target Damage (ability 720), effect_desc "Single Target\n-100F / -10H"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21071, 1216, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21072, 1216, 'HealthDamage', '10');
-- RECONSTRUCTION 1217 Cover Fire Cone Damage (ability 720), effect_desc "Medium Cone\n-100F / -10H"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21073, 1217, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21074, 1217, 'HealthDamage', '10');
-- RECONSTRUCTION 1219 Spray and Pray Target Damage (ability 722), effect_desc "Single Target\n-100F / -10H"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21075, 1219, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21076, 1219, 'HealthDamage', '10');
-- RECONSTRUCTION 1336 Drone Shot Damage (ability 1174), effect_desc "F-175\nH-20"
--   from "F-175 / H-20", FocusDamage 175, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21077, 1336, 'FocusDamage', '175');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21078, 1336, 'HealthDamage', '20');
-- RECONSTRUCTION 1382 Direct Damage (ability 1217), effect_desc "Single Target\nTarget F-100 / H-10"
--   from "Target F-100 / H-10", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21079, 1382, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21080, 1382, 'HealthDamage', '10');
-- RECONSTRUCTION 1389 Incendiary Strike Damage (ability 1229), effect_desc "Defensive Grenade Damage:\n-500 F / -50 H"
--   from "-500 F / -50 H", FocusDamage 500, HealthDamage 50
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21081, 1389, 'FocusDamage', '500');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21082, 1389, 'HealthDamage', '50');
-- RECONSTRUCTION 1391 Gamma Strike Damage (ability 1230), effect_desc "Gamma Strike Damage:\n-500 F / -50 H"
--   from "-500 F / -50 H", FocusDamage 500, HealthDamage 50
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21083, 1391, 'FocusDamage', '500');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21084, 1391, 'HealthDamage', '50');
-- RECONSTRUCTION 1401 Aimed Shot: Arm Damage (ability 1242), effect_desc "Targeted\n-500F / -50H"
--   from "-500F / -50H", FocusDamage 500, HealthDamage 50
--   note: the ability tooltip says "-300F / -30H"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21085, 1401, 'FocusDamage', '500');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21086, 1401, 'HealthDamage', '50');
-- RECONSTRUCTION 1409 Stealthed Stike Damage (ability 1246), effect_desc "-200F -20H"
--   from "-200F -20H", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21087, 1409, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21088, 1409, 'HealthDamage', '20');
-- RECONSTRUCTION 1455 Direct Damage (ability 523), effect_desc "Melee Radius AE\n-500 F / -50 H"
--   from "-500 F / -50 H", FocusDamage 500, HealthDamage 50
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21089, 1455, 'FocusDamage', '500');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21090, 1455, 'HealthDamage', '50');
-- RECONSTRUCTION 1463 C-4: Time Delay Damage (ability 647), effect_desc "Small Radius AE\n-800F / -80H"
--   from "-800F / -80H", FocusDamage 800, HealthDamage 80
--   note: the ability tooltip says "-500F / -50H"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21091, 1463, 'FocusDamage', '800');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21092, 1463, 'HealthDamage', '80');
-- RECONSTRUCTION 1568 Medium Cone Damage (ability 1331), effect_desc "Medium Cone\nSecondary -100F / -10H"
--   from "Secondary -100F / -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21093, 1568, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21094, 1568, 'HealthDamage', '10');
-- RECONSTRUCTION 1570 Single Target Damage (ability 1331), effect_desc "Single Target\nTarget -100F / -10H"
--   from "Target -100F / -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21095, 1570, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21096, 1570, 'HealthDamage', '10');
-- RECONSTRUCTION 1574 Direct Damage (ability 1332), effect_desc "Medium Narrow Cone\n-100F / -10H"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21097, 1574, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21098, 1574, 'HealthDamage', '10');
-- RECONSTRUCTION 1575 Direct Damage (ability 1332), effect_desc "Melee Narrow Cone\n-300F / -30H"
--   from "-300F / -30H", FocusDamage 300, HealthDamage 30
--   note: the ability tooltip says "-100F / -10H"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21099, 1575, 'FocusDamage', '300');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21100, 1575, 'HealthDamage', '30');
-- RECONSTRUCTION 1595 Direct Damage (ability 1354), effect_desc "Single Target\nTarget -250F / -25H"
--   from "Target -250F / -25H", FocusDamage 250, HealthDamage 25
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21101, 1595, 'FocusDamage', '250');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21102, 1595, 'HealthDamage', '25');
-- RECONSTRUCTION 1597 Direct Damage (ability 1355), effect_desc "Target\n-500F / -50H"
--   from "-500F / -50H", FocusDamage 500, HealthDamage 50
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21103, 1597, 'FocusDamage', '500');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21104, 1597, 'HealthDamage', '50');
-- RECONSTRUCTION 1600 Direct Damage (ability 1356), effect_desc "Target\n-300F / -30H"
--   from "-300F / -30H", FocusDamage 300, HealthDamage 30
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21105, 1600, 'FocusDamage', '300');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21106, 1600, 'HealthDamage', '30');
-- RECONSTRUCTION 1601 DOT (ability 1356), effect_desc "Target\n-100F / -10H\n8 Ticks x1 Second"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
--   note: per tick, 8 ticks of 1 s
--   note: the ability tooltip says "-300F /-30H"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21107, 1601, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21108, 1601, 'HealthDamage', '10');
-- RECONSTRUCTION 1603 Direct Damage (ability 1357), effect_desc "Target\n-100F / -10H"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21109, 1603, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21110, 1603, 'HealthDamage', '10');
-- RECONSTRUCTION 1605 Direct Damage (ability 1358), effect_desc "Single Target\n-200F / -20H"
--   from "-200F / -20H", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21111, 1605, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21112, 1605, 'HealthDamage', '20');
-- RECONSTRUCTION 1606 DoT (ability 1358), effect_desc "Target\n-100F / -10H\n10 Ticks x1 Sec"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
--   note: per tick, 10 ticks of 1 s
--   note: the ability tooltip says "-200F / -20H"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21113, 1606, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21114, 1606, 'HealthDamage', '10');
-- RECONSTRUCTION 1607 Direct Damage (ability 1359), effect_desc "Single Target\n-500F / -50H"
--   from "-500F / -50H", FocusDamage 500, HealthDamage 50
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21115, 1607, 'FocusDamage', '500');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21116, 1607, 'HealthDamage', '50');
-- RECONSTRUCTION 1620 Direct Damage (ability 1364), effect_desc "Secondary\n-100F / -10H"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21117, 1620, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21118, 1620, 'HealthDamage', '10');
-- RECONSTRUCTION 1764 Direct Damage (ability 1474), effect_desc "Single Target\nTarget -200F / -20H"
--   from "Target -200F / -20H", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21119, 1764, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21120, 1764, 'HealthDamage', '20');
-- RECONSTRUCTION 1767 Direct Damage (ability 1475), effect_desc "Medium Cone\nSecondary -100F / -10H"
--   from "Secondary -100F / -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21121, 1767, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21122, 1767, 'HealthDamage', '10');
-- RECONSTRUCTION 1770 No Rest for the Weary Cone damage (ability 1476), effect_desc "Secondary Target\n-200F\n-20H"
--   from "-200F / -20H", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21123, 1770, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21124, 1770, 'HealthDamage', '20');
-- RECONSTRUCTION 1772 No Rest for the Weary Target Damage (ability 1476), effect_desc "Single Target\n-200F\n-20H"
--   from "-200F / -20H", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21125, 1772, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21126, 1772, 'HealthDamage', '20');
-- RECONSTRUCTION 1781 Cluster Mortar Damage (ability 1485), effect_desc "Large Radius AE\n- 800 F / - 80 H"
--   from "- 800 F / - 80 H", FocusDamage 800, HealthDamage 80
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21127, 1781, 'FocusDamage', '800');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21128, 1781, 'HealthDamage', '80');
-- RECONSTRUCTION 1785 Rain of Steel Cone Damage (ability 1486), effect_desc "Wide Cone\n-500F / -50H"
--   from "-500F / -50H", FocusDamage 500, HealthDamage 50
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21129, 1785, 'FocusDamage', '500');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21130, 1785, 'HealthDamage', '50');
-- RECONSTRUCTION 1956 Back Slash Damae: Non-Positional (ability 1613), effect_desc "-200F / -20H"
--   from "-200F / -20H", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21131, 1956, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21132, 1956, 'HealthDamage', '20');
-- RECONSTRUCTION 1963 Onslaught Damage (ability 1620), effect_desc "-200F / -20H\n10 Pulses"
--   from "-200F / -20H", FocusDamage 200, HealthDamage 20
--   note: per tick, 10 ticks of 0.5 s
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21133, 1963, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21134, 1963, 'HealthDamage', '20');
-- RECONSTRUCTION 1965 Paralyze Damage (ability 1621), effect_desc "-100F / -10H"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
--   note: the ability tooltip says "-200F / -20H"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21135, 1965, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21136, 1965, 'HealthDamage', '10');
-- RECONSTRUCTION 1974 Direct Damage (ability 1626), effect_desc "Single Target\n-100F / -10H"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21137, 1974, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21138, 1974, 'HealthDamage', '10');
-- RECONSTRUCTION 1977 Disruption Beam Damage (ability 1627), effect_desc "Single Target\n-200F\n-20H"
--   from "-200F / -20H", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21139, 1977, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21140, 1977, 'HealthDamage', '20');
-- RECONSTRUCTION 1978 Ribbon Device Disruption Stream Damage (ability 1628), effect_desc "Single Target:\n-150F -15H"
--   from "-150F -15H", FocusDamage 150, HealthDamage 15
--   note: the ability tooltip says "-100F / -10H"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21141, 1978, 'FocusDamage', '150');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21142, 1978, 'HealthDamage', '15');
-- RECONSTRUCTION 1986 Destruction Shot Damage (ability 1638), effect_desc "Single Target\n-150F / -15H"
--   from "-150F / -15H", FocusDamage 150, HealthDamage 15
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21143, 1986, 'FocusDamage', '150');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21144, 1986, 'HealthDamage', '15');
-- RECONSTRUCTION 1993 Terror Stone Grenade Damage (ability 1640), effect_desc "Short Radius AE\n-500 F / -50 H"
--   from "-500 F / -50 H", FocusDamage 500, HealthDamage 50
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21145, 1993, 'FocusDamage', '500');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21146, 1993, 'HealthDamage', '50');
-- RECONSTRUCTION 1994 Terror Pod Damage (ability 1641), effect_desc "Medium Radius AE\n-800 F / -80 H"
--   from "-800 F / -80 H", FocusDamage 800, HealthDamage 80
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21147, 1994, 'FocusDamage', '800');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21148, 1994, 'HealthDamage', '80');
-- RECONSTRUCTION 2136 Cone Damage (ability 1733), effect_desc "Cone Damage:\n-100 F -10H"
--   from "-100 F -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21149, 2136, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21150, 2136, 'HealthDamage', '10');
-- RECONSTRUCTION 2137 Strike Damage (ability 1734), effect_desc "-125F -15H"
--   from "-125F -15H", FocusDamage 125, HealthDamage 15
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21151, 2137, 'FocusDamage', '125');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21152, 2137, 'HealthDamage', '15');
-- RECONSTRUCTION 2138 Blinding Shot Attack Damage (ability 1736), effect_desc "Single Target\n-120F\n-20H"
--   from "-120F / -20H", FocusDamage 120, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21153, 2138, 'FocusDamage', '120');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21154, 2138, 'HealthDamage', '20');
-- RECONSTRUCTION 2140 Ground Blast Damage (ability 1482), effect_desc "Secondary Targets\n-300F / -30H"
--   from "-300F / -30H", FocusDamage 300, HealthDamage 30
--   note: the ability tooltip says "-500F / -50H"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21155, 2140, 'FocusDamage', '300');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21156, 2140, 'HealthDamage', '30');
-- RECONSTRUCTION 2254 Aimed Burst Damage (ability 1005), effect_desc "-350 F\n-35 H"
--   from "-350 F / -35 H", FocusDamage 350, HealthDamage 35
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21157, 2254, 'FocusDamage', '350');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21158, 2254, 'HealthDamage', '35');
-- RECONSTRUCTION 2393 Pistol Shot Damage (ability 1879), effect_desc "Single Target\n-200F / -20H"
--   from "-200F / -20H", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21159, 2393, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21160, 2393, 'HealthDamage', '20');
-- RECONSTRUCTION 2394 Pistol Shot DOT (ability 1879), effect_desc "DOT: -150F -30H (8 Ticks)"
--   from "DOT: -150F -30H (8 Ticks)", FocusDamage 150, HealthDamage 30
--   note: per tick, 8 ticks of 1 s
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21161, 2394, 'FocusDamage', '150');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21162, 2394, 'HealthDamage', '30');
-- RECONSTRUCTION 2404 Incinerate Target Targeted Damage (ability 1883), effect_desc "Damage\n-100F / -10H per Pulse\n-5 Ammo per Pulse"
--   from "-100F / -10H per Pulse", FocusDamage 100, HealthDamage 10
--   note: "-5 Ammo per Pulse" is a cost; no ability cost is modelled (B-04)
--   note: per tick on a single-shot row: one application deals one tick (channel ticks are not modelled)
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21163, 2404, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21164, 2404, 'HealthDamage', '10');
-- RECONSTRUCTION 2405 Incinerate Target Cone Damage (ability 1883), effect_desc "Medium Cone\n-100F / -10H per Pulse"
--   from "-100F / -10H per Pulse", FocusDamage 100, HealthDamage 10
--   note: per tick on a single-shot row: one application deals one tick (channel ticks are not modelled)
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21165, 2405, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21166, 2405, 'HealthDamage', '10');
-- RECONSTRUCTION 2406 DOT (ability 1883), effect_desc "-50F -5H 8 Ticks"
--   from "-50F -5H 8 Ticks", FocusDamage 50, HealthDamage 5
--   note: per tick, 8 ticks of 1 s
--   note: the ability tooltip says "-150F / -30H per pulse"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21167, 2406, 'FocusDamage', '50');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21168, 2406, 'HealthDamage', '5');
-- RECONSTRUCTION 2411 Focus DOT (ability 1885), effect_desc "Focus DOT\n-50 F\n20 ticks"
--   from "-50 F", FocusDamage 50
--   note: per tick, 20 ticks of 0.5 s
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21169, 2411, 'FocusDamage', '50');
-- RECONSTRUCTION 2422 AOE Damage (ability 1889), effect_desc "F-200\nH-20"
--   from "F-200 / H-20", FocusDamage 200, HealthDamage 20
--   note: the ability tooltip says "-150F / -30H per pulse"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21170, 2422, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21171, 2422, 'HealthDamage', '20');
-- RECONSTRUCTION 2465 Direct Damage (ability 1917), effect_desc "Single Target\n-100F / -10H"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21172, 2465, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21173, 2465, 'HealthDamage', '10');
-- RECONSTRUCTION 2527 Direct Damage (ability 1952), effect_desc "Single Target\nTarget F-100 / H-10"
--   from "Target F-100 / H-10", FocusDamage 100, HealthDamage 10
--   note: the ability tooltip says "-1500F / -150H"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21174, 2527, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21175, 2527, 'HealthDamage', '10');
-- RECONSTRUCTION 2586 StaffSwing Damage (ability 1984), effect_desc "-100F -10H"
--   from "-100F -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21176, 2586, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21177, 2586, 'HealthDamage', '10');
-- RECONSTRUCTION 2611 Threat Increase (ability 1997), effect_desc "Increased Threat +200\nFocus Damage -100"
--   from "Focus Damage -100", FocusDamage 100
--   note: "Increased Threat +200" is not modelled
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21178, 2611, 'FocusDamage', '100');
-- RECONSTRUCTION 2612 Threat Increase (ability 1998), effect_desc "Increased Threat +400\nFocus Damage -100\nCone Narrow"
--   from "Focus Damage -100", FocusDamage 100
--   note: "Increased Threat +400" is not modelled
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21179, 2612, 'FocusDamage', '100');
-- RECONSTRUCTION 2615 Arc of Fury Single Target Damage (ability 2001), effect_desc "Single Target Damage\n-200F / -20H"
--   from "-200F / -20H", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21180, 2615, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21181, 2615, 'HealthDamage', '20');
-- RECONSTRUCTION 2616 Arc of Fury Cone Damage (ability 2001), effect_desc "Cone Damage\n-200F / -20H"
--   from "-200F / -20H", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21182, 2616, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21183, 2616, 'HealthDamage', '20');
-- RECONSTRUCTION 2652 Single Target Damage (ability 1733), effect_desc "-100F -10H"
--   from "-100F -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21184, 2652, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21185, 2652, 'HealthDamage', '10');
-- RECONSTRUCTION 2667 AOE Damage (ability 2025), effect_desc "AOE Damage\n-200 F -20 H"
--   from "-200 F -20 H", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21186, 2667, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21187, 2667, 'HealthDamage', '20');
-- RECONSTRUCTION 2670 Ground Blast Damage (ability 2026), effect_desc "Secondary Targets\n-500F / -50H"
--   from "-500F / -50H", FocusDamage 500, HealthDamage 50
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21188, 2670, 'FocusDamage', '500');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21189, 2670, 'HealthDamage', '50');
-- RECONSTRUCTION 2676 Offensive Grenade Damage (ability 2028), effect_desc "Medium Radius AE\n-1000 F / -100 H"
--   from "-1000 F / -100 H", FocusDamage 1000, HealthDamage 100
--   note: the ability tooltip says "-100 F -100 H"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21190, 2676, 'FocusDamage', '1000');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21191, 2676, 'HealthDamage', '100');
-- RECONSTRUCTION 2677 AOE Damage (ability 2029), effect_desc "AOE Damage\n-200 F -20 H"
--   from "-200 F -20 H", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21192, 2677, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21193, 2677, 'HealthDamage', '20');
-- RECONSTRUCTION 2695 Maximum Blast Damage (ability 2042), effect_desc "Secondary Targets\n-1000F / -100H"
--   from "-1000F / -100H", FocusDamage 1000, HealthDamage 100
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21194, 2695, 'FocusDamage', '1000');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21195, 2695, 'HealthDamage', '100');
-- RECONSTRUCTION 2699 Weapon of Terror Single Target Damage (ability 2043), effect_desc "Single Target Damage\n-400F / -40H"
--   from "-400F / -40H", FocusDamage 400, HealthDamage 40
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21196, 2699, 'FocusDamage', '400');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21197, 2699, 'HealthDamage', '40');
-- RECONSTRUCTION 2700 Weapon of Terror Cone Damage (ability 2043), effect_desc "Cone Damage\n-400F / -40H"
--   from "-400F / -40H", FocusDamage 400, HealthDamage 40
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21198, 2700, 'FocusDamage', '400');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21199, 2700, 'HealthDamage', '40');
-- RECONSTRUCTION 2719 Devastating Blast Damage (ability 2058), effect_desc "Secondary Targets\n-500F / -50H"
--   from "-500F / -50H", FocusDamage 500, HealthDamage 50
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21200, 2719, 'FocusDamage', '500');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21201, 2719, 'HealthDamage', '50');
-- RECONSTRUCTION 2720 DOT Damage (ability 2058), effect_desc "-150 F / -15 H\n10 Ticks"
--   from "-150 F / -15 H", FocusDamage 150, HealthDamage 15
--   note: per tick, 10 ticks of 1 s
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21202, 2720, 'FocusDamage', '150');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21203, 2720, 'HealthDamage', '15');
-- RECONSTRUCTION 2729 Area Blast Damage (ability 2061), effect_desc "Secondary Targets\n-1500F / -150H"
--   from "-1500F / -150H", FocusDamage 1500, HealthDamage 150
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21204, 2729, 'FocusDamage', '1500');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21205, 2729, 'HealthDamage', '150');
-- RECONSTRUCTION 2730 Area Cleanse Damage (ability 2062), effect_desc "Secondary Targets\n-1500F / -150H"
--   from "-1500F / -150H", FocusDamage 1500, HealthDamage 150
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21206, 2730, 'FocusDamage', '1500');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21207, 2730, 'HealthDamage', '150');
-- RECONSTRUCTION 2731 DOT Damage (ability 2062), effect_desc "-150 F / -15 H\n10 Ticks"
--   from "-150 F / -15 H", FocusDamage 150, HealthDamage 15
--   note: per tick, 10 ticks of 1 s
--   note: the ability tooltip says "-1500F / -150H"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21208, 2731, 'FocusDamage', '150');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21209, 2731, 'HealthDamage', '15');
-- RECONSTRUCTION 2738 Area Blast Damage (ability 2065), effect_desc "Secondary Targets\n-1500F / -150H"
--   from "-1500F / -150H", FocusDamage 1500, HealthDamage 150
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21210, 2738, 'FocusDamage', '1500');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21211, 2738, 'HealthDamage', '150');
-- RECONSTRUCTION 2747 Disintegration Damage (First Wave) (ability 2069), effect_desc "F-150 H-15"
--   from "F-150 H-15", FocusDamage 150, HealthDamage 15
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21212, 2747, 'FocusDamage', '150');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21213, 2747, 'HealthDamage', '15');
-- RECONSTRUCTION 2750 Disintegration Damage (Second Wave) (ability 2069), effect_desc "F-200 H-20"
--   from "F-200 H-20", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21214, 2750, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21215, 2750, 'HealthDamage', '20');
-- RECONSTRUCTION 2839 Concussion Grenade Damage (ability 2103), effect_desc "Concussion Grenade Damage:\n- 500 F\n- 50 H"
--   from "- 500 F / - 50 H", FocusDamage 500, HealthDamage 50
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21216, 2839, 'FocusDamage', '500');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21217, 2839, 'HealthDamage', '50');
-- RECONSTRUCTION 2926 Intimidation Fire Single Target Damage (ability 2143), effect_desc "Single Target Damage\n-200F / -20H"
--   from "-200F / -20H", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21218, 2926, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21219, 2926, 'HealthDamage', '20');
-- RECONSTRUCTION 2927 Intimidation Fire Cone Damage (ability 2143), effect_desc "Medium Cone Damage\n-200F / -20H"
--   from "-200F / -20H", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21220, 2927, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21221, 2927, 'HealthDamage', '20');
-- RECONSTRUCTION 3196 Direct Damage (ability 2262), effect_desc "Single Target\n-100F / -10H"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21222, 3196, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21223, 3196, 'HealthDamage', '10');
-- RECONSTRUCTION 3204 Direct Damage (ability 2263), effect_desc "Single Target\n-100F / -10H"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21224, 3204, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21225, 3204, 'HealthDamage', '10');
-- RECONSTRUCTION 3207 Direct Damage (ability 2265), effect_desc "Single Target\n-100F / -10H"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21226, 3207, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21227, 3207, 'HealthDamage', '10');
-- RECONSTRUCTION 3209 Direct Damage (ability 2264), effect_desc "Single Target\n-100F / -10H"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21228, 3209, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21229, 3209, 'HealthDamage', '10');
-- RECONSTRUCTION 3360 Direct Damage (ability 988), effect_desc "Single Target\nTarget -200F / -20H"
--   from "Target -200F / -20H", FocusDamage 200, HealthDamage 20
--   note: the ability tooltip says "-20H"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21230, 3360, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21231, 3360, 'HealthDamage', '20');
-- RECONSTRUCTION 3361 Direct Damage (ability 989), effect_desc "Single Target\nTarget -300F / -30H"
--   from "Target -300F / -30H", FocusDamage 300, HealthDamage 30
--   note: the ability tooltip says "-20H"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21232, 3361, 'FocusDamage', '300');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21233, 3361, 'HealthDamage', '30');
-- RECONSTRUCTION 3460 Defensive Grenade Damage (ability 1885), effect_desc "Defensive Grenade Damage:\n- 100 F\n- 10 H"
--   from "- 100 F / - 10 H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21234, 3460, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21235, 3460, 'HealthDamage', '10');
-- RECONSTRUCTION 3478 DOT (ability 1883), effect_desc "-50F -5H 8 Ticks"
--   from "-50F -5H 8 Ticks", FocusDamage 50, HealthDamage 5
--   note: per tick, 8 ticks of 1 s
--   note: the ability tooltip says "-150F / -30H per pulse"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21236, 3478, 'FocusDamage', '50');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21237, 3478, 'HealthDamage', '5');
-- RECONSTRUCTION 3511 Frag Grenade Damage: Standard (ability 2419), effect_desc "Frag Grenade Damage:\n- 500 F\n- 50 H"
--   from "- 500 F / - 50 H", FocusDamage 500, HealthDamage 50
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21238, 3511, 'FocusDamage', '500');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21239, 3511, 'HealthDamage', '50');
-- RECONSTRUCTION 3514 Explosion: AOE Medium (ability 2105), effect_desc "Grenade Damage:\n-500 F -50 H"
--   from "-500 F -50 H", FocusDamage 500, HealthDamage 50
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21240, 3514, 'FocusDamage', '500');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21241, 3514, 'HealthDamage', '50');
-- RECONSTRUCTION 3536 Direct Damage (ability 2430), effect_desc "Single Target\n-100F / -10H"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21242, 3536, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21243, 3536, 'HealthDamage', '10');
-- RECONSTRUCTION 3885 Shotgun Secondary Target Attack Damage (ability 2694), effect_desc "Medium Narrow Cone\n-200F\n-20H"
--   from "-200F / -20H", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21244, 3885, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21245, 3885, 'HealthDamage', '20');
-- RECONSTRUCTION 3910 Ground Blast Damage (ability 2715), effect_desc "Secondary Targets\n-500F / -50H"
--   from "-500F / -50H", FocusDamage 500, HealthDamage 50
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21246, 3910, 'FocusDamage', '500');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21247, 3910, 'HealthDamage', '50');
-- RECONSTRUCTION 3916 Ground Blast Damage (ability 2717), effect_desc "Secondary Targets\n-500F / -50H"
--   from "-500F / -50H", FocusDamage 500, HealthDamage 50
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21248, 3916, 'FocusDamage', '500');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21249, 3916, 'HealthDamage', '50');
-- RECONSTRUCTION 3920 Strike: Incendiary Damage (ability 2718), effect_desc "Secondary Targets\n-1500F / -150H"
--   from "-1500F / -150H", FocusDamage 1500, HealthDamage 150
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21250, 3920, 'FocusDamage', '1500');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21251, 3920, 'HealthDamage', '150');
-- RECONSTRUCTION 3925 Offensive Grenade Damage (ability 2719), effect_desc "Medium Radius AE\n-1000 F / -100 H"
--   from "-1000 F / -100 H", FocusDamage 1000, HealthDamage 100
--   note: the ability tooltip says "-100 F -100 H"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21252, 3925, 'FocusDamage', '1000');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21253, 3925, 'HealthDamage', '100');
-- RECONSTRUCTION 4005 Direct Damage (ability 2775), effect_desc "Single Target\n-100F / -10H"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21254, 4005, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21255, 4005, 'HealthDamage', '10');
-- RECONSTRUCTION 4054 Lord'sVisage Secondary Target Attack Damage (ability 2795), effect_desc "Medium Narrow Cone\n-200F\n-20H"
--   from "-200F / -20H", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21256, 4054, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21257, 4054, 'HealthDamage', '20');
-- RECONSTRUCTION 4092 Lord'sWill Secondary Target Attack Damage (ability 2827), effect_desc "Medium Narrow Cone\n-200F\n-20H"
--   from "-200F / -20H", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21258, 4092, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21259, 4092, 'HealthDamage', '20');
-- RECONSTRUCTION 4126 Ground Blast Damage (ability 2841), effect_desc "Secondary Targets\n-500F / -50H"
--   from "-500F / -50H", FocusDamage 500, HealthDamage 50
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21260, 4126, 'FocusDamage', '500');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21261, 4126, 'HealthDamage', '50');
-- RECONSTRUCTION 4137 Lord's Presence Damage (ability 2846), effect_desc "Single Target\n-500F\n-50H"
--   from "-500F / -50H", FocusDamage 500, HealthDamage 50
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21262, 4137, 'FocusDamage', '500');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21263, 4137, 'HealthDamage', '50');
-- RECONSTRUCTION 4139 Submit Damage (ability 2848), effect_desc "Single Target\n-100F / -10H per Tick\n10 Energy per pulse"
--   from "-100F / -10H per Tick", FocusDamage 100, HealthDamage 10
--   note: "10 Energy per pulse" is a cost; no ability cost is modelled (B-04)
--   note: per tick on a single-shot row: one application deals one tick (channel ticks are not modelled)
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21264, 4139, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21265, 4139, 'HealthDamage', '10');
-- RECONSTRUCTION 4148 Crippling Slash Damage (ability 2857), effect_desc "-100F / -10H"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21266, 4148, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21267, 4148, 'HealthDamage', '10');
-- RECONSTRUCTION 4152 Cone Damage (ability 2858), effect_desc "Cone Damage:\n-200 F -20H"
--   from "-200 F -20H", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21268, 4152, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21269, 4152, 'HealthDamage', '20');
-- RECONSTRUCTION 4156 Decimation Wound Damage (ability 2860), effect_desc "-100F / -10H"
--   from "-100F / -10H", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21270, 4156, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21271, 4156, 'HealthDamage', '10');
-- RECONSTRUCTION 4173 Disrupt Soul Damage (ability 2869), effect_desc "-200F / -20H\n10 Pulses"
--   from "-200F / -20H", FocusDamage 200, HealthDamage 20
--   note: per tick, 10 ticks of 0.5 s
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21272, 4173, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21273, 4173, 'HealthDamage', '20');
-- RECONSTRUCTION 4178 Single Target Damage (ability 2858), effect_desc "-200F -20H"
--   from "-200F -20H", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21274, 4178, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21275, 4178, 'HealthDamage', '20');
-- RECONSTRUCTION 4181 Prolong Agony Damage (ability 2861), effect_desc "-200F / -20H"
--   from "-200F / -20H", FocusDamage 200, HealthDamage 20
--   note: the ability tooltip says "-100 F -10 H"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21276, 4181, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21277, 4181, 'HealthDamage', '20');
-- RECONSTRUCTION 4182 Inevitable End Damage (ability 2862), effect_desc "-250F / -25H"
--   from "-250F / -25H", FocusDamage 250, HealthDamage 25
--   note: the ability tooltip says "-100 F -10 H"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21278, 4182, 'FocusDamage', '250');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21279, 4182, 'HealthDamage', '25');
-- RECONSTRUCTION 4235 Wound DOT Damage (ability 651), effect_desc "Single Target\n-50F / -5H\n8 Ticks x1sec"
--   from "-50F / -5H", FocusDamage 50, HealthDamage 5
--   note: per tick, 8 ticks of 1 s
--   note: the ability tooltip says "-100F /-10H"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21280, 4235, 'FocusDamage', '50');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21281, 4235, 'HealthDamage', '5');
-- RECONSTRUCTION 4237 Wound DOT Damage (ability 716), effect_desc "Single Target\n-50F / -5H\n8 Ticks x1sec"
--   from "-50F / -5H", FocusDamage 50, HealthDamage 5
--   note: per tick, 8 ticks of 1 s
--   note: the ability tooltip says "-100F / -10H"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21282, 4237, 'FocusDamage', '50');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21283, 4237, 'HealthDamage', '5');
-- RECONSTRUCTION 4409 Direct Damage (ability 1474), effect_desc "Narrow Cone\nSecondary -200F / -20H"
--   from "Secondary -200F / -20H", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21284, 4409, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21285, 4409, 'HealthDamage', '20');
-- RECONSTRUCTION 4522 Long Burst Damage: Single (ability 632), effect_desc "Single Target\n-200F / -20H"
--   from "-200F / -20H", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21286, 4522, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21287, 4522, 'HealthDamage', '20');
-- RECONSTRUCTION 4530 DOT AOE (ability 1889), effect_desc "F-50\nH-5\n8 Ticks"
--   from "F-50 / H-5", FocusDamage 50, HealthDamage 5
--   note: per tick, 8 ticks of 1 s
--   note: the ability tooltip says "-150F / -30H per pulse"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21288, 4530, 'FocusDamage', '50');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21289, 4530, 'HealthDamage', '5');
-- RECONSTRUCTION 4542 DOT Cone (ability 1889), effect_desc "F-50\nH-5\n8 Ticks"
--   from "F-50 / H-5", FocusDamage 50, HealthDamage 5
--   note: per tick, 8 ticks of 1 s
--   note: the ability tooltip says "-150F / -30H per pulse"; the effect row is what executes
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21290, 4542, 'FocusDamage', '50');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21291, 4542, 'HealthDamage', '5');
-- RECONSTRUCTION 4603 Damage (ability 1533), effect_desc "F-200\nH-20"
--   from "F-200 / H-20", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21292, 4603, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21293, 4603, 'HealthDamage', '20');
-- RECONSTRUCTION 4604 Damage (ability 1534), effect_desc "F-100\nH-10"
--   from "F-100 / H-10", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21294, 4604, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21295, 4604, 'HealthDamage', '10');
-- RECONSTRUCTION 4611 Damage (ability 1535), effect_desc "F-150\nH-15"
--   from "F-150 / H-15", FocusDamage 150, HealthDamage 15
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21296, 4611, 'FocusDamage', '150');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21297, 4611, 'HealthDamage', '15');
-- RECONSTRUCTION 4612 Initial Damage (ability 1536), effect_desc "F-200\nH-20"
--   from "F-200 / H-20", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21298, 4612, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21299, 4612, 'HealthDamage', '20');
-- RECONSTRUCTION 4618 PBAoE Damage (ability 1538), effect_desc "F-200\nH-20"
--   from "F-200 / H-20", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21300, 4618, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21301, 4618, 'HealthDamage', '20');
-- RECONSTRUCTION 4621 Focus DOT (ability 1539), effect_desc "Damage Over time F-50\n25 Ticks"
--   from "Damage Over time F-50", FocusDamage 50
--   note: per tick, 25 ticks of 1 s
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21302, 4621, 'FocusDamage', '50');
-- RECONSTRUCTION 4624 PBAOE Toggle Damage (ability 1541), effect_desc "F-100\nH-10"
--   from "F-100 / H-10", FocusDamage 100, HealthDamage 10
--   note: channelled (pulse_count 0): per pulse of 1 s
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21303, 4624, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21304, 4624, 'HealthDamage', '10');
-- RECONSTRUCTION 4628 Damage (ability 1543), effect_desc "F-200 H-20"
--   from "F-200 H-20", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21305, 4628, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21306, 4628, 'HealthDamage', '20');
-- RECONSTRUCTION 4629 Contamination Damage (ability 1544), effect_desc "F-200 H-20"
--   from "F-200 H-20", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21307, 4629, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21308, 4629, 'HealthDamage', '20');
-- RECONSTRUCTION 4631 DOT Damage (ability 1545), effect_desc "5 Pulses\nF-300\nH-30"
--   from "F-300 / H-30", FocusDamage 300, HealthDamage 30
--   note: per tick, 5 ticks of 1 s
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21309, 4631, 'FocusDamage', '300');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21310, 4631, 'HealthDamage', '30');
-- RECONSTRUCTION 4632 Cone 1 (ability 1546), effect_desc "F-150\nH-15"
--   from "F-150 / H-15", FocusDamage 150, HealthDamage 15
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21311, 4632, 'FocusDamage', '150');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21312, 4632, 'HealthDamage', '15');
-- RECONSTRUCTION 4634 Secondary targets (ability 1547), effect_desc "Secondary Damage: F-300 H-30"
--   from "Secondary Damage: F-300 H-30", FocusDamage 300, HealthDamage 30
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21313, 4634, 'FocusDamage', '300');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21314, 4634, 'HealthDamage', '30');
-- RECONSTRUCTION 4648 Damage (ability 1566), effect_desc "Damage:\nF-200 H-20"
--   from "F-200 H-20", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21315, 4648, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21316, 4648, 'HealthDamage', '20');
-- RECONSTRUCTION 4667 PBAoE Damage (ability 1548), effect_desc "F-200 H-20"
--   from "F-200 H-20", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21317, 4667, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21318, 4667, 'HealthDamage', '20');
-- RECONSTRUCTION 4694 AOE Damage (ability 3161), effect_desc "F-100\nH-10"
--   from "F-100 / H-10", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21319, 4694, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21320, 4694, 'HealthDamage', '10');
-- RECONSTRUCTION 4707 Cone 1 (ability 3167), effect_desc "F-150 H-15\nEnergy-15"
--   from "F-150 H-15", FocusDamage 150, HealthDamage 15
--   note: "Energy-15" is a cost; no ability cost is modelled (B-04)
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21321, 4707, 'FocusDamage', '150');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21322, 4707, 'HealthDamage', '15');
-- RECONSTRUCTION 4719 AOE Damage (ability 3169), effect_desc "F-300 H-30"
--   from "F-300 H-30", FocusDamage 300, HealthDamage 30
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21323, 4719, 'FocusDamage', '300');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21324, 4719, 'HealthDamage', '30');
-- RECONSTRUCTION 4728 Damage (ability 3170), effect_desc "F-200\nH-20"
--   from "F-200 / H-20", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21325, 4728, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21326, 4728, 'HealthDamage', '20');
-- RECONSTRUCTION 4729 AOE Damage (ability 3170), effect_desc "F-200 H-20"
--   from "F-200 H-20", FocusDamage 200, HealthDamage 20
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21327, 4729, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21328, 4729, 'HealthDamage', '20');
-- RECONSTRUCTION 4738 AOE Damage (ability 3171), effect_desc "F-300\nH-30\nAOE Radius: Short"
--   from "F-300 / H-30", FocusDamage 300, HealthDamage 30
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21329, 4738, 'FocusDamage', '300');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21330, 4738, 'HealthDamage', '30');
-- RECONSTRUCTION 4749 AOE Damage (ability 3173), effect_desc "AOE: Long\nF-500\nH-50"
--   from "F-500 / H-50", FocusDamage 500, HealthDamage 50
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21331, 4749, 'FocusDamage', '500');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21332, 4749, 'HealthDamage', '50');
-- RECONSTRUCTION 4772 Focus Damage (ability 3178), effect_desc "Focus Damage: -200"
--   from "Focus Damage: -200", FocusDamage 200
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21333, 4772, 'FocusDamage', '200');
-- RECONSTRUCTION 4784 Damage: F-200 H-20 (ability 1563), effect_desc "Damage: F-200 H-20\nEnergy Return: 5%"
--   from "Damage: F-200 H-20", FocusDamage 200, HealthDamage 20
--   note: "Energy Return: 5%" is not modelled
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21334, 4784, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21335, 4784, 'HealthDamage', '20');
-- RECONSTRUCTION 4974 Disintegration Damage (Third Wave) (ability 2069), effect_desc "F-300 H-30"
--   from "F-300 H-30", FocusDamage 300, HealthDamage 30
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21336, 4974, 'FocusDamage', '300');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21337, 4974, 'HealthDamage', '30');
-- RECONSTRUCTION 4975 Direct Damage (ability 1000), effect_desc "Single Target\nTarget F-100 / H-10"
--   from "Target F-100 / H-10", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21338, 4975, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21339, 4975, 'HealthDamage', '10');
-- RECONSTRUCTION 4976 Direct Damage (ability 1000), effect_desc "Narrow Cone\nSecondary F-100 / H-10"
--   from "Secondary F-100 / H-10", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21340, 4976, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21341, 4976, 'HealthDamage', '10');
-- RECONSTRUCTION 4978 Direct Damage (ability 1217), effect_desc "Medium Cone\nSecondary F-100 / H-10"
--   from "Secondary F-100 / H-10", FocusDamage 100, HealthDamage 10
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21342, 4978, 'FocusDamage', '100');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21343, 4978, 'HealthDamage', '10');
-- RECONSTRUCTION 4996 Direct Damage (ability 1487), effect_desc "Short Radius AE\n-300F / -30H"
--   from "-300F / -30H", FocusDamage 300, HealthDamage 30
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21344, 4996, 'FocusDamage', '300');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21345, 4996, 'HealthDamage', '30');
-- RECONSTRUCTION 5141 Cone 1 (ability 1540), effect_desc "F-200 H-20\n5 Energy per target"
--   from "F-200 H-20", FocusDamage 200, HealthDamage 20
--   note: "5 Energy per target" is a cost; no ability cost is modelled (B-04)
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21346, 5141, 'FocusDamage', '200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21347, 5141, 'HealthDamage', '20');
-- RECONSTRUCTION 5147 Cone 3 (ability 3167), effect_desc "F-150\nH-15"
--   from "F-150 / H-15", FocusDamage 150, HealthDamage 15
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21348, 5147, 'FocusDamage', '150');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (21349, 5147, 'HealthDamage', '15');
-- ability-mechanics generated damage end

-- ability-mechanics generated stat begin
-- GENERATED by tools/ability_mechanics/effect_nvps_from_desc.py (family 'stat', nvp_id 23000-23999).
-- Do not edit by hand: change the parser and regenerate. Rows outside these
-- markers are hand-authored and the tool never touches them.
-- RECONSTRUCTION: the 2009 rows shipped no NVP for these effects. Each value
-- is read from the effect's own effect_desc, quoted below (\n = line break);
-- it is not recovered server data.
-- The same run sets each effect's script_name in effects.sql.
-- RECONSTRUCTION 700 Aim Accuracy Buff (ability 637), effect_desc "Single Target\n+200 Accuracy: 15 Seconds"
--   from "+200 Accuracy: 15 Seconds" -> TimedStat, Accuracy 200
--   note: D-AB09: a bare +200 is +200 stat points (+2 QR per alias.xml)
--   note: 15 s, the effect's pulse_duration
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (23000, 700, 'Accuracy', '200');
-- RECONSTRUCTION 903 Defense Debuff (ability 847), effect_desc "-100 Defense: 15 Seconds"
--   from "-100 Defense: 15 Seconds" -> TimedStat, Defense -100
--   note: D-AB09: a bare -100 is -100 stat points (-1 QR per alias.xml)
--   note: 15 s, the effect's pulse_duration
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (23001, 903, 'Defense', '-100');
-- RECONSTRUCTION 905 Sight In ACC Buff (ability 849), effect_desc "+200 Cover ACC for 15 Seconds"
--   from "+200 Cover ACC for 15 Seconds" -> TimedStat, CoverAccuracy 200
--   note: D-AB09: a bare +200 is +200 stat points
--   note: 15 s, the effect's pulse_duration
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (23002, 905, 'CoverAccuracy', '200');
-- RECONSTRUCTION 907 Brace Accuracy Buff (ability 851), effect_desc "+200 Accuracy for 15 Seconds."
--   from "+200 Accuracy for 15 Seconds." -> TimedStat, Accuracy 200
--   note: D-AB09: a bare +200 is +200 stat points (+2 QR per alias.xml)
--   note: 15 s, the effect's pulse_duration
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (23003, 907, 'Accuracy', '200');
-- RECONSTRUCTION 928 Impeccable Aim Buff (ability 862), effect_desc "+1000 Accuracy for 5 seconds"
--   from "+1000 Accuracy for 5 seconds" -> TimedStat, Accuracy 1000
--   note: D-AB09: a bare +1000 is +1000 stat points (+10 QR per alias.xml)
--   note: 5 s, the effect's pulse_duration
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (23004, 928, 'Accuracy', '1000');
-- RECONSTRUCTION 937 Flashbang Blind Debuff (ability 868), effect_desc "Small Radius AE\nDebuff -200 ACC / DEF: 15 Seconds"
--   from "Debuff -200 ACC / DEF: 15 Seconds" -> TimedStat, Accuracy -200, Defense -200
--   note: D-AB09: a bare -200 is -200 stat points (-2 QR per alias.xml)
--   note: D-AB09: a bare -200 is -200 stat points (-2 QR per alias.xml)
--   note: 15 s, the effect's pulse_duration
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (23005, 937, 'Accuracy', '-200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (23006, 937, 'Defense', '-200');
-- RECONSTRUCTION 1460 Aimed Shot: Leg Snare (ability 855), effect_desc "Single Target\nTarget Movement Speed-30%"
--   from "Target Movement Speed-30%" -> TimedStat, MovementSpeedMod -30
--   note: D-AB09: -30% run speed is movementSpeedMod -30 (100 = unmodified)
--   note: 15 s, the effect's pulse_duration
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (23007, 1460, 'MovementSpeedMod', '-30');
-- RECONSTRUCTION 1743 Buff (ability 1452), effect_desc "Single Target\n+200 CoverDefense: 15 seconds"
--   from "+200 CoverDefense: 15 seconds" -> TimedStat, CoverDefense 200
--   note: D-AB09: a bare +200 is +200 stat points
--   note: 15 s, the effect's pulse_duration
--   note: the ability tooltip names ['CrouchingDefense']; the effect row is what executes (as for heal 3211)
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (23008, 1743, 'CoverDefense', '200');
-- RECONSTRUCTION 1744 Buff (ability 1453), effect_desc "Single Target\n+50 Response: 15 seconds"
--   from "+50 Response: 15 seconds" -> TimedStat, Response 50
--   note: D-AB09: a bare +50 is +50 stat points
--   note: 15 s, the effect's pulse_duration
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (23009, 1744, 'Response', '50');
-- RECONSTRUCTION 1747 Buff (ability 1454), effect_desc "Single Target\n+100 Cover Defense: 15 seconds"
--   from "+100 Cover Defense: 15 seconds" -> TimedStat, CoverDefense 100
--   note: D-AB09: a bare +100 is +100 stat points
--   note: 15 s, the effect's pulse_duration
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (23010, 1747, 'CoverDefense', '100');
-- RECONSTRUCTION 1962 Combat Sprint Run Speed Buff (ability 1619), effect_desc "Single Target\nUser +50% Run Speed\n10 Second Duration"
--   from "User +50% Run Speed" -> TimedStat, MovementSpeedMod 50
--   note: D-AB09: +50% run speed is movementSpeedMod +50 (100 = unmodified)
--   note: 10 s, the effect's pulse_duration
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (23011, 1962, 'MovementSpeedMod', '50');
-- RECONSTRUCTION 1969 Flashbang Blind Debuff (ability 1622), effect_desc "Small Radius AE\nDebuff -200 ACC / DEF: 15 Seconds"
--   from "Debuff -200 ACC / DEF: 15 Seconds" -> TimedStat, Accuracy -200, Defense -200
--   note: D-AB09: a bare -200 is -200 stat points (-2 QR per alias.xml)
--   note: D-AB09: a bare -200 is -200 stat points (-2 QR per alias.xml)
--   note: 15 s, the effect's pulse_duration
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (23012, 1969, 'Accuracy', '-200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (23013, 1969, 'Defense', '-200');
-- RECONSTRUCTION 1980 Accuracy Debuff (ability 1630), effect_desc "Single Target\n-200 Accuracy: 15 Seconds\n-200 Defense: 15 Seconds"
--   from "-200 Accuracy: 15 Seconds / -200 Defense: 15 Seconds" -> TimedStat, Accuracy -200, Defense -200
--   note: D-AB09: a bare -200 is -200 stat points (-2 QR per alias.xml)
--   note: D-AB09: a bare -200 is -200 stat points (-2 QR per alias.xml)
--   note: 15 s, the effect's pulse_duration
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (23014, 1980, 'Accuracy', '-200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (23015, 1980, 'Defense', '-200');
-- RECONSTRUCTION 2752 Accuracy -100 (ability 2070), effect_desc "Accuracy -100"
--   from "Accuracy -100" -> TimedStat, Accuracy -100
--   note: D-AB09: a bare -100 is -100 stat points (-1 QR per alias.xml)
--   note: 25 s, the effect's pulse_duration
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (23016, 2752, 'Accuracy', '-100');
-- RECONSTRUCTION 4239 Blind Debuff (ability 863), effect_desc "Single Target\nTarget -200 ACC / -200 DEF\nDuration: 15sec"
--   from "Target -200 ACC / -200 DEF" -> TimedStat, Accuracy -200, Defense -200
--   note: D-AB09: a bare -200 is -200 stat points (-2 QR per alias.xml)
--   note: D-AB09: a bare -200 is -200 stat points (-2 QR per alias.xml)
--   note: 15 s, the effect's pulse_duration
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (23017, 4239, 'Accuracy', '-200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (23018, 4239, 'Defense', '-200');
-- RECONSTRUCTION 4309 Stat Debuff (ability 1242), effect_desc "Single Target\nTarget -100 Response"
--   from "Target -100 Response" -> TimedStat, Response -100
--   note: D-AB09: a bare -100 is -100 stat points
--   note: 15 s, the effect's pulse_duration
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (23019, 4309, 'Response', '-100');
-- RECONSTRUCTION 4333 Blind (ability 1354), effect_desc "Single Target\nTarget -200ACC / -200DEF"
--   from "Target -200ACC / -200DEF" -> TimedStat, Accuracy -200, Defense -200
--   note: D-AB09: a bare -200 is -200 stat points (-2 QR per alias.xml)
--   note: D-AB09: a bare -200 is -200 stat points (-2 QR per alias.xml)
--   note: 15 s, the effect's pulse_duration
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (23020, 4333, 'Accuracy', '-200');
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (23021, 4333, 'Defense', '-200');
-- RECONSTRUCTION 4335 Slow (ability 1354), effect_desc "Single Target\nTarget -100 Response"
--   from "Target -100 Response" -> TimedStat, Response -100
--   note: D-AB09: a bare -100 is -100 stat points
--   note: 15 s, the effect's pulse_duration
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (23022, 4335, 'Response', '-100');
-- RECONSTRUCTION 4411 Snare Debuff (ability 1474), effect_desc "Single Target\nTarget -30% Movement Speed\nDuration: 15 Seconds"
--   from "Target -30% Movement Speed" -> TimedStat, MovementSpeedMod -30
--   note: D-AB09: -30% run speed is movementSpeedMod -30 (100 = unmodified)
--   note: 15 s, the effect's pulse_duration
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (23023, 4411, 'MovementSpeedMod', '-30');
-- RECONSTRUCTION 4715 Debuff: Cooldown Timers (ability 3168), effect_desc "Response -100"
--   from "Response -100" -> TimedStat, Response -100
--   note: D-AB09: a bare -100 is -100 stat points
--   note: 30 s, the effect's pulse_duration
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (23024, 4715, 'Response', '-100');
-- RECONSTRUCTION 5266 Debuff (ability 1728), effect_desc "Debuff -100 Defense\n35 seconds"
--   from "Debuff -100 Defense" -> TimedStat, Defense -100
--   note: D-AB09: a bare -100 is -100 stat points (-1 QR per alias.xml)
--   note: 35 s, the effect's pulse_duration
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (23025, 5266, 'Defense', '-100');
-- ability-mechanics generated stat end

--
-- TOC entry 3313 (class 0 OID 0)
-- Dependencies: 305
-- Name: effect_nvps_2_nvp_id_seq; Type: SEQUENCE SET; Schema: resources; Owner: -
--

SELECT pg_catalog.setval('effect_nvps_2_nvp_id_seq', 462, true);

