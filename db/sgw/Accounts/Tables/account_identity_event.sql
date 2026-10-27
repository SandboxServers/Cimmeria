-- Audit trail of identity changes. Holds no subject, email or IP address.
CREATE TABLE account_identity_event (
    event_id bigserial PRIMARY KEY,
    account_id integer NOT NULL,
    identity_id bigint,
    kind character varying(24) NOT NULL CHECK (kind IN ('signup', 'link', 'auto_link', 'unlink',
        'link_refused', 'refresh_reuse', 'password_cleared', 'link_code_issued')),
    provider character varying(32),
    at timestamp with time zone DEFAULT now() NOT NULL
);
CREATE INDEX account_identity_event_account_idx ON account_identity_event (account_id);
