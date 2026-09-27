--
-- sgw_organization_vault_log: every committed Team or Command vault item
-- move (bank-vault BV-07; D-BV15: every transfer is logged, there is no
-- withdraw cap). ViewBankLogs holders read it; the `bank` log target
-- carries the same move as `org_move_accepted`.
--
-- One row per committed move, written in the move transaction, so a row
-- exists exactly when the move does.
--
-- direction: deposit (into the vault) | withdraw (out of it) | within (one
-- vault slot to another). kind: the write, as in `org_move_accepted`:
-- deposit | withdraw | within for a whole stack into an empty slot, else
-- split | merge | swap. item_id is the moved instance as the player
-- named it; new_item_id is the row a split created (NULL otherwise).
-- source_* and target_* are the two ends; *_stack_before / *_stack_after
-- are the stack sizes at each end (0 for an empty slot or a deleted row).
-- account_id / player_id / org_id identify the actor (telemetry rule).
-- tx_id lines the row up with sgw_organization_events rows of the same
-- transaction.
--
-- No foreign keys: the trail must outlive a disband and a character
-- delete, and a RESTRICT key would block the empty-vault disband.
--
-- BV-08 (org cash) owns any extension for cash transfers.
--

CREATE TABLE sgw_organization_vault_log (
    log_id bigint NOT NULL DEFAULT nextval('sgw_organization_vault_log_log_id_seq'),
    org_id integer NOT NULL,
    org_type smallint NOT NULL,
    account_id integer NOT NULL,
    player_id integer NOT NULL,
    rank smallint NOT NULL,
    direction character varying(8) NOT NULL,
    kind character varying(16) NOT NULL,
    item_id integer NOT NULL,
    new_item_id integer,
    type_id integer NOT NULL,
    quantity integer NOT NULL,
    source_container_id integer NOT NULL,
    source_slot_id integer NOT NULL,
    target_container_id integer NOT NULL,
    target_slot_id integer NOT NULL,
    source_stack_before integer NOT NULL,
    source_stack_after integer NOT NULL,
    target_stack_before integer NOT NULL,
    target_stack_after integer NOT NULL,
    tx_id bigint NOT NULL DEFAULT txid_current(),
    logged_at timestamp with time zone NOT NULL DEFAULT now(),
    CONSTRAINT sgw_organization_vault_log_pkey PRIMARY KEY (log_id),
    CONSTRAINT sgw_organization_vault_log_direction_check
        CHECK (direction IN ('deposit', 'withdraw', 'within')),
    CONSTRAINT sgw_organization_vault_log_kind_check
        CHECK (kind IN ('deposit', 'withdraw', 'within', 'split', 'merge', 'swap')),
    CONSTRAINT sgw_organization_vault_log_quantity_check CHECK (quantity > 0)
);
