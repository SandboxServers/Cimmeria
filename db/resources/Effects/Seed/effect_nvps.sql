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

--
-- TOC entry 3313 (class 0 OID 0)
-- Dependencies: 305
-- Name: effect_nvps_2_nvp_id_seq; Type: SEQUENCE SET; Schema: resources; Owner: -
--

SELECT pg_catalog.setval('effect_nvps_2_nvp_id_seq', 201, true);

