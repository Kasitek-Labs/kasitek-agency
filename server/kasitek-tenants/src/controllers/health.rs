use actix_web::HttpResponse;
use chrono::Utc;
use types::HealthResponse;

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
