CREATE TABLE IF NOT EXISTS tenant_clients (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    slug TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    external_ref TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (workspace_id, slug)
);

CREATE INDEX IF NOT EXISTS idx_tenant_clients_workspace_id
    ON tenant_clients(workspace_id);

CREATE TABLE IF NOT EXISTS tenant_client_users (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    tenant_client_id TEXT NOT NULL REFERENCES tenant_clients(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    password_hash TEXT,
    name TEXT,
    status TEXT NOT NULL DEFAULT 'invited',
    last_login_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (tenant_client_id, email)
);

CREATE INDEX IF NOT EXISTS idx_tenant_client_users_workspace_id
    ON tenant_client_users(workspace_id);

CREATE INDEX IF NOT EXISTS idx_tenant_client_users_tenant_client_id
    ON tenant_client_users(tenant_client_id);

CREATE TABLE IF NOT EXISTS tenant_client_user_roles (
    tenant_client_user_id TEXT NOT NULL REFERENCES tenant_client_users(id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (tenant_client_user_id, role)
);

CREATE TABLE IF NOT EXISTS tenant_client_invites (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    tenant_client_id TEXT NOT NULL REFERENCES tenant_clients(id) ON DELETE CASCADE,
    tenant_client_user_id TEXT NOT NULL REFERENCES tenant_client_users(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    invited_by_tenant_user_id TEXT REFERENCES tenant_users(id) ON DELETE SET NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    accepted_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_tenant_client_invites_workspace_id
    ON tenant_client_invites(workspace_id);

CREATE INDEX IF NOT EXISTS idx_tenant_client_invites_tenant_client_id
    ON tenant_client_invites(tenant_client_id);

CREATE TABLE IF NOT EXISTS tenant_client_sessions (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    tenant_client_id TEXT NOT NULL REFERENCES tenant_clients(id) ON DELETE CASCADE,
    tenant_client_user_id TEXT NOT NULL REFERENCES tenant_client_users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    last_used_at TIMESTAMPTZ,
    ip_address TEXT,
    user_agent TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_tenant_client_sessions_workspace_id
    ON tenant_client_sessions(workspace_id);

CREATE INDEX IF NOT EXISTS idx_tenant_client_sessions_tenant_client_id
    ON tenant_client_sessions(tenant_client_id);

CREATE INDEX IF NOT EXISTS idx_tenant_client_sessions_tenant_client_user_id
    ON tenant_client_sessions(tenant_client_user_id);
