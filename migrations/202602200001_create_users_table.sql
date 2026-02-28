CREATE TABLE IF NOT EXISTS users (
    id uuid PRIMARY KEY,
    email varchar NOT NULL UNIQUE,
    password_hash varchar NOT NULL,
    auth_provider varchar NOT NULL,
    role varchar NOT NULL DEFAULT 'user',
    created_at timestamptz NOT NULL,
    updated_at timestamptz NOT NULL
);
