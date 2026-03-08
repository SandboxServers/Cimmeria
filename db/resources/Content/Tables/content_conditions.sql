SET search_path = resources, pg_catalog;

CREATE TABLE content_conditions (
    condition_id   serial PRIMARY KEY,
    chain_id       integer NOT NULL REFERENCES content_chains,
    condition_type varchar(50) NOT NULL,
    target_id      integer,
    target_key     varchar(200),
    operator       varchar(10) NOT NULL DEFAULT 'eq',
    value          varchar(200),
    sort_order     integer NOT NULL DEFAULT 0
);
