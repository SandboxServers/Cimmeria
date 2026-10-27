-- External sign-in (OIDC provider) linked to an account. The subject is the
-- provider's stable subject id. email is set only when the provider is marked
-- trusted_email. A linked account holds no password (guard triggers in
-- db/sgw/_functions.sql and _triggers.sql).
CREATE TABLE account_identity (
    identity_id bigserial PRIMARY KEY,
    account_id integer NOT NULL,
    provider character varying(32) NOT NULL,
    issuer text NOT NULL,
    subject text NOT NULL,
    display_name character varying(64) DEFAULT ''::character varying NOT NULL,
    email text,                       -- set only when verified by a trusted_email provider
    email_verified_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    last_used_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT account_identity_subject_key UNIQUE (provider, issuer, subject)
);
CREATE INDEX account_identity_account_idx ON account_identity (account_id);
CREATE INDEX account_identity_email_idx ON account_identity (lower(email)) WHERE email IS NOT NULL;
