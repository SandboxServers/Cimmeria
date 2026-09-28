--
-- Data for Name: deployables; Type: TABLE DATA; Schema: resources; Owner: -
--
-- Deployables Phase 0 (docs/analysis/deployables/). Deployable templates are
-- 400-409.
-- 1012 Deployable: Microwave Emitter (Scientist, Support branch, L25) ->
-- 400 "Deployable: Microwave Emitter". 5065 "Pulser" times it: 30 pulses x
-- 1 s, so it stands for 30 s. 5066 "Damage" is what each pulse applies, in
-- its "Medium" radius (10 m, the client's AE radius tier). One out at a time: a re-cast replaces it (the
-- ability's own 30 s cooldown equals the lifetime, so this only bites on a
-- GM cooldown reset).
-- 1236 Aggression Inducer (3176 Pulser + 3175 Threat Generator) has the
-- same shape but needs a threat script first; it is Phase 1.

INSERT INTO deployables (ability_id, template_id, lifetime_effect_id, pulse_effect_id, max_active) VALUES (1012, 400, 5065, 5066, 1);
