--
-- Deployables: which entity template a deployable ability places, and which
-- of its effects time it and hit with it (deployables Phase 0;
-- docs/gameplay/deployables.md, docs/analysis/deployables/). One row per
-- deployable ability.
--
-- No original data encodes this binding. 1012 "Deployable: Microwave
-- Emitter" carries a "Pulser" effect (5065: "Single Target / 30 pulses x1
-- Second duration / Despawn Target on Finish") and a "Damage" effect (5066:
-- "Medium Radius AE / Secondary -100F"), in the seed and in the client's
-- CookedDataEffects.pak alike, but neither names a template, and nothing
-- says the damage rides on the pulser. So the link is Cimmeria seed data,
-- read into the cell's startup caches by `load_deployables`
-- (crates/cell-catalog/src/cell/spawner/deployables.rs) and consulted when
-- the ability is cast.
--
-- `lifetime_effect_id`'s `pulse_count` x `pulse_duration` is the object's
-- lifetime and pulse cadence. `pulse_effect_id` is the effect each pulse
-- applies to every target the owner may hit within its radius (its
-- `Radius` NVP, else its `tcm_param1` range tier). `template_id` must name a
-- deployable template: 400-409, `class = 'being'`, no loot table, never in
-- `spawnlist`. The live-DB guards in
-- crates/cell-catalog/src/cell/spawner/tests/live_db_deployables.rs pin it.
-- `max_active` is how many objects from this ability one owner may have
-- out at once; a re-cast past it removes the oldest.
--
-- Name: deployables; Type: TABLE; Schema: resources; Owner: -
--

CREATE TABLE deployables (
    ability_id integer NOT NULL,
    template_id integer NOT NULL,
    lifetime_effect_id integer NOT NULL,
    pulse_effect_id integer NOT NULL,
    max_active integer DEFAULT 1 NOT NULL,
    CONSTRAINT deployables_max_active_positive CHECK (max_active >= 1)
);
