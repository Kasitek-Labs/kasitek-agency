use crate::types::HealthResponse;
use actix_web::HttpResponse;
use chrono::Utc;

pub async fn root() -> HttpResponse {
    HttpResponse::Ok().json(HealthResponse {
        status: "ok",
        service: "tenant-server",
        timestamp: Utc::now(),
    })
}

pub async fn health_check() -> HttpResponse {
    root().await
}
