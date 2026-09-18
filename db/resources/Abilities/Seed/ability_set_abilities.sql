--
-- TOC entry 3166 (class 0 OID 62791)
-- Dependencies: 183
-- Data for Name: ability_set_abilities; Type: TABLE DATA; Schema: resources; Owner: -
--

INSERT INTO ability_set_abilities (ability_set_id, ability_id) VALUES (2, 221);

INSERT INTO ability_set_abilities (ability_set_id, ability_id) VALUES (1, 579);

INSERT INTO ability_set_abilities (ability_set_id, ability_id) VALUES (3, 559);

-- Harset rebuild packet H11. Exactly one row per set -- the table's primary key
-- is `(ability_set_id)` alone, so a second row for the same set is a duplicate
-- key. See the header comment in ability_sets.sql.

-- 584 'Staff Auto Attack': ranged, `max_range = 0` so the AI falls back to
-- NPC_ATTACK_RANGE (30), `required_ammo = 0`, `event_set_id = 3` (same as 579
-- Pistol Auto Attack, which set 1 uses). The only ranged staff ability in the
-- seed with a non-NULL `event_set_id`.
INSERT INTO ability_set_abilities (ability_set_id, ability_id) VALUES (4, 584);

-- 712 'Ribbon Device Auto Attack': same shape, `event_set_id = 300`. The
-- hara'kesh an Ashrak carries is a hand-device variant, so this covers both the
-- Ashrak assassin and the hub Goa'uld.
INSERT INTO ability_set_abilities (ability_set_id, ability_id) VALUES (5, 712);

