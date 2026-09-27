--
-- Sequence: sgw_organization_events_event_id_seq
-- Drives sgw_organization_events.event_id (owned via _sequence_ownership.sql).
--

CREATE SEQUENCE sgw_organization_events_event_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;
