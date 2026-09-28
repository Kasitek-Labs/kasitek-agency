use actix_web::{web, HttpRequest, HttpResponse};
use serde::Serialize;
use sqlx::FromRow;

use crate::{error::AppError, middleware};

#[derive(Debug, FromRow, Serialize)]
struct ConversationRow {
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
struct MessageRow {
    id: String,
    sender_role: String,
    content: String,
    metadata: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn list_conversations(req: HttpRequest) -> Result<HttpResponse, AppError> {
    let access = middleware::extract_access_context(&req).await?;
    if access.tenant_user_id.is_some() {
        middleware::require_tenant_roles(
            &access.tenant_user_roles,
            &[
                "owner",
                "admin",
                "account_manager",
                "support",
                "sales_manager",
            ],
        )?;
    }
    let pool = middleware::db_pool(&req)?;

    let conversations = sqlx::query_as::<_, ConversationRow>(
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
        ORDER BY last_message_at DESC, updated_at DESC
        "#,
    )
    .bind(&access.workspace_id)
    .fetch_all(pool)
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "workspaceId": access.workspace_id,
        "conversations": conversations
    })))
}

pub async fn get_conversation(
    req: HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let access = middleware::extract_access_context(&req).await?;
    if access.tenant_user_id.is_some() {
        middleware::require_tenant_roles(
            &access.tenant_user_roles,
            &[
                "owner",
                "admin",
                "account_manager",
                "support",
                "sales_manager",
            ],
        )?;
    }
    let pool = middleware::db_pool(&req)?;
    let id = path.into_inner();

    let conversation = sqlx::query_as::<_, ConversationRow>(
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
        WHERE workspace_id = $1 AND id = $2
        LIMIT 1
        "#,
    )
    .bind(&access.workspace_id)
    .bind(&id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("conversation not found".to_string()))?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "workspaceId": access.workspace_id,
        "conversation": conversation
    })))
}

pub async fn list_messages(
    req: HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let access = middleware::extract_access_context(&req).await?;
    if access.tenant_user_id.is_some() {
        middleware::require_tenant_roles(
            &access.tenant_user_roles,
            &[
                "owner",
                "admin",
                "account_manager",
                "support",
                "sales_manager",
            ],
        )?;
    }
    let pool = middleware::db_pool(&req)?;
    let conversation_id = path.into_inner();

    let exists = sqlx::query_scalar::<_, String>(
        r#"
        SELECT id
        FROM conversations
        WHERE workspace_id = $1 AND id = $2
        LIMIT 1
        "#,
    )
    .bind(&access.workspace_id)
    .bind(&conversation_id)
    .fetch_optional(pool)
    .await?;

    if exists.is_none() {
        return Err(AppError::NotFound("conversation not found".to_string()));
    }

    let messages = sqlx::query_as::<_, MessageRow>(
        r#"
        SELECT
            id,
            sender_role,
            content,
            metadata,
            created_at
        FROM messages
        WHERE workspace_id = $1 AND conversation_id = $2
        ORDER BY created_at ASC
        "#,
    )
    .bind(&access.workspace_id)
    .bind(&conversation_id)
    .fetch_all(pool)
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "workspaceId": access.workspace_id,
        "conversationId": conversation_id,
        "messages": messages
    })))
}
