SET search_path = resources, pg_catalog;

CREATE TABLE content_chains (
    chain_id    serial PRIMARY KEY,
    description varchar(200),
    scope_type  varchar(20) NOT NULL CHECK (scope_type IN ('mission','space','effect','global')),
    scope_id    integer,
    enabled     boolean NOT NULL DEFAULT true,
    priority    integer NOT NULL DEFAULT 0
);
