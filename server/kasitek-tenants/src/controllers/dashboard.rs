use actix_web::{HttpRequest, HttpResponse};
use serde::Serialize;
use sqlx::FromRow;

use crate::{error::AppError, middleware};

#[derive(Debug, FromRow)]
struct DashboardSummaryRow {
    total_clients: i64,
    active_agents: i64,
    total_conversations: i64,
    total_leads: i64,
    requests_this_month: i64,
}

#[derive(Debug, FromRow, Serialize)]
struct DashboardActivityRow {
    id: String,
    title: String,
    description: String,
    timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
struct DashboardService {
    id: String,
    name: String,
    is_active: bool,
}

pub async fn get_summary(req: HttpRequest) -> Result<HttpResponse, AppError> {
    let session = middleware::extract_tenant_user_session(&req).await?;
    middleware::require_tenant_roles(
        &session.roles,
        &[
            "owner",
            "admin",
            "account_manager",
            "viewer",
            "support",
            "sales_manager",
            "content_manager",
            "automation_manager",
        ],
    )?;
    let pool = middleware::db_pool(&req)?;

    let summary = sqlx::query_as::<_, DashboardSummaryRow>(
        r#"
        SELECT
            (SELECT COUNT(*) FROM tenant_clients WHERE workspace_id = $1 AND status = 'active') AS total_clients,
            (SELECT COUNT(*) FROM workspace_agents WHERE workspace_id = $1 AND status = 'active') AS active_agents,
            (SELECT COUNT(*) FROM conversations WHERE workspace_id = $1) AS total_conversations,
            (SELECT COUNT(*) FROM tenant_leads WHERE workspace_id = $1) AS total_leads,
            (
                SELECT COALESCE(SUM(quantity), 0)::bigint
                FROM workspace_usage_events
                WHERE workspace_id = $1
                  AND event_type = 'request'
                  AND created_at >= date_trunc('month', NOW())
            ) AS requests_this_month
        "#,
    )
    .bind(&session.workspace_id)
    .fetch_one(pool)
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "summary": {
            "workspaceId": session.workspace_id,
            "workspaceSlug": session.workspace_slug,
            "totalClients": summary.total_clients,
            "activeAgents": summary.active_agents,
            "totalConversations": summary.total_conversations,
            "totalLeads": summary.total_leads,
            "requestsThisMonth": summary.requests_this_month,
        }
    })))
}

pub async fn get_activity(req: HttpRequest) -> Result<HttpResponse, AppError> {
    let session = middleware::extract_tenant_user_session(&req).await?;
    middleware::require_tenant_roles(
        &session.roles,
        &[
            "owner",
            "admin",
            "account_manager",
            "viewer",
            "support",
            "sales_manager",
            "content_manager",
            "automation_manager",
        ],
    )?;
    let pool = middleware::db_pool(&req)?;

    let activity = sqlx::query_as::<_, DashboardActivityRow>(
        r#"
        SELECT id, title, description, timestamp
        FROM (
            SELECT
                tc.id AS id,
                'Client added' AS title,
                'Added client ' || tc.name AS description,
                tc.created_at AS timestamp
            FROM tenant_clients tc
            WHERE tc.workspace_id = $1

            UNION ALL

            SELECT
                i.id AS id,
                'Client invite sent' AS title,
                'Sent invite to ' || i.email AS description,
                i.created_at AS timestamp
            FROM tenant_client_invites i
            WHERE i.workspace_id = $1

            UNION ALL

            SELECT
                l.id AS id,
                'Lead added' AS title,
                l.name || ' for ' || COALESCE(l.company, c.name) AS description,
                l.created_at AS timestamp
            FROM tenant_leads l
            JOIN tenant_clients c ON c.id = l.tenant_client_id
            WHERE l.workspace_id = $1

            UNION ALL

            SELECT
                c.id AS id,
                'Conversation updated' AS title,
                COALESCE(c.customer_name, c.customer_identifier) || ' on ' || c.channel AS description,
                c.last_message_at AS timestamp
            FROM conversations c
            WHERE c.workspace_id = $1

            UNION ALL

            SELECT
                a.id AS id,
                'Agent updated' AS title,
                a.name || ' is ' || a.status AS description,
                a.updated_at AS timestamp
            FROM workspace_agents a
            WHERE a.workspace_id = $1
        ) activity
        ORDER BY timestamp DESC
        LIMIT 12
        "#,
    )
    .bind(&session.workspace_id)
    .fetch_all(pool)
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "activity": activity
    })))
}

pub async fn list_services(req: HttpRequest) -> Result<HttpResponse, AppError> {
    let session = middleware::extract_tenant_user_session(&req).await?;
    middleware::require_tenant_roles(
        &session.roles,
        &[
            "owner",
            "admin",
            "account_manager",
            "viewer",
            "support",
            "sales_manager",
            "content_manager",
            "automation_manager",
        ],
    )?;
    let pool = middleware::db_pool(&req)?;

    let active_agents = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM workspace_agents
        WHERE workspace_id = $1 AND status = 'active'
        "#,
    )
    .bind(&session.workspace_id)
    .fetch_one(pool)
    .await?;

    let client_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM tenant_clients
        WHERE workspace_id = $1 AND status = 'active'
        "#,
    )
    .bind(&session.workspace_id)
    .fetch_one(pool)
    .await?;

    let conversation_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM conversations
        WHERE workspace_id = $1
        "#,
    )
    .bind(&session.workspace_id)
    .fetch_one(pool)
    .await?;

    let lead_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM tenant_leads
        WHERE workspace_id = $1
        "#,
    )
    .bind(&session.workspace_id)
    .fetch_one(pool)
    .await?;

    let services = vec![
        DashboardService {
            id: "workspace".to_string(),
            name: "Workspace Access".to_string(),
            is_active: true,
        },
        DashboardService {
            id: "clients".to_string(),
            name: "Client Management".to_string(),
            is_active: client_count > 0,
        },
        DashboardService {
            id: "agents".to_string(),
            name: "Automation Agents".to_string(),
            is_active: active_agents > 0,
        },
        DashboardService {
            id: "conversations".to_string(),
            name: "Conversation Hub".to_string(),
            is_active: conversation_count > 0,
        },
        DashboardService {
            id: "leads".to_string(),
            name: "Lead Pipeline".to_string(),
            is_active: lead_count > 0,
        },
    ];

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "services": services
    })))
}
