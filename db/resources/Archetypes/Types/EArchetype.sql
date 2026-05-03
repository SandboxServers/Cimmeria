--
-- TOC entry 800 (class 1247 OID 60100)
-- Name: EArchetype; Type: TYPE; Schema: resources; Owner: -
--
-- Adding a new archetype requires updates in (verify all):
--   1. This file                                                                   - enum entry
--   2. db/sgw/Players/Tables/sgw_player.sql                                         - CHECK (archetype <= N) bound
--   3. crates/entity/src/stats/archetype.rs::ARCHETYPE_COUNT                        - Rust-side cardinality anchor
--   4. db/resources/Archetypes/Seed/archetype_ability_tree.sql                      - ability tree rows (or accept empty)
--   5. crates/services/src/base/chardef.rs                                          - CharDefId entries (or no character can use the new archetype)
--   6. crates/services/src/mercury/world_data/stats.rs::archetype_ability_tree      - DB-down fallback (only if non-empty fallback is desired)
--
-- The live-DB test `archetype_count_matches_earchetype_enum_cardinality`
-- in `crates/services/src/base/world_entry/methods/player_load/meta.rs`
-- pins this list against ARCHETYPE_COUNT and fails CI loud if you bump
-- the enum without updating the const.
--

CREATE TYPE "EArchetype" AS ENUM (
    'ARCHETYPE_Any',
    'ARCHETYPE_Soldier',
    'ARCHETYPE_Commando',
    'ARCHETYPE_Scientist',
    'ARCHETYPE_Archeologist',
    'ARCHETYPE_Asgard',
    'ARCHETYPE_Goauld',
    'ARCHETYPE_Sholva',
    'ARCHETYPE_Jaffa'
);

