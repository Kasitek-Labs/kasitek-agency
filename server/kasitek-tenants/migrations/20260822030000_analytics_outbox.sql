-- A-07: Tenant Platform-owned transactional Analytics outbox.
--
-- Tenant Platform remains authoritative for workspace/client operations. This table is only the
-- durable source publisher boundary and never stores Analytics database credentials.

CREATE TABLE IF NOT EXISTS analytics_outbox (
    outbox_id TEXT PRIMARY KEY CHECK (length(trim(outbox_id)) BETWEEN 1 AND 512),
    source_sequence BIGINT GENERATED ALWAYS AS IDENTITY UNIQUE,
    source_service TEXT NOT NULL CHECK (source_service = 'tenant-platform'),
    source_domain TEXT NOT NULL CHECK (length(trim(source_domain)) BETWEEN 1 AND 256),
    source_stream_key TEXT NOT NULL CHECK (length(trim(source_stream_key)) BETWEEN 1 AND 512),
    record_category TEXT NOT NULL CHECK (record_category IN ('event', 'observation', 'scope_definition')),
    record_id TEXT NOT NULL CHECK (length(trim(record_id)) BETWEEN 1 AND 512),
    contract_name TEXT NOT NULL CHECK (contract_name ~ '^[a-z][a-z0-9_.-]+@[0-9]+$'),
    schema_version INTEGER NOT NULL CHECK (schema_version > 0),
    scope_kind TEXT NOT NULL CHECK (scope_kind IN ('core_organization', 'tenant_workspace', 'tenant_client')),
    scope_external_id TEXT NOT NULL CHECK (length(trim(scope_external_id)) BETWEEN 1 AND 512),
    idempotency_key TEXT NOT NULL CHECK (length(trim(idempotency_key)) BETWEEN 1 AND 512),
    envelope JSONB NOT NULL CHECK (jsonb_typeof(envelope) = 'object'),
    payload_hash TEXT NOT NULL CHECK (payload_hash ~ '^[0-9a-f]{64}$'),
    delivery_status TEXT NOT NULL DEFAULT 'pending' CHECK (delivery_status IN ('pending', 'leased', 'delivered', 'quarantined')),
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    lease_owner TEXT CHECK (lease_owner IS NULL OR length(trim(lease_owner)) BETWEEN 1 AND 256),
    lease_expires_at TIMESTAMPTZ,
    last_error_code TEXT CHECK (last_error_code IS NULL OR last_error_code ~ '^[a-z][a-z0-9_.-]+$'),
    delivered_at TIMESTAMPTZ,
    quarantined_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (source_service, record_category, record_id),
    UNIQUE (source_stream_key, source_sequence),
    CHECK ((delivery_status = 'leased') = (lease_owner IS NOT NULL AND lease_expires_at IS NOT NULL)),
    CHECK (delivered_at IS NULL OR delivery_status = 'delivered'),
    CHECK (quarantined_at IS NULL OR delivery_status = 'quarantined')
);

CREATE INDEX IF NOT EXISTS analytics_outbox_claim_idx
    ON analytics_outbox (delivery_status, next_attempt_at, source_sequence);
CREATE INDEX IF NOT EXISTS analytics_outbox_stream_sequence_idx
    ON analytics_outbox (source_stream_key, source_sequence);
CREATE INDEX IF NOT EXISTS analytics_outbox_error_idx
    ON analytics_outbox (delivery_status, last_error_code, updated_at);

CREATE OR REPLACE FUNCTION analytics_outbox_set_updated_at()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    NEW.updated_at := NOW();
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION analytics_outbox_protect_content()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    IF TG_OP = 'DELETE' OR NEW.outbox_id IS DISTINCT FROM OLD.outbox_id
       OR NEW.source_sequence IS DISTINCT FROM OLD.source_sequence
       OR NEW.source_service IS DISTINCT FROM OLD.source_service
       OR NEW.source_domain IS DISTINCT FROM OLD.source_domain
       OR NEW.source_stream_key IS DISTINCT FROM OLD.source_stream_key
       OR NEW.record_category IS DISTINCT FROM OLD.record_category
       OR NEW.record_id IS DISTINCT FROM OLD.record_id
       OR NEW.contract_name IS DISTINCT FROM OLD.contract_name
       OR NEW.schema_version IS DISTINCT FROM OLD.schema_version
       OR NEW.scope_kind IS DISTINCT FROM OLD.scope_kind
       OR NEW.scope_external_id IS DISTINCT FROM OLD.scope_external_id
       OR NEW.idempotency_key IS DISTINCT FROM OLD.idempotency_key
       OR NEW.envelope IS DISTINCT FROM OLD.envelope
       OR NEW.payload_hash IS DISTINCT FROM OLD.payload_hash
       THEN
        RAISE EXCEPTION 'Analytics outbox content is immutable';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS analytics_outbox_updated_at_trigger ON analytics_outbox;
CREATE TRIGGER analytics_outbox_updated_at_trigger
BEFORE UPDATE ON analytics_outbox
FOR EACH ROW EXECUTE FUNCTION analytics_outbox_set_updated_at();

DROP TRIGGER IF EXISTS analytics_outbox_content_trigger ON analytics_outbox;
CREATE TRIGGER analytics_outbox_content_trigger
BEFORE UPDATE OR DELETE ON analytics_outbox
FOR EACH ROW EXECUTE FUNCTION analytics_outbox_protect_content();

COMMENT ON TABLE analytics_outbox IS
    'Tenant Platform transactional Analytics facts; domain changes and outbox rows commit together';
