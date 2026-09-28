//! Tenant-owned source reconstruction for authenticated Analytics backfill jobs.
//!
//! Tenant operational tables are the current-state model. Historical analytics facts come from
//! the immutable Tenant outbox, so this worker can replay a bounded source range without reading
//! Analytics storage or reconstructing customer data from mutable rows.

use std::{env, time::Duration};

use analytics_producer::{
    backfill_source_sequence, AnalyticsIngestionClient, AnalyticsOutboxRecord, BackfillRecord,
    RecordCategory, ScopeKind, SourceBackfillWork,
};
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{FromRow, PgPool};

const DEFAULT_ENDPOINT: &str = "http://127.0.0.1:4300";
const DEFAULT_PRODUCER_ID: &str = "tenant-platform-customer-analytics";
const DEFAULT_BATCH_SIZE: i64 = 100;
const DEFAULT_POLL_INTERVAL_MS: u64 = 5_000;
const SOURCE_RECONSTRUCTION_METHOD: &str = "source_reconstruction";

#[derive(Clone, Debug)]
struct Settings {
    endpoint: String,
    token: String,
    producer_id: String,
    batch_size: i64,
    request_timeout: Duration,
}

#[derive(Debug, FromRow)]
struct TenantBackfillRow {
    source_sequence: i64,
    outbox_id: String,
    source_service: String,
    source_domain: String,
    source_stream_key: String,
    record_category: String,
    record_id: String,
    contract_name: String,
    schema_version: i32,
    scope_kind: String,
    scope_external_id: String,
    idempotency_key: String,
    envelope: Value,
    payload_hash: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BackfillStats {
    pub records: usize,
    pub rejected_records: usize,
}

pub async fn run(pool: PgPool) {
    let Some(settings) = settings_from_env() else {
        observability::tracing::warn!(
            event = "worker.disabled",
            worker = "tenant_analytics_backfill",
            reason = "missing_ingestion_token",
            "worker disabled"
        );
        return;
    };
    let client = match AnalyticsIngestionClient::new(
        settings.endpoint.clone(),
        settings.token.clone(),
        settings.request_timeout,
    ) {
        Ok(client) => client,
        Err(error) => {
            observability::tracing::error!(
                event = "worker.startup.failed",
                worker = "tenant_analytics_backfill",
                error_type = %std::any::type_name_of_val(&error),
                "worker transport configuration failed"
            );
            return;
        }
    };
    observability::tracing::info!(
        event = "worker.started",
        worker = "tenant_analytics_backfill",
        producer_id = %settings.producer_id,
        batch_size = settings.batch_size,
        "worker started"
    );

    loop {
        match client.claim_source_backfill_work().await {
            Ok(Some(work)) if work.source_service == "tenant-platform" => {
                match observability::result_span(
                    "queue",
                    "tenant_analytics_backfill.process",
                    process_source_reconstruction(&pool, &client, &settings, &work),
                )
                .await
                {
                    Ok(stats) => observability::tracing::info!(
                        event = "queue.item.completed",
                        worker = "tenant_analytics_backfill",
                        records = stats.records,
                        rejected_records = stats.rejected_records,
                        "backfill job completed"
                    ),
                    Err(_) => observability::tracing::error!(
                        event = "queue.item.failed",
                        worker = "tenant_analytics_backfill",
                        error_code = "backfill_processing_failed",
                        "backfill job failed"
                    ),
                }
            }
            Ok(Some(work)) => observability::tracing::warn!(
                event = "queue.item.ignored",
                worker = "tenant_analytics_backfill",
                source_service = %work.source_service,
                "backfill job belongs to another source service"
            ),
            Ok(None) => {}
            Err(error) => observability::tracing::error!(
                event = "queue.claim.failed",
                worker = "tenant_analytics_backfill",
                error_type = %std::any::type_name_of_val(&error),
                "backfill work claim failed"
            ),
        }
        tokio::time::sleep(poll_interval()).await;
    }
}

async fn process_source_reconstruction(
    pool: &PgPool,
    client: &AnalyticsIngestionClient,
    settings: &Settings,
    work: &SourceBackfillWork,
) -> Result<BackfillStats, String> {
    if work.source_service != "tenant-platform"
        || settings.producer_id.trim().is_empty()
        || !(1..=500).contains(&settings.batch_size)
        || work.method != SOURCE_RECONSTRUCTION_METHOD
        || work.scope_external_id.trim().is_empty()
        || work.requested_range_end <= work.requested_range_start
        || work.remaining_count.is_none()
    {
        return Err("invalid_tenant_source_backfill_work".to_string());
    }

    let workspace_id = resolve_workspace_id(pool, &work.scope_external_id).await?;
    let client_scope = workspace_id != work.scope_external_id;
    let source_domains = source_domains(&work.source_domain);
    let mut stats = BackfillStats::default();
    let mut cursor = None;

    loop {
        let rows = load_tenant_history(
            pool,
            &workspace_id,
            &work.scope_external_id,
            client_scope,
            &source_domains,
            work.requested_range_start,
            work.requested_range_end,
            cursor,
            settings.batch_size,
        )
        .await
        .map_err(|error| format!("load_tenant_history: {error}"))?;
        if rows.is_empty() {
            break;
        }
        let last_sequence = rows
            .last()
            .map(|row| row.source_sequence)
            .expect("non-empty Tenant page has a last row");
        let records = rows
            .into_iter()
            .map(|row| {
                build_backfill_record(row).map(|record| BackfillRecord {
                    source_sequence: backfill_source_sequence(&record.record_id),
                    record,
                })
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("build_tenant_fact: {error}"))?;
        let response = client
            .send_backfill_batch(&settings.producer_id, &work.job_id, &records)
            .await
            .map_err(|error| format!("send_tenant_batch: {error}"))?;
        if response.retryable_count > 0 {
            return Err("tenant_batch_has_retryable_records".to_string());
        }
        stats.records += records.len();
        stats.rejected_records += response.rejected_count;
        cursor = Some(last_sequence);
    }

    Ok(stats)
}

async fn resolve_workspace_id(pool: &PgPool, scope_external_id: &str) -> Result<String, String> {
    if let Some(workspace_id) =
        sqlx::query_scalar::<_, String>("SELECT id FROM workspaces WHERE id = $1")
            .bind(scope_external_id)
            .fetch_optional(pool)
            .await
            .map_err(|error| format!("resolve_workspace: {error}"))?
    {
        return Ok(workspace_id);
    }
    sqlx::query_scalar::<_, String>("SELECT workspace_id FROM tenant_clients WHERE id = $1")
        .bind(scope_external_id)
        .fetch_optional(pool)
        .await
        .map_err(|error| format!("resolve_client_workspace: {error}"))?
        .ok_or_else(|| "backfill_scope_not_found".to_string())
}

fn source_domains(source_domain: &str) -> Vec<String> {
    if source_domain == "tenants" {
        vec![
            "scopes".to_string(),
            "clients".to_string(),
            "leads".to_string(),
            "assignments".to_string(),
        ]
    } else {
        vec![source_domain.to_string()]
    }
}

async fn load_tenant_history(
    pool: &PgPool,
    workspace_id: &str,
    scope_external_id: &str,
    client_scope: bool,
    source_domains: &[String],
    range_start: DateTime<Utc>,
    range_end: DateTime<Utc>,
    cursor: Option<i64>,
    limit: i64,
) -> Result<Vec<TenantBackfillRow>, sqlx::Error> {
    sqlx::query_as::<_, TenantBackfillRow>(
        r#"
        SELECT source_sequence,
               outbox_id,
               source_service,
               source_domain,
               source_stream_key,
               record_category,
               record_id,
               contract_name,
               schema_version,
               scope_kind,
               scope_external_id,
               idempotency_key,
               envelope,
               payload_hash
        FROM analytics_outbox
        WHERE source_service = 'tenant-platform'
          AND source_domain = ANY($1)
          AND source_stream_key = ANY($2)
          AND record_category IN ('event', 'scope_definition')
          AND NULLIF(envelope ->> 'time', '')::timestamptz >= $3
          AND NULLIF(envelope ->> 'time', '')::timestamptz < $4
          AND ($5::boolean = FALSE OR scope_external_id = $6
               OR envelope #>> '{data,tenant_client_id}' = $6
               OR envelope #>> '{data,scope_external_id}' = $6)
          AND ($7::bigint IS NULL OR source_sequence > $7)
        ORDER BY source_sequence
        LIMIT $8
        "#,
    )
    .bind(source_domains)
    .bind(
        source_domains
            .iter()
            .map(|domain| format!("workspace:{workspace_id}:{domain}"))
            .collect::<Vec<_>>(),
    )
    .bind(range_start)
    .bind(range_end)
    .bind(client_scope)
    .bind(scope_external_id)
    .bind(cursor)
    .bind(limit)
    .fetch_all(pool)
    .await
}

fn build_backfill_record(row: TenantBackfillRow) -> Result<AnalyticsOutboxRecord, String> {
    if row.source_service != "tenant-platform"
        || row.source_domain.trim().is_empty()
        || row.source_stream_key.trim().is_empty()
        || row.outbox_id.trim().is_empty()
        || row.record_id.trim().is_empty()
    {
        return Err("tenant_source_identity_missing".to_string());
    }
    let record_category =
        serde_json::from_value::<RecordCategory>(Value::String(row.record_category))
            .map_err(|error| format!("invalid_record_category:{error}"))?;
    if !matches!(
        record_category,
        RecordCategory::Event | RecordCategory::ScopeDefinition
    ) {
        return Err("tenant_backfill_record_category_not_supported".to_string());
    }
    let scope_kind = serde_json::from_value::<ScopeKind>(Value::String(row.scope_kind))
        .map_err(|error| format!("invalid_scope_kind:{error}"))?;
    Ok(AnalyticsOutboxRecord {
        outbox_id: row.outbox_id,
        source_service: row.source_service,
        source_domain: row.source_domain,
        source_stream_key: row.source_stream_key,
        record_category,
        record_id: row.record_id,
        contract_name: row.contract_name,
        schema_version: row.schema_version,
        scope_kind,
        scope_external_id: row.scope_external_id,
        idempotency_key: row.idempotency_key,
        envelope: row.envelope,
        payload_hash: row.payload_hash,
    })
}

fn settings_from_env() -> Option<Settings> {
    let token = env::var("ANALYTICS_TENANT_INGESTION_TOKEN")
        .ok()
        .or_else(|| env::var("ANALYTICS_INGESTION_TOKEN").ok())
        .filter(|value| !value.trim().is_empty())?;
    Some(Settings {
        endpoint: env::var("ANALYTICS_INGESTION_URL")
            .unwrap_or_else(|_| DEFAULT_ENDPOINT.to_string()),
        token,
        producer_id: env::var("ANALYTICS_TENANT_PRODUCER_ID")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_PRODUCER_ID.to_string()),
        batch_size: env::var("ANALYTICS_BACKFILL_BATCH_SIZE")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(DEFAULT_BATCH_SIZE)
            .clamp(1, 500),
        request_timeout: Duration::from_millis(
            env::var("ANALYTICS_BACKFILL_REQUEST_TIMEOUT_MS")
                .ok()
                .and_then(|value| value.parse().ok())
                .filter(|value: &u64| *value > 0)
                .unwrap_or(10_000),
        ),
    })
}

fn poll_interval() -> Duration {
    Duration::from_millis(
        env::var("ANALYTICS_BACKFILL_POLL_INTERVAL_MS")
            .ok()
            .and_then(|value| value.parse().ok())
            .filter(|value: &u64| *value > 0)
            .unwrap_or(DEFAULT_POLL_INTERVAL_MS),
    )
}

#[cfg(test)]
mod tests {
    use super::{build_backfill_record, source_domains, TenantBackfillRow};
    use analytics_producer::RecordCategory;
    use serde_json::json;

    #[test]
    fn tenant_manifest_domain_expands_to_source_domains() {
        assert_eq!(source_domains("tenants").len(), 4);
        assert_eq!(source_domains("leads"), vec!["leads"]);
    }

    #[test]
    fn backfill_record_preserves_privacy_safe_outbox_envelope() {
        let row = TenantBackfillRow {
            source_sequence: 4,
            outbox_id: "outbox-1".to_string(),
            source_service: "tenant-platform".to_string(),
            source_domain: "leads".to_string(),
            source_stream_key: "workspace:workspace-1:leads".to_string(),
            record_category: "event".to_string(),
            record_id: "lead-1".to_string(),
            contract_name: "tenant.lead.lifecycle@1".to_string(),
            schema_version: 1,
            scope_kind: "tenant_client".to_string(),
            scope_external_id: "client-1".to_string(),
            idempotency_key: "lead-1".to_string(),
            envelope: json!({"data": {"status": "new"}}),
            payload_hash: "a".repeat(64),
        };
        let record = build_backfill_record(row).expect("record should be rebuilt");
        assert_eq!(record.record_category, RecordCategory::Event);
        assert_eq!(record.scope_external_id, "client-1");
        assert!(!record.envelope.to_string().contains("email"));
    }
}
