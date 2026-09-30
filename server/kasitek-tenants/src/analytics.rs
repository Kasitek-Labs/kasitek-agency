//! Tenant Platform's transactional Analytics producer boundary.
//!
//! Domain commands should call these helpers with their existing transaction. The helper does not
//! perform network I/O; the source outbox is the durability boundary and a later publisher sends
//! leased rows to the private Analytics ingestion API.

use analytics_producer::{
    build_event, build_manifest, build_observation, build_scope_definition, enqueue_outbox,
    AnalyticsOutboxRecord, EnqueueResult, EventInput, ManifestInput, ObservationInput,
    ProducerConfig, ProducerError, ScopeDefinitionInput,
};
use chrono::{DateTime, Duration, Utc};
use serde_json::json;
use sqlx::{Postgres, Transaction};

pub use analytics_producer::{
    apply_delivery_outcome, claim_outbox_batch, release_expired_leases, AnalyticsEnvelope,
    DeliveryOutcome, OutboxRow, RetryPolicy, ScopeKind,
};

pub fn producer_config() -> ProducerConfig {
    ProducerConfig::tenant_platform()
}

pub async fn enqueue_record(
    transaction: &mut Transaction<'_, Postgres>,
    record: &AnalyticsOutboxRecord,
) -> Result<EnqueueResult, ProducerError> {
    enqueue_outbox(transaction, record).await
}

pub async fn enqueue_event(
    transaction: &mut Transaction<'_, Postgres>,
    input: EventInput,
) -> Result<EnqueueResult, ProducerError> {
    let record = build_event(&producer_config(), input)?;
    enqueue_record(transaction, &record).await
}

pub async fn enqueue_observation(
    transaction: &mut Transaction<'_, Postgres>,
    input: ObservationInput,
) -> Result<EnqueueResult, ProducerError> {
    let record = build_observation(&producer_config(), input)?;
    enqueue_record(transaction, &record).await
}

pub async fn enqueue_scope_definition(
    transaction: &mut Transaction<'_, Postgres>,
    input: ScopeDefinitionInput,
) -> Result<EnqueueResult, ProducerError> {
    let record = build_scope_definition(&producer_config(), input)?;
    enqueue_record(transaction, &record).await
}

pub async fn enqueue_manifest(
    transaction: &mut Transaction<'_, Postgres>,
    input: ManifestInput,
) -> Result<EnqueueResult, ProducerError> {
    let record = build_manifest(&producer_config(), input)?;
    enqueue_record(transaction, &record).await
}

const MANIFEST_REFRESH_SECONDS: i64 = 3_600;
const TENANT_SCOPE_CONTRACTS: &[&str] = &["tenant.scope.definition@1"];
const TENANT_CLIENT_CONTRACTS: &[&str] =
    &["tenant.scope.definition@1", "tenant.client.lifecycle@1"];
const TENANT_LEAD_CONTRACTS: &[&str] = &["tenant.lead.lifecycle@1"];
const TENANT_ASSIGNMENT_CONTRACTS: &[&str] = &["tenant.client.assignment@1"];
const TENANT_CLIENT_METRICS: &[&str] = &["tenant.clients.lifecycle_events"];
const TENANT_LEAD_METRICS: &[&str] = &["tenant.leads.lifecycle_events"];
const TENANT_ASSIGNMENT_METRICS: &[&str] = &["tenant.client.assignment_events"];
const TENANT_SCOPE_DIMENSIONS: &[&str] = &[];
const TENANT_CLIENT_DIMENSIONS: &[&str] = &["tenant_client", "client_lifecycle_action", "status"];
const TENANT_LEAD_DIMENSIONS: &[&str] = &[
    "tenant_client",
    "lead_action",
    "lead_status",
    "lead_source",
    "lead_score_band",
    "lead_reason_code",
    "lead_actor_type",
];
const TENANT_ASSIGNMENT_DIMENSIONS: &[&str] = &[
    "tenant_client",
    "assignment_action",
    "assignment_role",
    "status",
];

pub async fn workspace_event_count(
    transaction: &mut Transaction<'_, Postgres>,
    source_stream_key: &str,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(
        r#"SELECT COUNT(*)::bigint
           FROM analytics_outbox
           WHERE source_service = 'tenant-platform'
             AND source_stream_key = $1
             AND record_category = 'event'"#,
    )
    .bind(source_stream_key)
    .fetch_one(&mut **transaction)
    .await
}

async fn enqueue_domain_manifest(
    transaction: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    source_domain: &str,
    stream_key: &str,
    manifest_key: &str,
    supported_contracts: &[&str],
    supported_metrics: &[&str],
    supported_dimensions: &[&str],
    approved_checkpoint_count: i64,
    collected_at: DateTime<Utc>,
) -> Result<EnqueueResult, ProducerError> {
    let manifest_version = collected_at
        .timestamp()
        .div_euclid(MANIFEST_REFRESH_SECONDS)
        .max(1);
    enqueue_manifest(
        transaction,
        ManifestInput {
            record_id: format!("manifest:{stream_key}:{manifest_key}:{manifest_version}"),
            source_domain: source_domain.to_string(),
            source_stream_key: stream_key.to_string(),
            contract_name: "source.manifest@1".to_string(),
            schema_version: 1,
            scope_kind: ScopeKind::TenantWorkspace,
            scope_external_id: workspace_id.to_string(),
            manifest_key: manifest_key.to_string(),
            manifest_version,
            provider: None,
            connection_ref: None,
            supported_contracts: supported_contracts
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
            supported_observation_contracts: Vec::new(),
            supported_metrics: supported_metrics
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
            supported_dimensions: supported_dimensions
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
            expected_refresh_seconds: MANIFEST_REFRESH_SECONDS,
            reliable_coverage_start: None,
            highest_authoritative_source_sequence: None,
            approved_checkpoint_count: Some(approved_checkpoint_count),
            approved_checkpoint_hash: None,
            last_successful_collection_at: Some(collected_at),
            last_attempted_collection_at: Some(collected_at),
            safe_failure_code: None,
            next_expected_at: Some(collected_at + Duration::seconds(MANIFEST_REFRESH_SECONDS)),
            manifest_state: "active".to_string(),
            generated_at: collected_at,
            idempotency_key: None,
        },
    )
    .await
}

pub async fn enqueue_scope_manifest(
    transaction: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    scope_count: i64,
    collected_at: DateTime<Utc>,
) -> Result<EnqueueResult, ProducerError> {
    enqueue_domain_manifest(
        transaction,
        workspace_id,
        "scopes",
        &format!("workspace:{workspace_id}:scopes"),
        "tenant.scopes",
        TENANT_SCOPE_CONTRACTS,
        &[],
        TENANT_SCOPE_DIMENSIONS,
        scope_count,
        collected_at,
    )
    .await
}

pub async fn enqueue_client_manifest(
    transaction: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    event_count: i64,
    collected_at: DateTime<Utc>,
) -> Result<EnqueueResult, ProducerError> {
    enqueue_domain_manifest(
        transaction,
        workspace_id,
        "clients",
        &format!("workspace:{workspace_id}:clients"),
        "tenant.clients",
        TENANT_CLIENT_CONTRACTS,
        TENANT_CLIENT_METRICS,
        TENANT_CLIENT_DIMENSIONS,
        event_count,
        collected_at,
    )
    .await
}

pub async fn enqueue_lead_manifest(
    transaction: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    event_count: i64,
    collected_at: DateTime<Utc>,
) -> Result<EnqueueResult, ProducerError> {
    enqueue_domain_manifest(
        transaction,
        workspace_id,
        "leads",
        &format!("workspace:{workspace_id}:leads"),
        "tenant.leads",
        TENANT_LEAD_CONTRACTS,
        TENANT_LEAD_METRICS,
        TENANT_LEAD_DIMENSIONS,
        event_count,
        collected_at,
    )
    .await
}

pub async fn enqueue_assignment_manifest(
    transaction: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    event_count: i64,
    collected_at: DateTime<Utc>,
) -> Result<EnqueueResult, ProducerError> {
    enqueue_domain_manifest(
        transaction,
        workspace_id,
        "assignments",
        &format!("workspace:{workspace_id}:assignments"),
        "tenant.assignments",
        TENANT_ASSIGNMENT_CONTRACTS,
        TENANT_ASSIGNMENT_METRICS,
        TENANT_ASSIGNMENT_DIMENSIONS,
        event_count,
        collected_at,
    )
    .await
}

pub async fn enqueue_workspace_scope_definition(
    transaction: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    source_version: i64,
    reporting_timezone: &str,
) -> Result<EnqueueResult, ProducerError> {
    enqueue_scope_definition(
        transaction,
        ScopeDefinitionInput {
            record_id: format!("tenant-scope:workspace:{workspace_id}:{source_version}"),
            source_domain: "scopes".to_string(),
            source_stream_key: format!("workspace:{workspace_id}:scopes"),
            contract_name: "tenant.scope.definition@1".to_string(),
            schema_version: 1,
            scope_kind: ScopeKind::TenantWorkspace,
            scope_external_id: workspace_id.to_string(),
            parent_scope_external_id: None,
            reporting_timezone: reporting_timezone.to_string(),
            status: "active".to_string(),
            source_version,
            idempotency_key: Some(format!(
                "tenant-scope:workspace:{workspace_id}:{source_version}"
            )),
        },
    )
    .await
}

pub async fn enqueue_client_scope_definition(
    transaction: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    tenant_client_id: &str,
    source_version: i64,
    reporting_timezone: &str,
    status: &str,
) -> Result<EnqueueResult, ProducerError> {
    enqueue_scope_definition(
        transaction,
        ScopeDefinitionInput {
            record_id: format!("tenant-scope:client:{tenant_client_id}:{source_version}"),
            source_domain: "scopes".to_string(),
            source_stream_key: format!("workspace:{workspace_id}:scopes"),
            contract_name: "tenant.scope.definition@1".to_string(),
            schema_version: 1,
            scope_kind: ScopeKind::TenantClient,
            scope_external_id: tenant_client_id.to_string(),
            parent_scope_external_id: Some(workspace_id.to_string()),
            reporting_timezone: reporting_timezone.to_string(),
            status: status.to_string(),
            source_version,
            idempotency_key: Some(format!(
                "tenant-scope:client:{tenant_client_id}:{source_version}"
            )),
        },
    )
    .await
}

pub async fn enqueue_client_lifecycle(
    transaction: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    tenant_client_id: &str,
    action: &str,
    status: &str,
    occurred_at: DateTime<Utc>,
) -> Result<EnqueueResult, ProducerError> {
    enqueue_event(
        transaction,
        EventInput {
            record_id: format!("tenant-client:{tenant_client_id}:lifecycle:{action}:{occurred_at}"),
            source_domain: "clients".to_string(),
            source_stream_key: format!("workspace:{workspace_id}:clients"),
            contract_name: "tenant.client.lifecycle@1".to_string(),
            schema_version: 1,
            scope_kind: ScopeKind::TenantWorkspace,
            scope_external_id: workspace_id.to_string(),
            subject_type: "tenant_client".to_string(),
            subject_id: tenant_client_id.to_string(),
            occurred_at,
            attributes: json!({
                "tenant_client_id": tenant_client_id,
                "action": action,
                "status": status,
                "occurred_at": occurred_at,
            }),
            correlation_id: None,
            causation_id: None,
            idempotency_key: Some(format!(
                "tenant-client:{tenant_client_id}:lifecycle:{action}:{occurred_at}"
            )),
        },
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn enqueue_client_assignment_lifecycle(
    transaction: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    tenant_client_id: &str,
    tenant_user_id: &str,
    assignment_id: &str,
    action: &str,
    status: &str,
    assignment_role: &str,
    history_id: &str,
    occurred_at: DateTime<Utc>,
) -> Result<EnqueueResult, ProducerError> {
    enqueue_event(
        transaction,
        EventInput {
            record_id: format!("tenant-assignment:{assignment_id}:history:{history_id}"),
            source_domain: "assignments".to_string(),
            source_stream_key: format!("workspace:{workspace_id}:assignments"),
            contract_name: "tenant.client.assignment@1".to_string(),
            schema_version: 1,
            scope_kind: ScopeKind::TenantWorkspace,
            scope_external_id: workspace_id.to_string(),
            subject_type: "tenant_client_assignment".to_string(),
            subject_id: assignment_id.to_string(),
            occurred_at,
            attributes: json!({
                "tenant_client_id": tenant_client_id,
                "tenant_user_id": tenant_user_id,
                "action": action,
                "status": status,
                "assignment_role": assignment_role,
                "occurred_at": occurred_at,
            }),
            correlation_id: None,
            causation_id: None,
            idempotency_key: Some(format!(
                "tenant-assignment:{assignment_id}:history:{history_id}"
            )),
        },
    )
    .await
}

pub async fn enqueue_lead_created(
    transaction: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    tenant_client_id: &str,
    lead_id: &str,
    source: &str,
    score_band: &str,
    occurred_at: DateTime<Utc>,
) -> Result<EnqueueResult, ProducerError> {
    enqueue_event(
        transaction,
        EventInput {
            record_id: format!("tenant-lead:{lead_id}:created"),
            source_domain: "leads".to_string(),
            source_stream_key: format!("workspace:{workspace_id}:leads"),
            contract_name: "tenant.lead.lifecycle@1".to_string(),
            schema_version: 1,
            scope_kind: ScopeKind::TenantClient,
            scope_external_id: tenant_client_id.to_string(),
            subject_type: "tenant_lead".to_string(),
            subject_id: lead_id.to_string(),
            occurred_at,
            attributes: json!({
                "lead_id": lead_id,
                "tenant_client_id": tenant_client_id,
                "action": "created",
                "status": "new",
                "source": source,
                "score_band": score_band,
                "occurred_at": occurred_at,
            }),
            correlation_id: None,
            causation_id: None,
            idempotency_key: Some(format!("tenant-lead:{lead_id}:created")),
        },
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub async fn enqueue_lead_status_changed(
    transaction: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    tenant_client_id: &str,
    lead_id: &str,
    previous_status: &str,
    status: &str,
    reason_code: &str,
    actor_type: &str,
    source: &str,
    score_band: &str,
    history_id: &str,
    occurred_at: DateTime<Utc>,
) -> Result<EnqueueResult, ProducerError> {
    enqueue_event(
        transaction,
        EventInput {
            record_id: format!("tenant-lead:{lead_id}:status:{history_id}"),
            source_domain: "leads".to_string(),
            source_stream_key: format!("workspace:{workspace_id}:leads"),
            contract_name: "tenant.lead.lifecycle@1".to_string(),
            schema_version: 1,
            scope_kind: ScopeKind::TenantClient,
            scope_external_id: tenant_client_id.to_string(),
            subject_type: "tenant_lead".to_string(),
            subject_id: lead_id.to_string(),
            occurred_at,
            attributes: json!({
                "lead_id": lead_id,
                "tenant_client_id": tenant_client_id,
                "action": "status_changed",
                "status": status,
                "previous_status": previous_status,
                "reason_code": reason_code,
                "actor_type": actor_type,
                "source": source,
                "score_band": score_band,
                "occurred_at": occurred_at,
            }),
            correlation_id: None,
            causation_id: None,
            idempotency_key: Some(format!("tenant-lead:{lead_id}:status:{history_id}")),
        },
    )
    .await
}
