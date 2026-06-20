-- Migration: add state_field column to sgw_player for persisted
-- user-preference state bits. (#412)
--
-- `BSF_AutoCycling` (bit 1 of the cell entity's `state_field`) is a player
-- preference toggle — the original game persisted it across logins, but the
-- emulator initialised `state_field` to 0 on every world entry, so the
-- auto-cycle loop never armed after a relog until the player pressed the
-- button again.
--
-- Only bits in PERSISTED_STATE_FIELD_MASK
-- (crates/services/src/cell/combat/state.rs) are ever written here — today
-- that's BSF_AutoCycling (1 << 1) alone. Transient combat bits (BSF_Dead,
-- BSF_InCombat, BSF_MovementLock) are masked out at the persist site so a
-- relog is always a clean combat slate. Safe to run on existing databases
-- (uses IF NOT EXISTS).

ALTER TABLE sgw_player
    ADD COLUMN IF NOT EXISTS state_field integer NOT NULL DEFAULT 0;

-- Guard against malformed payloads landing as durable corruption: the
-- persisted value is a masked bitfield, always non-negative. PostgreSQL has
-- no IF NOT EXISTS for CHECK constraints, so use a DO block for idempotency.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'sgw_player_state_field_nonnegative_chk'
          AND conrelid = 'sgw_player'::regclass
    ) THEN
        ALTER TABLE sgw_player
            ADD CONSTRAINT sgw_player_state_field_nonnegative_chk
            CHECK (state_field >= 0);
    END IF;
END $$;
