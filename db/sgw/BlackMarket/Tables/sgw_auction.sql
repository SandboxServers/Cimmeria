--
-- Name: sgw_auction; Type: TABLE; Schema: public; Owner: -
--
-- One row per Black Market / Auction House listing. sequence_id is the
-- wire-visible identity the client tracks (BMAuctions / BMAuctionUpdate /
-- BMAuctionRemove all key on it) and is the table's primary key.
--
-- status values (mirrors the issue / restoration findings):
--   0 = active, 1 = sold, 2 = cancelled, 3 = expired.
--
-- auction_length is the UINT8 duration enum the client sends in BMCreateAuction
-- (EBlackMarketTime: VeryShort/Short/Medium/Long/VeryLong); stored SMALLINT
-- because PostgreSQL has no unsigned 1-byte integer. expires_at is derived from
-- created_at + the duration; both are unix epoch seconds (INTEGER), matching the
-- sent_time / read_time columns on sgw_gate_mail.
--

CREATE TABLE sgw_auction (
    sequence_id integer NOT NULL,
    seller_id integer NOT NULL,
    item_id integer NOT NULL,
    item_def_id integer NOT NULL,
    stack_size integer DEFAULT 1 NOT NULL,
    durability integer DEFAULT 0 NOT NULL,
    charges integer DEFAULT 0 NOT NULL,
    starting_price integer DEFAULT 0 NOT NULL,
    buyout_price integer DEFAULT 0 NOT NULL,
    current_bid integer DEFAULT 0 NOT NULL,
    current_bidder integer,
    auction_length smallint DEFAULT 0 NOT NULL,
    created_at integer NOT NULL,
    expires_at integer NOT NULL,
    status smallint DEFAULT 0 NOT NULL
);

--
-- Name: sequence_id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_auction ALTER COLUMN sequence_id SET DEFAULT nextval('sgw_auction_sequence_id_seq'::regclass);
