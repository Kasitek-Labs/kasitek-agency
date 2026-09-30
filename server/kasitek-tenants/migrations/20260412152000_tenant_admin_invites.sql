CREATE TABLE IF NOT EXISTS tenant_user_invites (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    tenant_user_id TEXT NOT NULL REFERENCES tenant_users(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    invited_by_tenant_user_id TEXT REFERENCES tenant_users(id) ON DELETE SET NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    accepted_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_tenant_user_invites_workspace_id
    ON tenant_user_invites(workspace_id);

CREATE INDEX IF NOT EXISTS idx_tenant_user_invites_tenant_user_id
    ON tenant_user_invites(tenant_user_id);
