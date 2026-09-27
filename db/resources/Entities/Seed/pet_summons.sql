--
-- Data for Name: pet_summons; Type: TABLE DATA; Schema: resources; Owner: -
--
-- Pets campaign rows (docs/analysis/pets/). Pet templates are 350-369.
-- 2826 Summon Straegis (Goa'uld Servant Lord capstone, L50) -> 350
-- "Summoned Straegis Fighter", one active pet.
-- 3491/3493/3495/3496/3497 are also named "Summon Straegis" but are warmup-0
-- copies that no archetype tree or trainer list grants; they get no row.
-- Jaffa (1643), Prime (1645) and Lo'taur (1644) are added by PT-11.

INSERT INTO pet_summons (ability_id, template_id, max_active) VALUES (2826, 350, 1);
