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

--
-- TOC entry 3313 (class 0 OID 0)
-- Dependencies: 305
-- Name: effect_nvps_2_nvp_id_seq; Type: SEQUENCE SET; Schema: resources; Owner: -
--

SELECT pg_catalog.setval('effect_nvps_2_nvp_id_seq', 462, true);

