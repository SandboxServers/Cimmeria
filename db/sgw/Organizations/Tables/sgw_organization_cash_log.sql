--
-- sgw_organization_cash_log: every committed change to a Team's or a
-- Command's treasury, sgw_organizations.cash (bank-vault BV-08, BV-09;
-- D-BV15: no withdraw cap, every transfer is logged). ViewBankLogs
-- holders read it beside sgw_organization_vault_log; the `bank` log target
-- carries the same change as `org_cash_transfer`.
--
-- One row per committed change, written in the transaction that makes it,
-- so a row exists exactly when the change does.
--
-- A sibling of the vault log rather than more columns on it: that table's
-- fourteen item and slot columns are NOT NULL and mean something for every
-- row, and a cash row has none of them. A reader that wants both merges
-- them with UNION ALL, ordered by (logged_at, tx_id, log_id): the two
-- sequences are separate, so log_id alone is no global order.
--
-- direction:
--   deposit          player wallet -> treasury (organizationTransferCash > 0)
--   withdraw         treasury -> player wallet (organizationTransferCash < 0)
--   vault_expansion  treasury -> nothing: the leader bought a Team vault
--                    +10 step (BV-09, D-BV28). The wallet is untouched, so
--                    the player_cash_* columns are NULL, and
--                    vault_slots_before / vault_slots_after say what the
--                    debit bought.
-- amount is always positive; the direction gives the sign. The CHECKs pin
-- the arithmetic, so a row cannot claim a balance change it did not make.
-- sgw_player.naquadah has no CHECK of its own, so these are the only place
-- a negative wallet would be caught after the fact.
-- account_id / player_id / org_id identify the actor (telemetry rule).
-- tx_id lines the row up with sgw_organization_events rows of the same
-- transaction.
--
-- No foreign keys: the trail must outlive a disband and a character
-- delete, and a RESTRICT key would block the empty-treasury disband.
--

CREATE TABLE sgw_organization_cash_log (
    log_id bigint NOT NULL DEFAULT nextval('sgw_organization_cash_log_log_id_seq'),
    org_id integer NOT NULL,
    org_type smallint NOT NULL,
    account_id integer NOT NULL,
    player_id integer NOT NULL,
    rank smallint NOT NULL,
    direction character varying(16) NOT NULL,
    amount bigint NOT NULL,
    player_cash_before integer,
    player_cash_after integer,
    org_cash_before bigint NOT NULL,
    org_cash_after bigint NOT NULL,
    vault_slots_before smallint,
    vault_slots_after smallint,
    tx_id bigint NOT NULL DEFAULT txid_current(),
    logged_at timestamp with time zone NOT NULL DEFAULT now(),
    CONSTRAINT sgw_organization_cash_log_pkey PRIMARY KEY (log_id),
    CONSTRAINT sgw_organization_cash_log_direction_check
        CHECK (direction IN ('deposit', 'withdraw', 'vault_expansion')),
    CONSTRAINT sgw_organization_cash_log_amount_check CHECK (amount > 0),
    CONSTRAINT sgw_organization_cash_log_nonneg_check
        CHECK (org_cash_before >= 0 AND org_cash_after >= 0
               AND (player_cash_before IS NULL OR player_cash_before >= 0)
               AND (player_cash_after IS NULL OR player_cash_after >= 0)),
    CONSTRAINT sgw_organization_cash_log_arithmetic_check
        CHECK (
            (direction = 'deposit'
                AND org_cash_after = org_cash_before + amount
                AND player_cash_after = player_cash_before - amount
                AND vault_slots_before IS NULL AND vault_slots_after IS NULL)
            OR (direction = 'withdraw'
                AND org_cash_after = org_cash_before - amount
                AND player_cash_after = player_cash_before + amount
                AND vault_slots_before IS NULL AND vault_slots_after IS NULL)
            OR (direction = 'vault_expansion'
                AND org_cash_after = org_cash_before - amount
                AND player_cash_before IS NULL AND player_cash_after IS NULL
                AND vault_slots_after = vault_slots_before + 10)
        )
);
