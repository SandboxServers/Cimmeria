\set ON_ERROR_STOP on

BEGIN;

DO $$
DECLARE
    v_account_id integer := -900001;
    v_player_id integer := -900001;
    v_account_name text := '__codex_vendor_smoke__';
    v_player_name text := '__CodexVendorSmoke__';
    v_world_id integer;
    v_world text;
    v_vendor_template_id integer;
    v_sell_item_list integer;
    v_type_id integer;
    v_unit_price integer;
    v_buy_vendor_template_id integer;
    v_buy_item_list integer;
    v_purchase_store_index integer;
    v_purchase_type_id integer;
    v_purchase_quantity integer;
    v_purchase_price integer;
    v_start_cash integer;
    v_base_item_id integer;
    v_full_item_id integer;
    v_stack_item_id integer;
    v_buyback_copy_item_id integer;
    v_main_copy_item_id integer;
    v_purchase_item_id integer;
    v_cash integer;
    v_count integer;
    v_slot integer;
    v_stack integer;
    v_flags integer;
    v_grant_type_id integer;
    v_durability integer;
    v_charges integer;
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

    SELECT et.template_id, et.sell_item_list, ili.design_id, ili.naquadah
      INTO v_vendor_template_id, v_sell_item_list, v_type_id, v_unit_price
      FROM resources.entity_templates et
      JOIN resources.item_list_items ili
        ON ili.item_list_id = et.sell_item_list
      JOIN resources.items ri
        ON ri.item_id = ili.design_id
     WHERE et.sell_item_list IS NOT NULL
       AND (ri.flags & 1024) <> 0
       AND ili.naquadah > 0
     ORDER BY et.template_id, ili.item_id
     LIMIT 1;

    IF v_type_id IS NULL THEN
        RAISE EXCEPTION 'no positive-price sellable vendor item found';
    END IF;

    SELECT template_id,
           buy_item_list,
           store_index,
           design_id,
           quantity,
           naquadah
      INTO v_buy_vendor_template_id,
           v_buy_item_list,
           v_purchase_store_index,
           v_purchase_type_id,
           v_purchase_quantity,
           v_purchase_price
      FROM (
          SELECT et.template_id,
                 et.buy_item_list,
                 (ROW_NUMBER() OVER (PARTITION BY et.template_id ORDER BY ili.item_id) - 1)::integer AS store_index,
                 ili.item_id AS list_item_id,
                 ili.design_id,
                 GREATEST(ili.quantity, 1)::integer AS quantity,
                 ili.naquadah
            FROM resources.entity_templates et
            JOIN resources.item_list_items ili
              ON ili.item_list_id = et.buy_item_list
           WHERE et.buy_item_list IS NOT NULL
             AND ili.naquadah > 0
      ) ordered_items
     WHERE NOT EXISTS (
           SELECT 1
             FROM resources.item_list_prices ilp
            WHERE ilp.item_id = ordered_items.list_item_id
     )
     ORDER BY template_id, store_index
     LIMIT 1;

    IF v_purchase_type_id IS NULL THEN
        RAISE EXCEPTION 'no positive-price vendor purchase item without item prerequisites found';
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

    v_full_item_id := v_base_item_id;
    v_stack_item_id := v_base_item_id + 1;
    v_buyback_copy_item_id := v_base_item_id + 2;
    v_main_copy_item_id := v_base_item_id + 3;
    v_purchase_item_id := v_base_item_id + 4;
    v_start_cash := LEAST(
        1000000000::bigint,
        GREATEST(
            (v_unit_price::bigint * 10) + 1000,
            (v_purchase_price::bigint * 2) + 1000
        )
    )::integer;

    INSERT INTO account (account_id, account_name, password, accesslevel, enabled)
    VALUES (v_account_id, v_account_name, 'smoke', 0, true);

    INSERT INTO sgw_player (
        account_id,
        player_id,
        level,
        alignment,
        archetype,
        gender,
        player_name,
        extra_name,
        world_location,
        bodyset,
        title,
        pos_x,
        pos_y,
        pos_z,
        heading,
        naquadah,
        exp,
        first_login,
        world_id,
        skin_color_id
    )
    VALUES (
        v_account_id,
        v_player_id,
        1,
        0,
        0,
        1,
        v_player_name,
        '',
        v_world,
        'BS_HumanMale.BS_HumanMale',
        0,
        0,
        0,
        0,
        0,
        v_start_cash,
        0,
        1,
        v_world_id,
        0
    );

    INSERT INTO sgw_inventory (
        item_id,
        character_id,
        type_id,
        stack_size,
        slot_id,
        container_id,
        bound,
        durability,
        charges,
        flags,
        ammo
    )
    VALUES (
        v_full_item_id,
        v_player_id,
        v_type_id,
        1,
        0,
        1,
        false,
        77,
        3,
        0,
        0
    );

    SELECT COUNT(*)
      INTO v_count
      FROM resources.item_list_items ili
      JOIN sgw_inventory inv
        ON inv.type_id = ili.design_id
      JOIN resources.items ri
        ON ri.item_id = inv.type_id
     WHERE inv.character_id = v_player_id
       AND inv.item_id = v_full_item_id
       AND ili.item_list_id = v_sell_item_list
       AND inv.container_id = ANY(ARRAY[1, 3])
       AND inv.bound = false
       AND (ri.flags & 1024) <> 0
       AND ili.naquadah = v_unit_price;

    IF v_count <> 1 THEN
        RAISE EXCEPTION 'full-stack inventory item did not appear in the vendor sell-price query';
    END IF;

    UPDATE sgw_inventory
       SET container_id = 16,
           slot_id = 0,
           flags = v_unit_price
     WHERE character_id = v_player_id
       AND item_id = v_full_item_id;

    UPDATE sgw_player
       SET naquadah = naquadah + v_unit_price
     WHERE player_id = v_player_id;

    SELECT container_id, slot_id, flags, durability, charges
      INTO v_count, v_slot, v_flags, v_durability, v_charges
      FROM sgw_inventory
     WHERE character_id = v_player_id
       AND item_id = v_full_item_id;

    IF v_count <> 16 OR v_slot <> 0 OR v_flags <> v_unit_price OR v_durability <> 77 OR v_charges <> 3 THEN
        RAISE EXCEPTION 'full-stack sell did not preserve item state and buyback price';
    END IF;

    SELECT flags
      INTO v_flags
      FROM sgw_inventory
     WHERE character_id = v_player_id
       AND container_id = 16
       AND item_id = v_full_item_id
     ORDER BY slot_id;

    IF v_flags <> v_unit_price THEN
        RAISE EXCEPTION 'buyback price query would not return the stored full-stack price';
    END IF;

    SELECT naquadah
      INTO v_cash
      FROM sgw_player
     WHERE player_id = v_player_id;

    IF v_cash <> v_start_cash + v_unit_price THEN
        RAISE EXCEPTION 'full-stack sell cash mismatch: got %, expected %', v_cash, v_start_cash + v_unit_price;
    END IF;

    UPDATE sgw_player
       SET naquadah = naquadah - v_unit_price
     WHERE player_id = v_player_id;

    UPDATE sgw_inventory
       SET container_id = 1,
           slot_id = 1,
           flags = 0
     WHERE character_id = v_player_id
       AND item_id = v_full_item_id
       AND container_id = 16;

    SELECT container_id, slot_id, flags
      INTO v_count, v_slot, v_flags
      FROM sgw_inventory
     WHERE character_id = v_player_id
       AND item_id = v_full_item_id;

    IF v_count <> 1 OR v_slot <> 1 OR v_flags <> 0 THEN
        RAISE EXCEPTION 'full-stack buyback did not return item to main inventory with flags cleared';
    END IF;

    SELECT naquadah
      INTO v_cash
      FROM sgw_player
     WHERE player_id = v_player_id;

    IF v_cash <> v_start_cash THEN
        RAISE EXCEPTION 'full-stack buyback cash mismatch: got %, expected %', v_cash, v_start_cash;
    END IF;

    INSERT INTO sgw_inventory (
        item_id,
        character_id,
        type_id,
        stack_size,
        slot_id,
        container_id,
        bound,
        durability,
        charges,
        flags,
        ammo
    )
    VALUES (
        v_stack_item_id,
        v_player_id,
        v_type_id,
        3,
        2,
        1,
        false,
        61,
        4,
        0,
        0
    );

    INSERT INTO sgw_inventory (
        item_id,
        character_id,
        type_id,
        stack_size,
        slot_id,
        container_id,
        bound,
        durability,
        charges,
        ammo_type,
        ammo_types,
        ammo,
        flags
    )
    SELECT v_buyback_copy_item_id,
           character_id,
           type_id,
           2,
           0,
           16,
           bound,
           durability,
           charges,
           ammo_type,
           ammo_types,
           ammo,
           v_unit_price
      FROM sgw_inventory
     WHERE character_id = v_player_id
       AND item_id = v_stack_item_id;

    UPDATE sgw_inventory
       SET stack_size = stack_size - 2
     WHERE character_id = v_player_id
       AND item_id = v_stack_item_id;

    UPDATE sgw_player
       SET naquadah = naquadah + (v_unit_price * 2)
     WHERE player_id = v_player_id;

    SELECT stack_size, container_id, slot_id, flags
      INTO v_stack, v_count, v_slot, v_flags
      FROM sgw_inventory
     WHERE character_id = v_player_id
       AND item_id = v_stack_item_id;

    IF v_stack <> 1 OR v_count <> 1 OR v_slot <> 2 OR v_flags <> 0 THEN
        RAISE EXCEPTION 'partial sell did not leave the source stack in main inventory';
    END IF;

    SELECT stack_size, container_id, slot_id, flags, durability, charges
      INTO v_stack, v_count, v_slot, v_flags, v_durability, v_charges
      FROM sgw_inventory
     WHERE character_id = v_player_id
       AND item_id = v_buyback_copy_item_id;

    IF v_stack <> 2 OR v_count <> 16 OR v_slot <> 0 OR v_flags <> v_unit_price OR v_durability <> 61 OR v_charges <> 4 THEN
        RAISE EXCEPTION 'partial sell did not create the expected buyback stack copy';
    END IF;

    SELECT naquadah
      INTO v_cash
      FROM sgw_player
     WHERE player_id = v_player_id;

    IF v_cash <> v_start_cash + (v_unit_price * 2) THEN
        RAISE EXCEPTION 'partial sell cash mismatch: got %, expected %', v_cash, v_start_cash + (v_unit_price * 2);
    END IF;

    INSERT INTO sgw_inventory (
        item_id,
        character_id,
        type_id,
        stack_size,
        slot_id,
        container_id,
        bound,
        durability,
        charges,
        ammo_type,
        ammo_types,
        ammo,
        flags
    )
    SELECT v_main_copy_item_id,
           character_id,
           type_id,
           1,
           3,
           1,
           bound,
           durability,
           charges,
           ammo_type,
           ammo_types,
           ammo,
           0
      FROM sgw_inventory
     WHERE character_id = v_player_id
       AND item_id = v_buyback_copy_item_id;

    UPDATE sgw_inventory
       SET stack_size = stack_size - 1
     WHERE character_id = v_player_id
       AND item_id = v_buyback_copy_item_id;

    UPDATE sgw_player
       SET naquadah = naquadah - v_unit_price
     WHERE player_id = v_player_id;

    SELECT stack_size, container_id, slot_id, flags, durability, charges
      INTO v_stack, v_count, v_slot, v_flags, v_durability, v_charges
      FROM sgw_inventory
     WHERE character_id = v_player_id
       AND item_id = v_main_copy_item_id;

    IF v_stack <> 1 OR v_count <> 1 OR v_slot <> 3 OR v_flags <> 0 OR v_durability <> 61 OR v_charges <> 4 THEN
        RAISE EXCEPTION 'partial buyback did not create the expected main-bag stack copy';
    END IF;

    SELECT stack_size, container_id, flags
      INTO v_stack, v_count, v_flags
      FROM sgw_inventory
     WHERE character_id = v_player_id
       AND item_id = v_buyback_copy_item_id;

    IF v_stack <> 1 OR v_count <> 16 OR v_flags <> v_unit_price THEN
        RAISE EXCEPTION 'partial buyback did not leave the expected remainder in buyback';
    END IF;

    UPDATE sgw_player
       SET naquadah = naquadah - v_unit_price
     WHERE player_id = v_player_id;

    UPDATE sgw_inventory
       SET container_id = 1,
           slot_id = 4,
           flags = 0
     WHERE character_id = v_player_id
       AND item_id = v_buyback_copy_item_id
       AND container_id = 16;

    SELECT COUNT(*)
      INTO v_count
      FROM sgw_inventory
     WHERE character_id = v_player_id
       AND container_id = 16;

    IF v_count <> 0 THEN
        RAISE EXCEPTION 'buyback inventory should be empty after buying back remaining stack';
    END IF;

    SELECT naquadah
      INTO v_cash
      FROM sgw_player
     WHERE player_id = v_player_id;

    IF v_cash <> v_start_cash THEN
        RAISE EXCEPTION 'final cash mismatch: got %, expected %', v_cash, v_start_cash;
    END IF;

    SELECT free_slot
      INTO v_slot
      FROM generate_series(0, 31) AS free_slot
     WHERE NOT EXISTS (
           SELECT 1
             FROM sgw_inventory inv
            WHERE inv.character_id = v_player_id
              AND inv.container_id = 1
              AND inv.slot_id = free_slot
     )
     ORDER BY free_slot
     LIMIT 1;

    IF v_slot <> 0 THEN
        RAISE EXCEPTION 'purchase slot reservation should fill main-bag hole 0, got %', v_slot;
    END IF;

    UPDATE sgw_player
       SET naquadah = naquadah - v_purchase_price
     WHERE player_id = v_player_id;

    INSERT INTO sgw_inventory (
        item_id,
        character_id,
        type_id,
        stack_size,
        slot_id,
        container_id,
        bound,
        durability,
        charges
    )
    SELECT v_purchase_item_id,
           v_player_id,
           ri.item_id,
           v_purchase_quantity,
           v_slot,
           1,
           false,
           100,
           ri.charges
      FROM resources.items ri
     WHERE ri.item_id = v_purchase_type_id;

    GET DIAGNOSTICS v_count = ROW_COUNT;

    IF v_count <> 1 THEN
        RAISE EXCEPTION 'purchase insert did not grant exactly one inventory row';
    END IF;

    SELECT inv.type_id, inv.stack_size, inv.container_id, inv.slot_id, inv.durability, inv.charges, inv.flags
      INTO v_grant_type_id, v_stack, v_count, v_slot, v_durability, v_charges, v_flags
      FROM sgw_inventory inv
     WHERE inv.character_id = v_player_id
       AND inv.item_id = v_purchase_item_id;

    IF v_grant_type_id <> v_purchase_type_id OR v_stack <> v_purchase_quantity OR v_count <> 1 OR v_slot <> 0 OR v_durability <> 100 OR v_flags <> 0 THEN
        RAISE EXCEPTION 'purchase grant row mismatch';
    END IF;

    SELECT charges
      INTO v_count
      FROM resources.items
     WHERE item_id = v_purchase_type_id;

    IF v_charges <> v_count THEN
        RAISE EXCEPTION 'purchase grant did not copy default item charges';
    END IF;

    SELECT naquadah
      INTO v_cash
      FROM sgw_player
     WHERE player_id = v_player_id;

    IF v_cash <> v_start_cash - v_purchase_price THEN
        RAISE EXCEPTION 'purchase cash mismatch: got %, expected %', v_cash, v_start_cash - v_purchase_price;
    END IF;

    RAISE NOTICE 'vendor store smoke passed: sell vendor_template_id %, sell item %, buy vendor_template_id %, store_index %, buy item %',
        v_vendor_template_id, v_type_id, v_buy_vendor_template_id, v_purchase_store_index, v_purchase_type_id;
END $$;

ROLLBACK;
