--
-- TOC entry 3225 (class 0 OID 63055)
-- Dependencies: 242
-- Data for Name: resource_types; Type: TABLE DATA; Schema: resources; Owner: -
--

INSERT INTO resource_types (type, "table", key_column) VALUES ('RESOURCE_Text', 'texts', 'moniker_id');

INSERT INTO resource_types (type, "table", key_column) VALUES ('RESOURCE_Ability', 'abilities', 'ability_id');

INSERT INTO resource_types (type, "table", key_column) VALUES ('RESOURCE_Mission', 'missions', 'mission_id');

INSERT INTO resource_types (type, "table", key_column) VALUES ('RESOURCE_Item', 'items', 'item_id');

INSERT INTO resource_types (type, "table", key_column) VALUES ('RESOURCE_Dialog', 'dialogs', 'dialog_id');

INSERT INTO resource_types (type, "table", key_column) VALUES ('RESOURCE_Sequence', 'sequences', 'sequence_id');

INSERT INTO resource_types (type, "table", key_column) VALUES ('RESOURCE_EventSet', 'event_sets', 'event_set_id');

INSERT INTO resource_types (type, "table", key_column) VALUES ('RESOURCE_InteractionSetMap', 'dialog_set_maps', 'dialog_set_map_id');

INSERT INTO resource_types (type, "table", key_column) VALUES ('RESOURCE_Effect', 'effects', 'effect_id');

INSERT INTO resource_types (type, "table", key_column) VALUES ('RESOURCE_WorldInfo', 'worlds', 'world_id');

INSERT INTO resource_types (type, "table", key_column) VALUES ('RESOURCE_Stargate', 'stargates', 'stargate_id');

INSERT INTO resource_types (type, "table", key_column) VALUES ('RESOURCE_Container', 'containers', 'container_id');

INSERT INTO resource_types (type, "table", key_column) VALUES ('RESOURCE_Blueprint', 'blueprints', 'blueprint_id');

INSERT INTO resource_types (type, "table", key_column) VALUES ('RESOURCE_AppliedScience', 'applied_science', 'id');

INSERT INTO resource_types (type, "table", key_column) VALUES ('RESOURCE_Discipline', 'disciplines', 'discipline_id');

INSERT INTO resource_types (type, "table", key_column) VALUES ('RESOURCE_RacialParadigm', 'racial_paradigm', 'id');

INSERT INTO resource_types (type, "table", key_column) VALUES ('RESOURCE_Interaction', 'interactions', 'interaction_id');

INSERT INTO resource_types (type, "table", key_column) VALUES ('RESOURCE_EventSet', 'event_sets_sequences', 'event_set_id');

INSERT INTO resource_types (type, "table", key_column) VALUES ('RESOURCE_Item', 'items_event_sets', 'item_id');

INSERT INTO resource_types (type, "table", key_column) VALUES ('RESOURCE_Mission', 'mission_steps', 'mission_id');

INSERT INTO resource_types (type, "table", key_column) VALUES ('RESOURCE_Sequence', 'sequences_nvp', 'sequence_id');

INSERT INTO resource_types (type, "table", key_column) VALUES ('RESOURCE_ErrorText', 'error_texts', 'error_id');

