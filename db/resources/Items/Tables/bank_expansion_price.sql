--
-- Vault expansion prices (bank-vault campaign, BV-05; decision D-BV02 in
-- docs/analysis/bank-vault/README.md). The personal vault starts at 40
-- slots (sgw_player.bank_slots) and grows in steps of 10 up to 100, each
-- step bought at a Banker. One row per step, keyed by the size the step
-- buys: to_slots 50 is the step from 40 to 50.
--
-- The price lives here so it can be tuned without code. A missing row
-- makes that step unbuyable (expand_rejected reason=price_missing), never
-- free. The purchase reads the row in the same statement that raises
-- bank_slots and debits the cash
-- (crates/base-session/src/base/bank_expand/persist.rs).
--
-- Name: bank_expansion_price; Type: TABLE; Schema: resources; Owner: -
--

CREATE TABLE bank_expansion_price (
    to_slots smallint NOT NULL,
    price_naquadah integer NOT NULL,
    CONSTRAINT bank_expansion_price_step CHECK (((to_slots >= 50) AND (to_slots <= 100) AND ((to_slots % 10) = 0))),
    CONSTRAINT bank_expansion_price_non_negative CHECK ((price_naquadah >= 0))
);
