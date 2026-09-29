use ::config::{environment_allows_loopback_origins, is_loopback_origin};
use actix_cors::Cors;
use actix_web::{http::header::HeaderValue, web, App, HttpServer};
use dotenv::dotenv;
use sqlx::postgres::PgPoolOptions;

pub mod analytics;
mod analytics_backfill;
mod analytics_publisher;
mod config;
mod controllers;
mod error;
mod mail;
mod middleware;
mod routes;
mod types;

use config::AppConfig;

fn origin_matches_platform_domain(origin: &HeaderValue, platform_base_domain: &str) -> bool {
    let Some(origin) = origin.to_str().ok() else {
        return false;
    };

    let Some((scheme, remainder)) = origin.split_once("://") else {
        return false;
    };

    if !matches!(scheme, "http" | "https") {
        return false;
    }

    let host = remainder
        .split('/')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("");
    if host.is_empty() {
        return false;
    }

    let normalized_base = platform_base_domain
        .trim()
        .trim_matches('.')
        .to_ascii_lowercase();

    if normalized_base.is_empty() {
        return false;
    }

    let host = host.to_ascii_lowercase();
    host == normalized_base || host.ends_with(&format!(".{normalized_base}"))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    observability::init_with_service("info", "kasitek-tenants");
    let _telemetry_guard = observability::ShutdownGuard;

    let config = AppConfig::from_env().expect("failed to load tenant server config");

    let db_pool = observability::result_span(
        "database",
        "postgres.connect",
        PgPoolOptions::new()
            .max_connections(config.database_max_connections)
            .connect(&config.database_url),
    )
    .await
    .expect("failed to connect tenant postgres");

    observability::in_span(
        "migration",
        "tenants.sqlx_migrate",
        sqlx::migrate!("./migrations").run(&db_pool),
    )
    .await
    .expect("failed to run tenant migrations");

    match std::env::var("KASITEK_PROCESS").ok().as_deref() {
        Some("analytics-publisher") => {
            observability::tracing::info!(
                event = "worker.started",
                worker = "analytics_publisher_and_backfill"
            );
            tokio::join!(
                observability::in_span(
                    "worker",
                    "analytics_publisher",
                    analytics_publisher::run(db_pool.clone())
                ),
                observability::in_span(
                    "worker",
                    "analytics_backfill",
                    analytics_backfill::run(db_pool)
                )
            );
            return Ok(());
        }
        Some("analytics-backfill-worker") => {
            observability::tracing::info!(event = "worker.started", worker = "analytics_backfill");
            observability::in_span(
                "worker",
                "analytics_backfill",
                analytics_backfill::run(db_pool),
            )
            .await;
            return Ok(());
        }
        _ => {}
    }

    let port = config.port;
    let allowed_origins = config.allowed_origins.clone();
    let allow_loopback_origins = environment_allows_loopback_origins(&config.environment);

    observability::tracing::info!(
        event = "service.startup",
        service = "kasitek-tenants",
        process = "api",
        port,
        "server starting"
    );

    HttpServer::new(move || {
        let mut cors = Cors::default()
            .allowed_methods(vec!["GET", "POST", "PATCH", "PUT", "DELETE", "OPTIONS"])
            .allowed_headers(vec!["Content-Type", "Authorization", "x-admin-secret"])
            .max_age(3600);

        for origin in &allowed_origins {
            cors = cors.allowed_origin(origin);
        }

        let platform_base_domain = config.platform_base_domain.clone();
        if allow_loopback_origins || platform_base_domain.is_some() {
            cors = cors.allowed_origin_fn(move |origin, _req_head| {
                (allow_loopback_origins && origin.to_str().is_ok_and(is_loopback_origin))
                    || platform_base_domain.as_deref().is_some_and(|base_domain| {
                        origin_matches_platform_domain(origin, base_domain)
                    })
            });
        }

        if config.cors_allow_credentials {
            cors = cors.supports_credentials();
        }

        App::new()
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(config.clone()))
            .wrap(cors)
            .wrap(observability::correlation_id())
            .wrap(observability::http_metrics())
            .wrap(observability::http_tracing_logger())
            .wrap(observability::http_logger())
            .configure(routes::configure)
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
