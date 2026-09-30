-- Tenant analytics source dimensions and lifecycle history.
-- Operational Tenant tables remain authoritative; Analytics receives only bounded references.

ALTER TABLE workspaces
    ADD COLUMN IF NOT EXISTS reporting_timezone TEXT NOT NULL DEFAULT 'UTC';

ALTER TABLE tenant_clients
    ADD COLUMN IF NOT EXISTS reporting_timezone TEXT;

CREATE INDEX IF NOT EXISTS idx_tenant_clients_workspace_status
    ON tenant_clients(workspace_id, status);
