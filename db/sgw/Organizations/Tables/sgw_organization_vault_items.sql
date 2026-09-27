--
-- sgw_organization_vault_items: the items a Team (container 19) or a
-- Command (container 20) holds in its vault (bank-vault BV-07; D-BV09,
-- D-BV13, D-BV14, D-BV18).
--
-- sgw_inventory.character_id is NOT NULL (audit A-23), so org-owned items
-- live here. When a member deposits, the item leaves sgw_inventory in the
-- move transaction: the whole row moves here with its item_id unchanged,
-- or, for part of a stack, the source row is decremented and the split-off
-- quantity lands here under a fresh id from sgw_inventory_item_id_seq. A
-- withdrawal is the reverse. item_id is therefore unique across
-- sgw_inventory, this table and sgw_gate_mail_item by that discipline (one
-- sequence, moves are delete plus insert); nothing enforces it across
-- tables, and the live-DB tests assert it after every kind of move.
--
-- A standalone table, not INHERITS (sgw_inventory_base), for the reason
-- sgw_gate_mail_item gives: an inheriting table would show vault rows to
-- any SELECT on sgw_inventory_base. No query that touches sgw_inventory,
-- by character_id or by item_id alone, can reach a vault row.
--
-- Every instance column is kept, so a withdrawal restores the item
-- exactly. org_type is a copy of the organization's type, pinned by the
-- composite foreign key (org_id, org_type) -> sgw_organizations, so a Team
-- can only hold container 19 and a Command only 20.
--
-- ON DELETE RESTRICT, not CASCADE (D-BV18): an organization whose vault
-- holds items is never deleted with it. Voluntary disbands refuse while
-- org_vault_is_empty_sql is false (D-BV13), and the member-delete trigger
-- keeps a memberless organization instead of deleting it, so RESTRICT never
-- fires on those paths. A hand-run DELETE of a non-empty organization fails
-- loudly instead of destroying items. This holds because every vault write
-- takes the organization row lock (lock_org) first, the same lock the
-- disband and the trigger decide under.
--
-- bound items never enter a shared vault: another member could withdraw
-- them, which would transfer a bound item. The Rust move path refuses them
-- first with reason bound_item_not_org_storable; the CHECK is the last line.
--
-- deposited_by_player_id is forensics only (no FK: deleting the depositor
-- must neither be blocked nor erase the trail). The full trail is
-- sgw_organization_vault_log.
--

CREATE TABLE sgw_organization_vault_items (
    item_id integer NOT NULL,
    org_id integer NOT NULL,
    org_type smallint NOT NULL,
    container_id integer NOT NULL,
    slot_id integer NOT NULL,
    type_id integer NOT NULL,
    stack_size integer NOT NULL,
    charges integer NOT NULL,
    durability integer NOT NULL,
    flags integer NOT NULL,
    bound boolean NOT NULL,
    ammo integer NOT NULL,
    cur_ammo_type integer NOT NULL,
    ammo_type resources."EAmmoType" NOT NULL,
    ammo_types resources."EAmmoType"[] NOT NULL,
    deposited_by_player_id integer NOT NULL,
    deposited_at timestamp with time zone NOT NULL DEFAULT now(),
    CONSTRAINT sgw_organization_vault_items_pkey PRIMARY KEY (item_id),
    -- Leads with org_id, so it also serves the vault load and the
    -- empty check; the vault has no other index. DEFERRABLE INITIALLY
    -- IMMEDIATE so it is checked at the end of each statement, not per
    -- row: a swap of two vault slots is then one UPDATE, with no parking
    -- slot outside 0-99.
    CONSTRAINT sgw_organization_vault_items_slot_key UNIQUE (org_id, container_id, slot_id)
        DEFERRABLE INITIALLY IMMEDIATE,
    CONSTRAINT sgw_organization_vault_items_container_check
        CHECK ((org_type = 1 AND container_id = 19) OR (org_type = 2 AND container_id = 20)),
    CONSTRAINT sgw_organization_vault_items_slot_check CHECK (slot_id BETWEEN 0 AND 99),
    -- Mirrors sgw_inventory.local_id_check, so a withdrawn item fits back.
    CONSTRAINT sgw_organization_vault_items_local_id_check CHECK (item_id >= 10000),
    CONSTRAINT sgw_organization_vault_items_stack_size_positive_chk CHECK (stack_size > 0),
    CONSTRAINT sgw_organization_vault_items_not_bound_chk CHECK (NOT bound)
);
