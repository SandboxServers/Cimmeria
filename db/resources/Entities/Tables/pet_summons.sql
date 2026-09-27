--
-- Pet summons: which entity template a pet-summon ability spawns (pets
-- campaign, PT-S; docs/analysis/pets/). One row per summon ability.
--
-- No original data encodes this binding. The 2009 summon abilities
-- (1643 Summon Jaffa, 1644 Lo'taur, 1645 Prime, 2826 Straegis, ...) carry
-- no effects, and the editor-only "Spawn Mob" effects have no script, NVP
-- or template id, in the seed and in the client's CookedDataAbilities.pak
-- alike. So the ability -> template link is Cimmeria seed data, read into
-- the cell's startup caches by `load_pet_summons`
-- (crates/cell-catalog/src/cell/spawner/pet_summons.rs) and consulted when
-- the ability fires.
--
-- `template_id` must name a pet template: 350-369, `class = 'pet'`, the
-- ENTITYFLAG_Pet (1024) bit set, no loot table, never in `spawnlist`. The
-- live-DB guards in crates/cell-catalog/src/cell/spawner/tests/
-- live_db_pet_summons.rs pin all of it. `max_active` is how many pets from
-- this ability one owner may have out at once (D-PT04: 1; a re-summon
-- replaces the current pet).
--
-- Name: pet_summons; Type: TABLE; Schema: resources; Owner: -
--

CREATE TABLE pet_summons (
    ability_id integer NOT NULL,
    template_id integer NOT NULL,
    max_active integer DEFAULT 1 NOT NULL,
    CONSTRAINT pet_summons_max_active_positive CHECK (max_active >= 1)
);
