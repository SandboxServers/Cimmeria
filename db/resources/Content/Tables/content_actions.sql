SET search_path = resources, pg_catalog;

CREATE TABLE content_actions (
    action_id   serial PRIMARY KEY,
    chain_id    integer NOT NULL REFERENCES content_chains,
    action_type varchar(50) NOT NULL,
    target_id   integer,
    target_key  varchar(200),
    params      jsonb NOT NULL DEFAULT '{}',
    delay_ms    integer NOT NULL DEFAULT 0,
    sort_order  integer NOT NULL DEFAULT 0
);
