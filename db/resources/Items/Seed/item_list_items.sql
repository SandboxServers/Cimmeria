--
-- TOC entry 3201 (class 0 OID 62955)
-- Dependencies: 218
-- Data for Name: item_list_items; Type: TABLE DATA; Schema: resources; Owner: -
--

INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (1, 1, 5228, 1, 100);

INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (3, 1, 5192, 1, 0);

INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (4, 2, 21, 1, 1000);

INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (6, 2, 3437, 1, 100);

INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (5, 2, 55, 1, 300);

INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (7, 1, 3241, 1, 50);

-- NEW CONTENT (debug hub, crafting): buy list 310 of the crafting supplies
--   vendor (template 314), everything at quantity 1 for 1 naquadah. The store
--   shows rows in item_id order, so ids 3101-3124 keep the groups together:
--   the UAT recipe components (docs/analysis/crafting/audit.md section 2),
--   the research target, the four kickers, one -5 and one -50 Field Crafting
--   Tool per applied science, the five Racial Paradigm Guides and the one
--   Blueprint item. A purchase lands in the main bag (1).
-- 3101: Steel Core (Materials): blueprint 25 set 1 (13), blueprints 412 and 161
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (3101, 310, 5254, 1, 1);
-- 3102: Titanium Core (Materials): blueprint 412 set 2 (5)
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (3102, 310, 5256, 1, 1);
-- 3103: Titanium Plating: blueprint 161 with 5254 (412's product)
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (3103, 310, 5401, 1, 1);
-- 3104: Cell (Bio-Medical), tier 2 Good: alloy blueprint 42's current-tier item
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (3104, 310, 5192, 1, 1);
-- 3105: T1 Cell (Bio-Medical), tier 1 Good: alloy 42's elementary items (5 for Good)
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (3105, 310, 5189, 1, 1);
-- 3106: Crafted Pistol of the Whale: the research and reverse-engineering target
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (3106, 310, 5481, 1, 1);
-- 3107: BioMedical Engineering Research Kicker
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (3107, 310, 5668, 1, 1);
-- 3108: Electronics Engineering Research Kicker
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (3108, 310, 5669, 1, 1);
-- 3109: Power Systems Engineering Research Kicker
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (3109, 310, 5670, 1, 1);
-- 3110: Materials Engineering Research Kicker
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (3110, 310, 5671, 1, 1);
-- 3111: BMAS-5 Field Crafting Tool
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (3111, 310, 5369, 1, 1);
-- 3112: BMAS-50 Field Crafting Tool
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (3112, 310, 8415, 1, 1);
-- 3113: EAS-5 Field Crafting Tool
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (3113, 310, 8402, 1, 1);
-- 3114: EAS-50 Field Crafting Tool
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (3114, 310, 8441, 1, 1);
-- 3115: PSAS-5 Field Crafting Tool
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (3115, 310, 8405, 1, 1);
-- 3116: PSAS-50 Field Crafting Tool
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (3116, 310, 8461, 1, 1);
-- 3117: MAS-5 Field Crafting Tool
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (3117, 310, 8406, 1, 1);
-- 3118: MAS-50 Field Crafting Tool
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (3118, 310, 8451, 1, 1);
-- 3119: Racial Paradigm Guide: Human
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (3119, 310, 7805, 1, 1);
-- 3120: Racial Paradigm Guide: Common
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (3120, 310, 7806, 1, 1);
-- 3121: Racial Paradigm Guide: Asgard
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (3121, 310, 7807, 1, 1);
-- 3122: Racial Paradigm Guide: Goa'uld
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (3122, 310, 7808, 1, 1);
-- 3123: Racial Paradigm Guide: Ancient
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (3123, 310, 7809, 1, 1);
-- 3124: Blueprint: Steel Plating (teaches blueprint 25)
INSERT INTO item_list_items (item_id, item_list_id, design_id, quantity, naquadah) VALUES (3124, 310, 6483, 1, 1);

--
-- TOC entry 3319 (class 0 OID 0)
-- Dependencies: 219
-- Name: item_list_items_item_id_seq; Type: SEQUENCE SET; Schema: resources; Owner: -
--

SELECT pg_catalog.setval('item_list_items_item_id_seq', 7, true);

