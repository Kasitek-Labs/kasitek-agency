ALTER TABLE conversations
    ADD COLUMN IF NOT EXISTS tenant_client_id TEXT REFERENCES tenant_clients(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_conversations_workspace_tenant_client_last_message
    ON conversations(workspace_id, tenant_client_id, last_message_at DESC);
