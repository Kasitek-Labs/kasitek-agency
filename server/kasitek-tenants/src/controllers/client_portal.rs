use actix_web::{web, HttpRequest, HttpResponse};
use serde::Serialize;
use sqlx::FromRow;

use crate::{error::AppError, middleware};

#[derive(Debug, FromRow)]
struct ClientOverviewRow {
    total_users: i64,
    total_conversations: i64,
    unread_or_open_conversations: i64,
    total_messages: i64,
}

#[derive(Debug, FromRow, Serialize)]
struct ClientConversationRow {
    id: String,
    external_id: Option<String>,
    channel: String,
    customer_identifier: String,
    customer_name: Option<String>,
    status: String,
    metadata: serde_json::Value,
    last_message_at: chrono::DateTime<chrono::Utc>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, FromRow, Serialize)]
struct ClientMessageRow {
    id: String,
    sender_role: String,
    content: String,
    metadata: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
}

async fn ensure_client_conversation_access(
    pool: &sqlx::PgPool,
    workspace_id: &str,
    tenant_client_id: &str,
    conversation_id: &str,
) -> Result<(), AppError> {
    middleware::require_client_scoped_conversation(
        pool,
        workspace_id,
        tenant_client_id,
        conversation_id,
    )
    .await
}

pub async fn get_overview(req: HttpRequest) -> Result<HttpResponse, AppError> {
    let session = middleware::extract_tenant_client_user_session(&req).await?;
    let pool = middleware::db_pool(&req)?;

    let overview = sqlx::query_as::<_, ClientOverviewRow>(
        r#"
        SELECT
            (
                SELECT COUNT(*)
                FROM tenant_client_users
                WHERE workspace_id = $1 AND tenant_client_id = $2 AND status = 'active'
            ) AS total_users,
            (
                SELECT COUNT(*)
                FROM conversations
                WHERE workspace_id = $1 AND tenant_client_id = $2
            ) AS total_conversations,
            (
                SELECT COUNT(*)
                FROM conversations
                WHERE workspace_id = $1
                  AND tenant_client_id = $2
                  AND status IN ('active', 'open')
            ) AS unread_or_open_conversations,
            (
                SELECT COUNT(*)
                FROM messages m
                JOIN conversations c ON c.id = m.conversation_id
                WHERE m.workspace_id = $1 AND c.tenant_client_id = $2
            ) AS total_messages
        "#,
    )
    .bind(&session.workspace_id)
    .bind(&session.tenant_client_id)
    .fetch_one(pool)
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "overview": {
            "workspaceId": session.workspace_id,
            "workspaceSlug": session.workspace_slug,
            "tenantClientId": session.tenant_client_id,
            "tenantClientName": session.tenant_client_name,
            "totalUsers": overview.total_users,
            "totalConversations": overview.total_conversations,
            "openConversations": overview.unread_or_open_conversations,
            "totalMessages": overview.total_messages,
        }
    })))
}

pub async fn list_conversations(req: HttpRequest) -> Result<HttpResponse, AppError> {
    let session = middleware::extract_tenant_client_user_session(&req).await?;
    let pool = middleware::db_pool(&req)?;

    let conversations = sqlx::query_as::<_, ClientConversationRow>(
        r#"
        SELECT
            id,
            external_id,
            channel,
            customer_identifier,
            customer_name,
            status,
            metadata,
            last_message_at,
            created_at,
            updated_at
        FROM conversations
        WHERE workspace_id = $1
          AND tenant_client_id = $2
        ORDER BY last_message_at DESC, updated_at DESC
        "#,
    )
    .bind(&session.workspace_id)
    .bind(&session.tenant_client_id)
    .fetch_all(pool)
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "conversations": conversations
    })))
}

pub async fn get_conversation(
    req: HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let session = middleware::extract_tenant_client_user_session(&req).await?;
    let pool = middleware::db_pool(&req)?;
    let conversation_id = path.into_inner();

    ensure_client_conversation_access(
        pool,
        &session.workspace_id,
        &session.tenant_client_id,
        &conversation_id,
    )
    .await?;

    let conversation = sqlx::query_as::<_, ClientConversationRow>(
        r#"
        SELECT
            id,
            external_id,
            channel,
            customer_identifier,
            customer_name,
            status,
            metadata,
            last_message_at,
            created_at,
            updated_at
        FROM conversations
        WHERE workspace_id = $1
          AND tenant_client_id = $2
          AND id = $3
        LIMIT 1
        "#,
    )
    .bind(&session.workspace_id)
    .bind(&session.tenant_client_id)
    .bind(&conversation_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("conversation not found".to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "conversation": conversation
    })))
}

pub async fn list_messages(
    req: HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let session = middleware::extract_tenant_client_user_session(&req).await?;
    let pool = middleware::db_pool(&req)?;
    let conversation_id = path.into_inner();

    ensure_client_conversation_access(
        pool,
        &session.workspace_id,
        &session.tenant_client_id,
        &conversation_id,
    )
    .await?;

    let messages = sqlx::query_as::<_, ClientMessageRow>(
        r#"
        SELECT
            id,
            sender_role,
            content,
            metadata,
            created_at
        FROM messages
        WHERE workspace_id = $1
          AND conversation_id = $2
        ORDER BY created_at ASC
        "#,
    )
    .bind(&session.workspace_id)
    .bind(&conversation_id)
    .fetch_all(pool)
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "conversationId": conversation_id,
        "messages": messages
    })))
}
