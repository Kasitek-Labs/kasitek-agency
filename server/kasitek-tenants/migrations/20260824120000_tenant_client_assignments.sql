-- Tenant workspace staff-to-client assignments and their immutable lifecycle history.

CREATE TABLE IF NOT EXISTS tenant_client_assignments (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    tenant_client_id TEXT NOT NULL REFERENCES tenant_clients(id) ON DELETE CASCADE,
    tenant_user_id TEXT NOT NULL REFERENCES tenant_users(id) ON DELETE CASCADE,
    assignment_role TEXT NOT NULL DEFAULT 'member'
        CHECK (assignment_role IN ('owner', 'admin', 'member', 'viewer')),
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'revoked')),
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (workspace_id, tenant_client_id, tenant_user_id),
    CHECK ((status = 'revoked') = (revoked_at IS NOT NULL))
);

CREATE INDEX IF NOT EXISTS idx_tenant_client_assignments_workspace_status
    ON tenant_client_assignments(workspace_id, status);

CREATE TABLE IF NOT EXISTS tenant_client_assignment_history (
    id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    tenant_client_id TEXT NOT NULL REFERENCES tenant_clients(id) ON DELETE CASCADE,
    tenant_user_id TEXT NOT NULL REFERENCES tenant_users(id) ON DELETE CASCADE,
    assignment_id TEXT NOT NULL REFERENCES tenant_client_assignments(id) ON DELETE CASCADE,
    action TEXT NOT NULL CHECK (action IN ('assigned', 'unassigned', 'role_changed')),
    assignment_role TEXT NOT NULL
        CHECK (assignment_role IN ('owner', 'admin', 'member', 'viewer')),
    actor_tenant_user_id TEXT REFERENCES tenant_users(id) ON DELETE SET NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_tenant_client_assignment_history_client_occurred_at
    ON tenant_client_assignment_history(tenant_client_id, occurred_at DESC);

CREATE OR REPLACE FUNCTION tenant_client_assignment_history_protect()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'Tenant client assignment history is append-only';
END;
$$;

DROP TRIGGER IF EXISTS tenant_client_assignment_history_protect_trigger
    ON tenant_client_assignment_history;
CREATE TRIGGER tenant_client_assignment_history_protect_trigger
BEFORE UPDATE OR DELETE ON tenant_client_assignment_history
FOR EACH ROW EXECUTE FUNCTION tenant_client_assignment_history_protect();
