use std::{
    env,
    time::{Duration, Instant},
};

use analytics_producer::{
    publish_outbox_once, reconcile_outbox_once, AnalyticsIngestionClient,
    AnalyticsPublisherSettings, ReconciliationCursor, RetryPolicy, MAX_RECONCILIATION_STREAMS,
    TENANT_SOURCE_SERVICE,
};
use chrono::Utc;
use sqlx::PgPool;

const DEFAULT_ENDPOINT: &str = "http://127.0.0.1:4300";
const DEFAULT_PRODUCER_ID: &str = "tenant-platform-customer-analytics";
const DEFAULT_PUBLISHER_ID: &str = "tenant-platform-analytics-publisher";

pub async fn run(pool: PgPool) {
    let Some(settings) = settings_from_env() else {
        observability::tracing::warn!(
            event = "worker.disabled",
            worker = "analytics_publisher",
            reason = "missing_ingestion_token",
            "tenant analytics publication disabled"
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
                event = "worker.failed",
                worker = "analytics_publisher",
                phase = "configuration",
                error_type = %std::any::type_name_of_val(&error),
                "tenant analytics publisher configuration invalid"
            );
            return;
        }
    };
    observability::tracing::info!(
        event = "worker.started",
        worker = "analytics_publisher",
        producer_id = %settings.producer_id,
        batch_size = settings.batch_size,
        "tenant analytics publisher started"
    );
    let reconciliation_every = reconciliation_interval();
    let reconciliation_stream_limit = reconciliation_stream_limit();
    let mut next_reconciliation = Instant::now();
    let mut reconciliation_cursor: Option<ReconciliationCursor> = None;
    loop {
        match publish_outbox_once(&pool, &client, &settings, Utc::now()).await {
            Ok(stats) if stats.claimed > 0 || stats.leases_released > 0 => {
                observability::tracing::info!(
                    event = "queue.item.completed",
                    worker = "analytics_publisher",
                    phase = "publication",
                    claimed = stats.claimed,
                    delivered = stats.delivered,
                    retryable = stats.retryable,
                    quarantined = stats.quarantined,
                    leases_released = stats.leases_released,
                    "tenant analytics publication batch completed"
                )
            }
            Ok(_) => {}
            Err(error) => observability::tracing::error!(
                event = "queue.item.failed",
                worker = "analytics_publisher",
                phase = "publication",
                error_type = %std::any::type_name_of_val(&error),
                "tenant analytics publication batch failed"
            ),
        }
        if Instant::now() >= next_reconciliation {
            match reconcile_outbox_once(
                &pool,
                &client,
                &settings.producer_id,
                TENANT_SOURCE_SERVICE,
                reconciliation_cursor.as_ref(),
                reconciliation_stream_limit,
            )
            .await
            {
                Ok(stats) => {
                    reconciliation_cursor = stats.next_cursor;
                    if stats.attempted > 0 || stats.mismatched > 0 {
                        observability::tracing::info!(
                            event = "queue.item.completed",
                            worker = "analytics_publisher",
                            phase = "reconciliation",
                            attempted = stats.attempted,
                            complete = stats.complete,
                            mismatched = stats.mismatched,
                            "tenant analytics reconciliation completed"
                        );
                    }
                }
                Err(error) => {
                    observability::tracing::error!(
                        event = "queue.item.failed",
                        worker = "analytics_publisher",
                        phase = "reconciliation",
                        error_type = %std::any::type_name_of_val(&error),
                        "tenant analytics reconciliation failed"
                    )
                }
            }
            next_reconciliation = Instant::now() + reconciliation_every;
        }
        tokio::time::sleep(poll_interval()).await;
    }
}

fn settings_from_env() -> Option<AnalyticsPublisherSettings> {
    let token = env::var("ANALYTICS_TENANT_INGESTION_TOKEN")
        .ok()
        .or_else(|| env::var("ANALYTICS_INGESTION_TOKEN").ok())
        .filter(|value| !value.trim().is_empty())?;
    Some(AnalyticsPublisherSettings {
        endpoint: env::var("ANALYTICS_INGESTION_URL")
            .unwrap_or_else(|_| DEFAULT_ENDPOINT.to_string()),
        token,
        producer_id: env_string("ANALYTICS_TENANT_PRODUCER_ID", DEFAULT_PRODUCER_ID),
        publisher_id: env_string("ANALYTICS_TENANT_PUBLISHER_ID", DEFAULT_PUBLISHER_ID),
        batch_size: env_i64("ANALYTICS_PUBLISHER_BATCH_SIZE", 100).clamp(1, 500),
        lease_duration: Duration::from_millis(env_u64(
            "ANALYTICS_PUBLISHER_LEASE_DURATION_MS",
            30_000,
        )),
        request_timeout: Duration::from_millis(env_u64(
            "ANALYTICS_PUBLISHER_REQUEST_TIMEOUT_MS",
            10_000,
        )),
        retry_policy: RetryPolicy {
            base_delay: Duration::from_millis(env_u64(
                "ANALYTICS_PUBLISHER_RETRY_BASE_DELAY_MS",
                2_000,
            )),
            max_delay: Duration::from_millis(env_u64(
                "ANALYTICS_PUBLISHER_RETRY_MAX_DELAY_MS",
                300_000,
            )),
            jitter_window: Duration::from_millis(env_u64(
                "ANALYTICS_PUBLISHER_RETRY_JITTER_MS",
                2_000,
            )),
        },
    })
}

fn poll_interval() -> Duration {
    Duration::from_millis(env_u64("ANALYTICS_PUBLISHER_POLL_INTERVAL_MS", 1_000))
}

fn reconciliation_interval() -> Duration {
    Duration::from_millis(env_u64("ANALYTICS_RECONCILIATION_INTERVAL_MS", 60_000))
}

fn reconciliation_stream_limit() -> i64 {
    env_i64("ANALYTICS_RECONCILIATION_MAX_STREAMS", 100).clamp(1, MAX_RECONCILIATION_STREAMS)
}

fn env_string(name: &str, default: &str) -> String {
    env::var(name)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn env_i64(name: &str, default: i64) -> i64 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn env_u64(name: &str, default: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|value: &u64| *value > 0)
        .unwrap_or(default)
}
