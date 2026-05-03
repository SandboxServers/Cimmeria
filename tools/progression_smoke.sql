\set ON_ERROR_STOP on

BEGIN;

DO $$
DECLARE
    v_account_id integer := -900003;
    v_player_a_id integer := -900003;
    v_player_b_id integer := -900004;
    v_account_name text := '__codex_progression_smoke__';
    v_player_a_name text := '__CodexProgressionSmokeA__';
    v_player_b_name text := '__CodexProgressionSmokeB__';
    v_world_id integer;
    v_world text;

    v_starting_naquadah constant integer := 100;
    v_grant_amount constant integer := 500;
    v_a_naquadah integer;
    v_b_naquadah integer;
    v_a_exp integer;
    v_b_exp integer;
    v_a_level integer;
    v_b_level integer;
    v_a_tp integer;
    v_b_tp integer;
BEGIN
    IF EXISTS (SELECT 1 FROM account WHERE account_id = v_account_id) THEN
        RAISE EXCEPTION 'temporary account_id % is already in use', v_account_id;
    END IF;

    IF EXISTS (
        SELECT 1
          FROM sgw_player
         WHERE player_id IN (v_player_a_id, v_player_b_id)
            OR player_name IN (v_player_a_name, v_player_b_name)
    ) THEN
        RAISE EXCEPTION 'temporary player_ids/names are already in use';
    END IF;

    SELECT world_id, world
      INTO v_world_id, v_world
      FROM resources.worlds
     ORDER BY world_id
     LIMIT 1;

    IF v_world IS NULL THEN
        RAISE EXCEPTION 'resources.worlds has no rows';
    END IF;

    -- Single account with two characters — the multi-character
    -- isolation tests need both characters to exist under the same
    -- account_id so a buggy WHERE that targets account_id rather
    -- than player_id would land on both.
    INSERT INTO account (account_id, account_name, password, accesslevel, enabled)
    VALUES (v_account_id, v_account_name, 'smoke', 0, true);

    INSERT INTO sgw_player (
        account_id, player_id, level, alignment, archetype, gender,
        player_name, extra_name, world_location, bodyset, title,
        pos_x, pos_y, pos_z, heading, naquadah, exp, training_points,
        first_login, world_id, skin_color_id
    )
    VALUES
        (
            v_account_id, v_player_a_id, 1, 0, 0, 1,
            v_player_a_name, '', v_world, 'BS_HumanMale.BS_HumanMale', 0,
            0, 0, 0, 0, v_starting_naquadah, 0, 0,
            1, v_world_id, 0
        ),
        (
            v_account_id, v_player_b_id, 1, 0, 0, 1,
            v_player_b_name, '', v_world, 'BS_HumanMale.BS_HumanMale', 0,
            0, 0, 0, 0, v_starting_naquadah, 0, 0,
            1, v_world_id, 0
        );

    -- ── Stage 1: GrantCash to player A only ─────────────────────────
    -- Mirrors handle_grant_cash (mod.rs:305-307):
    --   UPDATE sgw_player SET naquadah = naquadah + $1 WHERE player_id = $2
    -- This must stay scoped to player_id. If a refactor changes the
    -- WHERE to account_id, both characters on this account would be
    -- credited. The script asserts player B's naquadah stays at the
    -- starting value to catch that.
    UPDATE sgw_player
       SET naquadah = naquadah + v_grant_amount
     WHERE player_id = v_player_a_id;

    SELECT naquadah INTO v_a_naquadah
      FROM sgw_player WHERE player_id = v_player_a_id;
    SELECT naquadah INTO v_b_naquadah
      FROM sgw_player WHERE player_id = v_player_b_id;

    IF v_a_naquadah <> v_starting_naquadah + v_grant_amount THEN
        RAISE EXCEPTION 'after GrantCash: player A naquadah expected %, got %',
            v_starting_naquadah + v_grant_amount, v_a_naquadah;
    END IF;

    IF v_b_naquadah <> v_starting_naquadah THEN
        RAISE EXCEPTION 'after GrantCash: player B naquadah leaked! expected % (unchanged), got %',
            v_starting_naquadah, v_b_naquadah;
    END IF;

    -- ── Stage 2: GrantXP to player A only ───────────────────────────
    -- Mirrors the handle_grant_xp contract:
    --   UPDATE sgw_player SET exp = $1, level = $2, training_points = $3
    --     WHERE player_id = $4
    -- Same multi-character isolation concern: player B's xp/level/tp
    -- must be untouched after the UPDATE.
    UPDATE sgw_player
       SET exp = 1500, level = 2, training_points = 1
     WHERE player_id = v_player_a_id;

    SELECT exp, level, training_points
      INTO v_a_exp, v_a_level, v_a_tp
      FROM sgw_player WHERE player_id = v_player_a_id;
    SELECT exp, level, training_points
      INTO v_b_exp, v_b_level, v_b_tp
      FROM sgw_player WHERE player_id = v_player_b_id;

    IF v_a_exp <> 1500 OR v_a_level <> 2 OR v_a_tp <> 1 THEN
        RAISE EXCEPTION 'after GrantXP: player A expected (exp=1500, level=2, tp=1), got (exp=%, level=%, tp=%)',
            v_a_exp, v_a_level, v_a_tp;
    END IF;

    IF v_b_exp <> 0 OR v_b_level <> 1 OR v_b_tp <> 0 THEN
        RAISE EXCEPTION 'after GrantXP: player B leaked! expected (exp=0, level=1, tp=0), got (exp=%, level=%, tp=%)',
            v_b_exp, v_b_level, v_b_tp;
    END IF;

    -- ── Stage 3: GrantCash to player B (reverse direction) ──────────
    -- The first stage proved A->A doesn't leak to B. This stage
    -- proves the WHERE clause works in the other direction too: a
    -- grant targeting B mustn't credit A. Catches a hypothetical
    -- "always credit player_id=lowest-on-account" bug.
    UPDATE sgw_player
       SET naquadah = naquadah + v_grant_amount
     WHERE player_id = v_player_b_id;

    SELECT naquadah INTO v_a_naquadah
      FROM sgw_player WHERE player_id = v_player_a_id;
    SELECT naquadah INTO v_b_naquadah
      FROM sgw_player WHERE player_id = v_player_b_id;

    IF v_a_naquadah <> v_starting_naquadah + v_grant_amount THEN
        RAISE EXCEPTION 'after second GrantCash: player A naquadah leaked! expected %, got %',
            v_starting_naquadah + v_grant_amount, v_a_naquadah;
    END IF;

    IF v_b_naquadah <> v_starting_naquadah + v_grant_amount THEN
        RAISE EXCEPTION 'after second GrantCash: player B naquadah expected %, got %',
            v_starting_naquadah + v_grant_amount, v_b_naquadah;
    END IF;

    RAISE NOTICE 'progression smoke passed: account %, player A %, player B %',
        v_account_id, v_player_a_id, v_player_b_id;
END $$;

ROLLBACK;
