SET search_path = resources, pg_catalog;

CREATE TABLE content_counters (
    counter_id    serial PRIMARY KEY,
    chain_id      integer NOT NULL REFERENCES content_chains,
    counter_name  varchar(100) NOT NULL,
    target_value  integer NOT NULL DEFAULT 1,
    reset_on      varchar(50) NOT NULL DEFAULT 'mission_complete'
);
