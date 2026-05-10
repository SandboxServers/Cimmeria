-- Migration: add required_mission_id column to ring_transport_regions.
--
-- Mission-tied rings (e.g. the Cellblock armory ring used in mission 688)
-- need to refuse interaction/destination selection unless the player has
-- completed the gating mission. Chain-level gates can't enforce this
-- because `handle_interact` runs in the runtime path, not via chains.
--
-- NULL means "no gating" — the historical default for all 32 rings shipped
-- prior to mission 688. Safe to run on existing databases (uses IF NOT
-- EXISTS).

ALTER TABLE resources.ring_transport_regions
    ADD COLUMN IF NOT EXISTS required_mission_id integer;
