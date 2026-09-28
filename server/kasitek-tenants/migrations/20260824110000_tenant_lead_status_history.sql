-- Tenant lead lifecycle history. The current tenant_leads row remains the operational projection;
-- this append-only table is the source-owned history boundary for analytics and auditing.

CREATE TABLE IF NOT EXISTS tenant_lead_status_history (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    tenant_client_id TEXT NOT NULL REFERENCES tenant_clients(id) ON DELETE CASCADE,
    lead_id TEXT NOT NULL REFERENCES tenant_leads(id) ON DELETE CASCADE,
    from_status TEXT NOT NULL,
    to_status TEXT NOT NULL,
    reason_code TEXT NOT NULL,
    actor_type TEXT NOT NULL CHECK (actor_type IN ('tenant_user', 'client_user', 'system')),
    changed_by_tenant_user_id TEXT REFERENCES tenant_users(id) ON DELETE SET NULL,
    changed_by_client_user_id TEXT REFERENCES tenant_client_users(id) ON DELETE SET NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (lead_id, id)
);

CREATE INDEX IF NOT EXISTS idx_tenant_lead_status_history_lead_occurred_at
    ON tenant_lead_status_history(lead_id, occurred_at DESC);

CREATE INDEX IF NOT EXISTS idx_tenant_lead_status_history_workspace_occurred_at
    ON tenant_lead_status_history(workspace_id, occurred_at DESC);

CREATE OR REPLACE FUNCTION tenant_lead_status_history_protect()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'Tenant lead status history is append-only';
END;
$$;

DROP TRIGGER IF EXISTS tenant_lead_status_history_protect_trigger
    ON tenant_lead_status_history;
CREATE TRIGGER tenant_lead_status_history_protect_trigger
BEFORE UPDATE OR DELETE ON tenant_lead_status_history
FOR EACH ROW EXECUTE FUNCTION tenant_lead_status_history_protect();
