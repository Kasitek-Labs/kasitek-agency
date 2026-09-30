use actix_web::{web, HttpRequest, HttpResponse};
use serde::Serialize;
use sqlx::FromRow;

use crate::{error::AppError, middleware};

#[derive(Debug, FromRow, Serialize)]
struct AgentRow {
    id: String,
    name: String,
    status: String,
    config: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list_agents(req: HttpRequest) -> Result<HttpResponse, AppError> {
    let access = middleware::extract_access_context(&req).await?;
    if access.tenant_user_id.is_some() {
        middleware::require_tenant_roles(
            &access.tenant_user_roles,
            &["owner", "admin", "automation_manager", "support"],
        )?;
    }
    let pool = middleware::db_pool(&req)?;

    let agents = sqlx::query_as::<_, AgentRow>(
        r#"
        SELECT
            id,
            name,
            status,
            config,
            created_at,
            updated_at
        FROM workspace_agents
        WHERE workspace_id = $1
        ORDER BY updated_at DESC, created_at DESC
        "#,
    )
    .bind(&access.workspace_id)
    .fetch_all(pool)
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "workspaceId": access.workspace_id,
        "agents": agents
    })))
}

pub async fn get_agent(
    req: HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let access = middleware::extract_access_context(&req).await?;
    if access.tenant_user_id.is_some() {
        middleware::require_tenant_roles(
            &access.tenant_user_roles,
            &["owner", "admin", "automation_manager", "support"],
        )?;
    }
    let pool = middleware::db_pool(&req)?;
    let id = path.into_inner();

    let agent = sqlx::query_as::<_, AgentRow>(
        r#"
        SELECT
            id,
            name,
            status,
            config,
            created_at,
            updated_at
        FROM workspace_agents
        WHERE workspace_id = $1 AND id = $2
        LIMIT 1
        "#,
    )
    .bind(&access.workspace_id)
    .bind(&id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("agent not found".to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "workspaceId": access.workspace_id,
        "agent": agent
    })))
}
