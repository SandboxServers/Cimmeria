-- Refresh tokens of the identity sign-in. Only the SHA-256 of a token string
-- is stored. A family_id groups the tokens of one sign-in chain.
CREATE TABLE auth_refresh_token (
    token_hash bytea PRIMARY KEY,     -- SHA-256 of the token string
    family_id character varying(32) NOT NULL,
    account_id integer NOT NULL,
    issued_at timestamp with time zone DEFAULT now() NOT NULL,
    last_used_at timestamp with time zone DEFAULT now() NOT NULL,
    family_expires_at timestamp with time zone NOT NULL,
    revoked_at timestamp with time zone,
    replaced_by bytea
);
CREATE INDEX auth_refresh_token_family_idx ON auth_refresh_token (family_id);
CREATE INDEX auth_refresh_token_account_idx ON auth_refresh_token (account_id);
