CREATE TABLE IF NOT EXISTS workspace_email_domains (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    provider TEXT NOT NULL DEFAULT 'resend',
    provider_domain_id TEXT NOT NULL UNIQUE,
    domain TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL DEFAULT 'pending',
    region TEXT,
    capabilities JSONB NOT NULL DEFAULT '{}'::jsonb,
    verified_at TIMESTAMPTZ,
    last_synced_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_workspace_email_domains_workspace_id
    ON workspace_email_domains(workspace_id);

CREATE TABLE IF NOT EXISTS workspace_email_domain_records (
    id TEXT PRIMARY KEY,
    email_domain_id TEXT NOT NULL REFERENCES workspace_email_domains(id) ON DELETE CASCADE,
    record_group TEXT NOT NULL,
    record_type TEXT NOT NULL,
    record_name TEXT NOT NULL,
    record_value TEXT NOT NULL,
    record_status TEXT NOT NULL DEFAULT 'not_started',
    ttl TEXT,
    priority INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_workspace_email_domain_records_domain_id
    ON workspace_email_domain_records(email_domain_id);

CREATE TABLE IF NOT EXISTS workspace_email_settings (
    workspace_id TEXT PRIMARY KEY REFERENCES workspaces(id) ON DELETE CASCADE,
    sending_mode TEXT NOT NULL DEFAULT 'shared',
    sender_name TEXT,
    reply_to_email TEXT,
    shared_sender_local_part TEXT NOT NULL DEFAULT 'no-reply',
    custom_domain_id TEXT REFERENCES workspace_email_domains(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO workspace_email_settings (workspace_id, sender_name, created_at, updated_at)
SELECT
    w.id,
    w.display_name,
    NOW(),
    NOW()
FROM workspaces w
ON CONFLICT (workspace_id) DO NOTHING;
