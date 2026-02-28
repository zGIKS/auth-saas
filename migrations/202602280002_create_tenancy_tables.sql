CREATE TABLE IF NOT EXISTS tenants (
    id uuid PRIMARY KEY,
    name varchar NOT NULL,
    schema_name varchar NOT NULL UNIQUE,
    admin_user_id uuid NOT NULL REFERENCES users(id),
    anon_key varchar NOT NULL UNIQUE,
    secret_key_hash varchar NOT NULL,
    google_client_id varchar,
    google_client_secret varchar,
    google_redirect_uri varchar,
    status varchar NOT NULL DEFAULT 'active',
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL
);

CREATE TABLE IF NOT EXISTS tenant_memberships (
    id uuid PRIMARY KEY,
    tenant_id uuid NOT NULL REFERENCES tenants(id),
    user_id uuid NOT NULL REFERENCES users(id),
    role varchar NOT NULL DEFAULT 'user',
    status varchar NOT NULL DEFAULT 'active',
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL,
    UNIQUE (tenant_id, user_id)
);
