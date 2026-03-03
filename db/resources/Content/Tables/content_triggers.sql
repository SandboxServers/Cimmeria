SET search_path = resources, pg_catalog;

CREATE TABLE content_triggers (
    trigger_id  serial PRIMARY KEY,
    chain_id    integer NOT NULL REFERENCES content_chains,
    event_type  varchar(50) NOT NULL,
    event_key   varchar(200),
    scope       varchar(20) NOT NULL DEFAULT 'player' CHECK (scope IN ('player','space','global')),
    once        boolean NOT NULL DEFAULT false,
    sort_order  integer NOT NULL DEFAULT 0
);
