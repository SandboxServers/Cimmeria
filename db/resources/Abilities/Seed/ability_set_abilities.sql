--
-- TOC entry 3166 (class 0 OID 62791)
-- Dependencies: 183
-- Data for Name: ability_set_abilities; Type: TABLE DATA; Schema: resources; Owner: -
--

INSERT INTO ability_set_abilities (ability_set_id, ability_id) VALUES (2, 221);

INSERT INTO ability_set_abilities (ability_set_id, ability_id) VALUES (1, 579);

INSERT INTO ability_set_abilities (ability_set_id, ability_id) VALUES (3, 559);

-- Harset rebuild packets H11 (sets 4 and 5) and H09 (their second rows).
--
-- H11 shipped one row per set because the primary key was `(ability_set_id)`
-- alone and a second row aborted the whole seed load. H09 widened that key to
-- `(ability_set_id, ability_id)`, so a set holds N abilities and the loaders'
-- `array_agg(asa.ability_id ORDER BY asa.ability_id)` finally returns more than
-- one. See the header comment in ability_sets.sql for how the members are
-- chosen, and docs/analysis/harset-rebuild/worknotes/H09.md for the evidence.
--
-- Members come in ranged/melee pairs because that is how the 2009 data binds a
-- weapon: every row in `items_event_sets` pairs `EVENT_ITEM_RANGED = 7` with
-- `EVENT_ITEM_MELEE = 6` for the same item. 204 staff items bind 584 + 710; 151
-- ribbon-device items bind 712 + 711.
--
-- Selection order is ascending ability id (`choose_npc_ability` sorts, then
-- takes the first off-cooldown id), so the LOWER id is the NPC's primary attack
-- and the other is a cooldown fallback. That makes set 4 ranged-first (584 <
-- 710) and set 5 melee-first (711 < 712). The inversion on set 5 is cosmetically
-- inert -- 711 and 712 share `event_set_id = 300`, so they emit the same
-- `onSequence` -- but 711 is `is_ranged = false`, so most ribbon attacks roll
-- against melee rather than ranged defence in `calculate_qr`. Recorded as a
-- known consequence of the lowest-id rule, not a bug to paper over here.

-- 584 'Staff Auto Attack': ranged, `max_range = 0` so the AI falls back to
-- NPC_ATTACK_RANGE (30), `required_ammo = 0`, `event_set_id = 3` (same as 579
-- Pistol Auto Attack, which set 1 uses). Bound to `EVENT_ITEM_RANGED` on all
-- 204 staff items. Lowest id in the set, so this is the Jaffa's primary.
INSERT INTO ability_set_abilities (ability_set_id, ability_id) VALUES (4, 584);

-- 710 'Staff Melee AA': the `EVENT_ITEM_MELEE` half of the same 204 bindings.
-- `event_set_id = 300`, `required_ammo = 0`, cooldown 2 against 584's 3, so it
-- fires in 584's cooldown window. `max_range = 0` resolves to NPC_ATTACK_RANGE
-- like everything else in the seed, so the swing can play at up to 30m -- that
-- matches the python selector, which never range-gated the pick at all
-- (`deprecated/python/cell/SGWMob.py` classifyHostileAbility: `# TODO: Check
-- distance, LOS`). Not a new artefact; see H09.md.
INSERT INTO ability_set_abilities (ability_set_id, ability_id) VALUES (4, 710);

-- 712 'Ribbon Device Auto Attack': ranged half, `event_set_id = 300`. The
-- hara'kesh an Ashrak carries is a hand-device variant, so this covers both the
-- Ashrak assassin and the hub Goa'uld.
INSERT INTO ability_set_abilities (ability_set_id, ability_id) VALUES (5, 712);

-- 711 'Ribbon Device Melee AA': the melee half of the same 151 bindings, same
-- `event_set_id = 300`. Sorts below 712, so it is the set's primary pick.
INSERT INTO ability_set_abilities (ability_set_id, ability_id) VALUES (5, 711);

-- NEW CONTENT (debug hub): 710 'Staff Melee AA' alone. Its effect 736 carries
-- no damage values, so a hit resolves to 0. Melee, so the stationary crate
-- holds fire until the player stands in melee reach.
INSERT INTO ability_set_abilities (ability_set_id, ability_id) VALUES (6, 710);


-- Pets campaign, set 350 (the Straegis pet, template 350). The Straegis mob kit
-- in the seed is 1156 Disengage, 2847 Dissonance and 1240 Straegis Explode,
-- and none of the three deals damage today: their effects (1322, 4138,
-- 1397/1398) carry no damage NVP and no script. So the pet needs a real
-- repeatable attack, and two of the three must stay out:
-- * 1240 Straegis Explode has cooldown 0 and event set 1507 "Straegis death
--   ability source": the lowest-id fallback would play the death burst on a
--   living pet every AI tick.
-- * 2847 Dissonance is is_ranged = false (3 m reach) and self-targeted
--   (target type 1); the AI would cast it at its enemy and walk into melee.
-- 221 Energy Shock (the prisoner retrieval unit's attack, set 2): ranged,
-- cooldown 2, effect 264 deals HealthDamage 16, event set 802 plays the
-- EnergyShock beam (sequence 1866), which needs no humanoid animation. It is
-- the lowest id, so it is the primary. 1156 Disengage (event set 1499,
-- sequence 2824, the Straegis-native sEmitAura) is cosmetic filler while 221
-- cools. Its is_ranged = true with max_range 0 resolves to the 30 m default.
INSERT INTO ability_set_abilities (ability_set_id, ability_id) VALUES (350, 221);

INSERT INTO ability_set_abilities (ability_set_id, ability_id) VALUES (350, 1156);
