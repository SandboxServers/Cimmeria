--
-- Name: sgw_auction_sequence_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--
-- Drives sgw_auction.sequence_id (the auction's wire-visible primary key,
-- echoed to the client in BMAuctions / BMAuctionUpdate / BMAuctionRemove).
--

CREATE SEQUENCE sgw_auction_sequence_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;
