use actix_web::{HttpRequest, HttpResponse};
use sqlx::FromRow;

use crate::{error::AppError, middleware};

#[derive(Debug, FromRow)]
struct UsageTotalsRow {
    requests_this_month: i64,
    tokens_this_month: i64,
    conversations_this_month: i64,
}

#[derive(Debug, FromRow)]
struct UsageLimitsRow {
    requests_per_month: i64,
    tokens_per_month: i64,
    conversations_per_month: i64,
}

pub async fn get_usage(req: HttpRequest) -> Result<HttpResponse, AppError> {
    let access = middleware::extract_access_context(&req).await?;
    if access.tenant_user_id.is_some() {
        middleware::require_tenant_roles(
            &access.tenant_user_roles,
            &["owner", "admin", "account_manager", "viewer"],
        )?;
    }
    let pool = middleware::db_pool(&req)?;

    let usage = sqlx::query_as::<_, UsageTotalsRow>(
        r#"
        SELECT
            COALESCE(SUM(CASE WHEN event_type = 'request' THEN quantity ELSE 0 END), 0)::bigint AS requests_this_month,
            COALESCE(SUM(CASE WHEN event_type = 'token' THEN quantity ELSE 0 END), 0)::bigint AS tokens_this_month,
            COALESCE(SUM(CASE WHEN event_type = 'conversation' THEN quantity ELSE 0 END), 0)::bigint AS conversations_this_month
        FROM workspace_usage_events
        WHERE workspace_id = $1
          AND created_at >= date_trunc('month', NOW())
        "#
    )
    .bind(&access.workspace_id)
    .fetch_one(pool)
    .await?;

    let limits = sqlx::query_as::<_, UsageLimitsRow>(
        r#"
        SELECT requests_per_month, tokens_per_month, conversations_per_month
        FROM workspace_usage_limits
        WHERE workspace_id = $1
        LIMIT 1
        "#,
    )
    .bind(&access.workspace_id)
    .fetch_optional(pool)
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "workspaceId": access.workspace_id,
        "usage": {
            "requestsThisMonth": usage.requests_this_month,
            "tokensThisMonth": usage.tokens_this_month,
            "conversationsThisMonth": usage.conversations_this_month,
            "limits": {
                "requestsPerMonth": limits.as_ref().map(|row| row.requests_per_month).unwrap_or(0),
                "tokensPerMonth": limits.as_ref().map(|row| row.tokens_per_month).unwrap_or(0),
                "conversationsPerMonth": limits.as_ref().map(|row| row.conversations_per_month).unwrap_or(0),
            }
        }
    })))
}
