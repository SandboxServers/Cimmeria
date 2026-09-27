--
-- Name: sgw_auction_bid_bid_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--
-- Drives sgw_auction_bid.bid_id (the bid-history row surrogate key).
--

CREATE SEQUENCE sgw_auction_bid_bid_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;
