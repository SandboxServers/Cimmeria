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

-- Wires effect 1383 (Medical Attention: Recuperation — ability 1218,
-- 75% Health heal over 25 seconds) to the existing `HealHealth` script.
-- The effect already has `pulse_count = 25` and `pulse_duration = 1`
-- on its row in effects.sql.
--
-- Pulse-count accounting (verified against
-- `crates/services/src/cell/effects/pulsing.rs:118-119`):
--   - `damage_apply` fires the initial pulse synchronously
--     (counts as pulse 1 of 25 — NOT an extra pulse)
--   - `register_active_effect` schedules `remaining = pulse_count - 1`
--     follow-up pulses (24 in this case) at `pulse_duration` intervals
--   - Total: 1 initial + 24 follow-ups = 25 pulses × 3% = 75% of max HP
--     over 25 seconds. Matches the effect's `effect_desc` exactly.
--
-- nvp_id 200 set well clear of PR #493 (18/19) and PR #496 (100) so
-- this PR doesn't collide regardless of merge order. The 18-199 gap
-- is harmless; nvp_id is a primary key, not a packed array index, and
-- the seed loader doesn't iterate by id.
INSERT INTO effect_nvps (nvp_id, effect_id, name, value) VALUES (200, 1383, 'HealPercentage', '3.00');

--
-- TOC entry 3313 (class 0 OID 0)
-- Dependencies: 305
-- Name: effect_nvps_2_nvp_id_seq; Type: SEQUENCE SET; Schema: resources; Owner: -
--

SELECT pg_catalog.setval('effect_nvps_2_nvp_id_seq', 201, true);

