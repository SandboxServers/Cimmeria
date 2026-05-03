\set ON_ERROR_STOP on

BEGIN;

DO $$
DECLARE
    v_account_id integer := -900002;
    v_player_id integer := -900002;
    v_account_name text := '__codex_inventory_move_smoke__';
    v_player_name text := '__CodexInventoryMoveSmoke__';
    v_world_id integer;
    v_world text;
    v_type_id integer;
    v_base_item_id integer;
    v_stack_item_id integer;
    v_single_item_id integer;
    v_split_item_id integer;
    v_count integer;
    v_stack integer;
    v_slot integer;

    -- Sentinel slot used in the three-step swap. Mirrors the
    -- `RESERVED_SENTINEL_SLOT` convention in the move handler — pick
    -- a value high enough that it can't collide with a real bag slot.
    v_sentinel_slot integer := 9999;

    -- Container 1 = INV_MAIN. Both moves and splits stay within this
    -- container; cross-container moves don't add coverage past what
    -- the per-handler tests already pin.
    INV_MAIN constant integer := 1;
BEGIN
    IF EXISTS (SELECT 1 FROM account WHERE account_id = v_account_id) THEN
        RAISE EXCEPTION 'temporary account_id % is already in use', v_account_id;
    END IF;

    IF EXISTS (SELECT 1 FROM sgw_player WHERE player_id = v_player_id OR player_name = v_player_name) THEN
        RAISE EXCEPTION 'temporary player_id/name is already in use';
    END IF;

    SELECT world_id, world
      INTO v_world_id, v_world
      FROM resources.worlds
     ORDER BY world_id
     LIMIT 1;

    IF v_world IS NULL THEN
        RAISE EXCEPTION 'resources.worlds has no rows';
    END IF;

    -- Pick any item type that's allowed in INV_MAIN (container 1).
    -- The split / move / swap tests don't care about the type's
    -- properties — only that the FK to resources.items is satisfied.
    SELECT ri.item_id
      INTO v_type_id
      FROM resources.items ri
     WHERE ri.container_sets IS NULL OR 1 = ANY(ri.container_sets)
     ORDER BY ri.item_id
     LIMIT 1;

    IF v_type_id IS NULL THEN
        RAISE EXCEPTION 'no main-bag-allowed item type in resources.items';
    END IF;

    SELECT GREATEST(10000, COALESCE(MAX(item_id), 9999) + 1000)
      INTO v_base_item_id
      FROM sgw_inventory;

    IF EXISTS (
        SELECT 1
          FROM sgw_inventory
         WHERE item_id >= v_base_item_id
           AND item_id < v_base_item_id + 10
    ) THEN
        RAISE EXCEPTION 'temporary item_id range starting at % is already in use', v_base_item_id;
    END IF;

    v_stack_item_id := v_base_item_id;
    v_single_item_id := v_base_item_id + 1;
    v_split_item_id := v_base_item_id + 2;

    INSERT INTO account (account_id, account_name, password, accesslevel, enabled)
    VALUES (v_account_id, v_account_name, 'smoke', 0, true);

    INSERT INTO sgw_player (
        account_id, player_id, level, alignment, archetype, gender,
        player_name, extra_name, world_location, bodyset, title,
        pos_x, pos_y, pos_z, heading, naquadah, exp, first_login,
        world_id, skin_color_id
    )
    VALUES (
        v_account_id, v_player_id, 1, 0, 0, 1,
        v_player_name, '', v_world, 'BS_HumanMale.BS_HumanMale', 0,
        0, 0, 0, 0, 0, 0, 1,
        v_world_id, 0
    );

    -- Initial inventory:
    --   slot 0: stack of 10 (the row we'll split, then swap)
    --   slot 1: single (the row we'll move, then swap with the stack)
    INSERT INTO sgw_inventory (
        item_id, character_id, type_id, stack_size, slot_id,
        container_id, bound, durability, charges, flags, ammo
    )
    VALUES
        (v_stack_item_id, v_player_id, v_type_id, 10, 0, INV_MAIN, false, 100, 0, 0, 0),
        (v_single_item_id, v_player_id, v_type_id, 1, 1, INV_MAIN, false, 100, 0, 0, 0);

    -- ── Stage 1: split ──────────────────────────────────────────────
    -- Take 3 from the stack at slot 0 (10 -> 7), insert a new row at
    -- slot 5 with stack_size=3. Mirrors the split path in
    -- handle_move_inventory_item.
    UPDATE sgw_inventory
       SET stack_size = stack_size - 3
     WHERE character_id = v_player_id AND item_id = v_stack_item_id;

    INSERT INTO sgw_inventory (
        item_id, character_id, type_id, stack_size, slot_id,
        container_id, bound, durability, charges, flags, ammo
    )
    VALUES
        (v_split_item_id, v_player_id, v_type_id, 3, 5, INV_MAIN, false, 100, 0, 0, 0);

    -- Verify the split arithmetic.
    SELECT stack_size, slot_id
      INTO v_stack, v_slot
      FROM sgw_inventory
     WHERE character_id = v_player_id AND item_id = v_stack_item_id;

    IF v_stack <> 7 OR v_slot <> 0 THEN
        RAISE EXCEPTION 'after split: source row expected (slot=0, stack=7), got (slot=%, stack=%)',
            v_slot, v_stack;
    END IF;

    SELECT stack_size, slot_id
      INTO v_stack, v_slot
      FROM sgw_inventory
     WHERE character_id = v_player_id AND item_id = v_split_item_id;

    IF v_stack <> 3 OR v_slot <> 5 THEN
        RAISE EXCEPTION 'after split: split row expected (slot=5, stack=3), got (slot=%, stack=%)',
            v_slot, v_stack;
    END IF;

    SELECT COUNT(*)
      INTO v_count
      FROM sgw_inventory
     WHERE character_id = v_player_id;

    IF v_count <> 3 THEN
        RAISE EXCEPTION 'after split: expected 3 inventory rows, got %', v_count;
    END IF;

    -- Total stack across the type must conserve: 10 (orig stack) + 1
    -- (single) = 7 + 3 + 1 = 11. The split creates a new row but
    -- doesn't change the conserved total.
    SELECT COALESCE(SUM(stack_size), 0)
      INTO v_count
      FROM sgw_inventory
     WHERE character_id = v_player_id AND type_id = v_type_id;

    IF v_count <> 11 THEN
        RAISE EXCEPTION 'after split: total stack must conserve at 11, got %', v_count;
    END IF;

    -- ── Stage 2: simple move ────────────────────────────────────────
    -- Move the single from slot 1 to slot 4. The unique
    -- (character, container, slot) index in sgw_inventory is what
    -- enforces "no two items at the same slot" — this UPDATE would
    -- fail if slot 4 were already occupied.
    UPDATE sgw_inventory
       SET slot_id = 4
     WHERE character_id = v_player_id AND item_id = v_single_item_id;

    SELECT slot_id INTO v_slot
      FROM sgw_inventory
     WHERE character_id = v_player_id AND item_id = v_single_item_id;

    IF v_slot <> 4 THEN
        RAISE EXCEPTION 'after simple move: single expected at slot 4, got slot %', v_slot;
    END IF;

    -- Slot 1 must be empty.
    IF EXISTS (
        SELECT 1
          FROM sgw_inventory
         WHERE character_id = v_player_id AND container_id = INV_MAIN AND slot_id = 1
    ) THEN
        RAISE EXCEPTION 'after simple move: slot 1 must be empty';
    END IF;

    -- ── Stage 3: three-step swap ────────────────────────────────────
    -- Swap the items at slot 0 (the post-split stack, size 7) and
    -- slot 4 (the single). Mirrors the move handler's three-step
    -- procedure: park source at a sentinel slot, move occupied to
    -- source's old slot, move source to occupied's old slot. The
    -- intermediate sentinel slot exists because the unique index
    -- forbids two rows briefly sharing a slot mid-swap.

    -- Step A: park the source at the sentinel slot.
    UPDATE sgw_inventory
       SET slot_id = v_sentinel_slot
     WHERE character_id = v_player_id AND item_id = v_stack_item_id;

    -- Step B: move the occupant of the target into the source's old slot.
    UPDATE sgw_inventory
       SET slot_id = 0
     WHERE character_id = v_player_id AND item_id = v_single_item_id;

    -- Step C: move the parked source into the target's old slot.
    UPDATE sgw_inventory
       SET slot_id = 4
     WHERE character_id = v_player_id AND item_id = v_stack_item_id;

    -- Verify both rows ended up swapped.
    SELECT slot_id, stack_size
      INTO v_slot, v_stack
      FROM sgw_inventory
     WHERE character_id = v_player_id AND item_id = v_stack_item_id;

    IF v_slot <> 4 OR v_stack <> 7 THEN
        RAISE EXCEPTION 'after swap: stack expected (slot=4, stack=7), got (slot=%, stack=%)',
            v_slot, v_stack;
    END IF;

    SELECT slot_id, stack_size
      INTO v_slot, v_stack
      FROM sgw_inventory
     WHERE character_id = v_player_id AND item_id = v_single_item_id;

    IF v_slot <> 0 OR v_stack <> 1 THEN
        RAISE EXCEPTION 'after swap: single expected (slot=0, stack=1), got (slot=%, stack=%)',
            v_slot, v_stack;
    END IF;

    -- The sentinel slot must be free at the end of the swap. A bug
    -- that fails to issue Step C would leave the source parked there.
    IF EXISTS (
        SELECT 1
          FROM sgw_inventory
         WHERE character_id = v_player_id
           AND container_id = INV_MAIN
           AND slot_id = v_sentinel_slot
    ) THEN
        RAISE EXCEPTION 'after swap: sentinel slot % must be vacated', v_sentinel_slot;
    END IF;

    -- Total row count and stack-conservation invariants still hold.
    SELECT COUNT(*)
      INTO v_count
      FROM sgw_inventory
     WHERE character_id = v_player_id;

    IF v_count <> 3 THEN
        RAISE EXCEPTION 'after swap: expected 3 inventory rows, got %', v_count;
    END IF;

    SELECT COALESCE(SUM(stack_size), 0)
      INTO v_count
      FROM sgw_inventory
     WHERE character_id = v_player_id AND type_id = v_type_id;

    IF v_count <> 11 THEN
        RAISE EXCEPTION 'after swap: total stack must still conserve at 11, got %', v_count;
    END IF;

    -- Schema-level invariant: no two rows share a (container, slot).
    -- The unique index would have errored mid-procedure if this were
    -- violated; pin it explicitly here too so a future "drop the
    -- unique index" refactor surfaces the consequence.
    SELECT COUNT(*)
      INTO v_count
      FROM (
          SELECT container_id, slot_id, COUNT(*) AS n
            FROM sgw_inventory
           WHERE character_id = v_player_id
           GROUP BY container_id, slot_id
          HAVING COUNT(*) > 1
      ) duplicates;

    IF v_count <> 0 THEN
        RAISE EXCEPTION 'unique-slot invariant violated: % (container, slot) collisions', v_count;
    END IF;

    RAISE NOTICE 'inventory move smoke passed: type_id %, stack item %, single item %, split item %',
        v_type_id, v_stack_item_id, v_single_item_id, v_split_item_id;
END $$;

ROLLBACK;
