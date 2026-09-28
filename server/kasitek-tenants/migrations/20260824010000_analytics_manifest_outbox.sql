-- Allow the Tenant Platform producer to persist versioned source manifests in its transactional
-- outbox.

ALTER TABLE analytics_outbox
    DROP CONSTRAINT IF EXISTS analytics_outbox_record_category_check;

ALTER TABLE analytics_outbox
    ADD CONSTRAINT analytics_outbox_record_category_check
    CHECK (record_category IN ('event', 'observation', 'scope_definition', 'manifest'));
