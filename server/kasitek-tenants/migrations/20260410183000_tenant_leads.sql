CREATE TABLE IF NOT EXISTS tenant_leads (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    tenant_client_id TEXT NOT NULL REFERENCES tenant_clients(id) ON DELETE CASCADE,
    conversation_id TEXT REFERENCES conversations(id) ON DELETE SET NULL,
    created_by_tenant_user_id TEXT REFERENCES tenant_users(id) ON DELETE SET NULL,
    created_by_client_user_id TEXT REFERENCES tenant_client_users(id) ON DELETE SET NULL,
    name TEXT NOT NULL,
    email TEXT NOT NULL,
    phone TEXT,
    company TEXT,
    source TEXT NOT NULL DEFAULT 'manual',
    status TEXT NOT NULL DEFAULT 'new',
    score INTEGER NOT NULL DEFAULT 0,
    notes TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_tenant_leads_workspace_id_created_at
    ON tenant_leads(workspace_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_tenant_leads_tenant_client_id_created_at
    ON tenant_leads(tenant_client_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_tenant_leads_workspace_status
    ON tenant_leads(workspace_id, status);
