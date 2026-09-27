--
-- sgw_gate_mail_item: the item a gate mail holds in escrow (social-systems
-- SS-M2, decision D-SS08).
--
-- When a player mails an item, the item leaves sgw_inventory in the send
-- transaction: the whole row moves here, or, for part of a stack, the
-- source row is decremented and the split-off quantity lands here under a
-- fresh id from sgw_inventory_item_id_seq. It must leave sgw_inventory,
-- because both copies of INVENTORY_ITEM_SELECT read every sgw_inventory row
-- of a character (audit A-14): a row left there would reappear in a bag.
--
-- A standalone table, not INHERITS (sgw_inventory_base): an inheriting table
-- would show escrow rows to any SELECT on sgw_inventory_base, and
-- container_id/slot_id mean nothing while an item is in the mail.
--
-- Every instance column is kept, so taking the item (SS-M3) restores it
-- exactly. At most one item per mail (the PK). Invariants:
--   * only the send transaction inserts a row, together with its mail row;
--     nothing ever attaches an item to an existing mail;
--   * the take, return and expiry paths delete or move the row in the same
--     transaction as the mail change;
--   * deleteMailMessage refuses a mail that still has a row here.
--
-- mail_id -> sgw_gate_mail is ON DELETE CASCADE (_foreign_keys.sql) so that
-- deleting a character, which cascades its mail, is not blocked; the
-- player-facing delete is refused in the application instead.
--

CREATE TABLE sgw_gate_mail_item (
    mail_id integer NOT NULL,
    -- The instance id: unchanged by a whole-row move, new on a split.
    item_id integer NOT NULL,
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
    -- The sender's character, for forensics only (no FK: deleting the sender
    -- must neither be blocked nor erase the trail).
    source_character_id integer NOT NULL,
    -- Epoch seconds, like sgw_gate_mail.sent_time.
    escrowed_at integer NOT NULL,
    CONSTRAINT sgw_gate_mail_item_pkey PRIMARY KEY (mail_id),
    CONSTRAINT sgw_gate_mail_item_item_id_key UNIQUE (item_id),
    -- Mirrors sgw_inventory.local_id_check, so a taken item fits back.
    CONSTRAINT sgw_gate_mail_item_local_id_check CHECK ((item_id >= 10000)),
    CONSTRAINT sgw_gate_mail_item_stack_size_positive_chk CHECK ((stack_size > 0))
);
