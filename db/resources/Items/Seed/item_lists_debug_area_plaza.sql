--
-- NEW CONTENT (Debug Area, DA-02): buy list 1300 of the services plaza's
-- munitions vendor (template 1302, docs/content/debug-area.md). Everything
-- costs 1 naquadah and no item, except the pistol (300) and the SMG (1000):
-- sell list 2 buys those two back at exactly that, and a buy row cheaper
-- than any sell list's price for the same item mints naquadah (buy, sell,
-- repeat). `live_db_vendor_arbitrage` guards every buy list against every
-- sell list. The store shows rows in item_id order, so ids
-- 13001-13019 keep the groups together:
--
--   13001-13005  bullet special-ammo reserve stacks (9000-9004), 100 rounds
--   13006-13015  dart special-ammo reserve stacks (9005-9014), 100 rounds
--   13016        Health Slappack TC1 (2893), a stack of 5
--   13017-13019  one SI 3 9mm Pistol (55, 300), SGHC 6 SMG (21, 1000) and
--                CO2 Pistol Dartgun (3584, 1), the weapons those stacks load
--                into
--
-- A purchase lands in the main bag (1) and ignores max_stack_size. The ammo
-- reserve is drawn by a reload once `ammo.finite_special` is on (#1026);
-- deployables are abilities, not items, so the ability granter covers them.
--

SET search_path = resources, pg_catalog;

INSERT INTO item_lists (item_list_id, name) VALUES (1300, 'Debug Area munitions');

INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (13001, 1300, 9000, 100, 1);
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (13002, 1300, 9001, 100, 1);
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (13003, 1300, 9002, 100, 1);
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (13004, 1300, 9003, 100, 1);
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (13005, 1300, 9004, 100, 1);
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (13006, 1300, 9005, 100, 1);
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (13007, 1300, 9006, 100, 1);
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (13008, 1300, 9007, 100, 1);
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (13009, 1300, 9008, 100, 1);
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (13010, 1300, 9009, 100, 1);
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (13011, 1300, 9010, 100, 1);
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (13012, 1300, 9011, 100, 1);
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (13013, 1300, 9012, 100, 1);
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (13014, 1300, 9013, 100, 1);
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (13015, 1300, 9014, 100, 1);
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (13016, 1300, 2893, 5, 1);
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (13017, 1300, 55, 1, 300);
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (13018, 1300, 21, 1, 1000);
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (13019, 1300, 3584, 1, 1);

-- These rows load after the base files' sequence footers, so they raise the
-- sequences themselves.
SELECT pg_catalog.setval('item_lists_item_list_id_seq', GREATEST((SELECT MAX(item_list_id) FROM item_lists), (SELECT last_value FROM item_lists_item_list_id_seq)), true);
SELECT pg_catalog.setval('item_list_items_item_id_seq', GREATEST((SELECT MAX(item_id) FROM item_list_items), (SELECT last_value FROM item_list_items_item_id_seq)), true);
