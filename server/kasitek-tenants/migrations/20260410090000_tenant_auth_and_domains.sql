CREATE TABLE IF NOT EXISTS workspace_domains (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    domain TEXT NOT NULL UNIQUE,
    is_primary BOOLEAN NOT NULL DEFAULT FALSE,
    verified_at TIMESTAMPTZ,
    dns_status TEXT NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_workspace_domains_workspace_id
    ON workspace_domains(workspace_id);

INSERT INTO workspace_domains (id, workspace_id, domain, is_primary, verified_at, dns_status, created_at, updated_at)
SELECT
    'wd_' || md5(wb.workspace_id || ':' || wb.custom_domain),
    wb.workspace_id,
    lower(wb.custom_domain),
    TRUE,
    NOW(),
    'verified',
    NOW(),
    NOW()
FROM workspace_branding wb
WHERE wb.custom_domain IS NOT NULL
  AND trim(wb.custom_domain) <> ''
ON CONFLICT (domain) DO NOTHING;

CREATE TABLE IF NOT EXISTS tenant_users (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    name TEXT,
    status TEXT NOT NULL DEFAULT 'active',
    last_login_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (workspace_id, email)
);

CREATE INDEX IF NOT EXISTS idx_tenant_users_workspace_id
    ON tenant_users(workspace_id);

CREATE TABLE IF NOT EXISTS tenant_user_roles (
    tenant_user_id TEXT NOT NULL REFERENCES tenant_users(id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (tenant_user_id, role)
);

CREATE TABLE IF NOT EXISTS tenant_sessions (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    tenant_user_id TEXT NOT NULL REFERENCES tenant_users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    last_used_at TIMESTAMPTZ,
    ip_address TEXT,
    user_agent TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_tenant_sessions_workspace_id
    ON tenant_sessions(workspace_id);

CREATE INDEX IF NOT EXISTS idx_tenant_sessions_tenant_user_id
    ON tenant_sessions(tenant_user_id);
