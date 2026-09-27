--
-- Data for Name: pet_summons; Type: TABLE DATA; Schema: resources; Owner: -
--
-- Pets campaign rows (docs/analysis/pets/). Pet templates are 350-369.
-- 2826 Summon Straegis (Goa'uld Servant Lord capstone, L50) -> 350
-- "Summoned Straegis Fighter", one active pet.
-- 3491/3493/3495/3496/3497 are also named "Summon Straegis" but are warmup-0
-- copies that no archetype tree or trainer list grants; they get no row.
-- PT-11 adds the rest of the Servant Lord roster, one active pet each
-- (D-PT04): 1643 Summon Jaffa (L1 root) -> 351 "Jaffa Soldier", 1645 Summon
-- Prime (L15) -> 352 "Jaffa Prime", 1644 Summon Lo'taur (L10) -> 353
-- "Lo'Taur Servant". 1862 MS019_Summon Jaffa is an editor template no tree
-- or trainer grants; it gets no row.

INSERT INTO pet_summons (ability_id, template_id, max_active) VALUES (2826, 350, 1);
INSERT INTO pet_summons (ability_id, template_id, max_active) VALUES (1643, 351, 1);
INSERT INTO pet_summons (ability_id, template_id, max_active) VALUES (1645, 352, 1);
INSERT INTO pet_summons (ability_id, template_id, max_active) VALUES (1644, 353, 1);
