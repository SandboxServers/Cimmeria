--
-- Name: sgw_auction_bid; Type: TABLE; Schema: public; Owner: -
--
-- Bid history for the Black Market. One row per accepted bid. Retained for
-- refund / audit (the live "current" bid is denormalised onto sgw_auction).
-- created_at is unix epoch seconds, matching sgw_auction.
--

CREATE TABLE sgw_auction_bid (
    bid_id integer NOT NULL,
    sequence_id integer NOT NULL,
    bidder_id integer NOT NULL,
    amount bigint NOT NULL,
    created_at integer NOT NULL
);

--
-- Name: bid_id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY sgw_auction_bid ALTER COLUMN bid_id SET DEFAULT nextval('sgw_auction_bid_bid_id_seq'::regclass);
