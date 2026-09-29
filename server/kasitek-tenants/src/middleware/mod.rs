use crate::types::{
    TenantAccessContext, TenantClientUserSessionContext, TenantResolvedWorkspace,
    TenantUserSessionContext, TenantWorkspaceContext,
};
use actix_web::HttpRequest;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{FromRow, PgPool};

use crate::{config::AppConfig, error::AppError};

#[derive(Debug, FromRow)]
struct WorkspaceApiKeyRow {
    workspace_id: String,
    workspace_slug: String,
    workspace_status: String,
    scopes: Value,
    key_id: String,
    revoked_at: Option<DateTime<Utc>>,
    expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, FromRow)]
struct ResolvedWorkspaceRow {
    workspace_id: String,
    workspace_slug: String,
    display_name: String,
    status: String,
}

#[derive(Debug, FromRow)]
struct TenantSessionRow {
    workspace_id: String,
    workspace_slug: String,
    tenant_user_id: String,
    email: String,
    name: Option<String>,
    user_status: String,
    session_id: String,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct TenantClientSessionRow {
    workspace_id: String,
    workspace_slug: String,
    tenant_client_id: String,
    tenant_client_name: String,
    tenant_client_user_id: String,
    email: String,
    name: Option<String>,
    user_status: String,
    session_id: String,
    expires_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
pub struct TenantClientWorkspaceRow {
    pub name: String,
    pub status: String,
}

pub fn db_pool(req: &HttpRequest) -> Result<&PgPool, AppError> {
    req.app_data::<actix_web::web::Data<PgPool>>()
        .map(|data| data.get_ref())
        .ok_or_else(|| AppError::Internal("database pool is not configured".to_string()))
}

pub fn app_config(req: &HttpRequest) -> Result<&AppConfig, AppError> {
    req.app_data::<actix_web::web::Data<AppConfig>>()
        .map(|data| data.get_ref())
        .ok_or_else(|| AppError::Internal("tenant config is not configured".to_string()))
}

pub fn tenant_user_has_any_role(roles: &[String], allowed_roles: &[&str]) -> bool {
    allowed_roles
        .iter()
        .any(|allowed_role| roles.iter().any(|role| role == allowed_role))
}

pub fn require_tenant_roles(roles: &[String], allowed_roles: &[&str]) -> Result<(), AppError> {
    if tenant_user_has_any_role(roles, allowed_roles) {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

pub async fn require_active_tenant_client(
    pool: &PgPool,
    workspace_id: &str,
    tenant_client_id: &str,
) -> Result<TenantClientWorkspaceRow, AppError> {
    let client = sqlx::query_as::<_, TenantClientWorkspaceRow>(
        r#"
        SELECT name, status
        FROM tenant_clients
        WHERE workspace_id = $1
          AND id = $2
        LIMIT 1
        "#,
    )
    .bind(workspace_id)
    .bind(tenant_client_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("tenant client not found".to_string()))?;

    if client.status != "active" {
        return Err(AppError::Forbidden);
    }

    Ok(client)
}

pub async fn require_client_scoped_conversation(
    pool: &PgPool,
    workspace_id: &str,
    tenant_client_id: &str,
    conversation_id: &str,
) -> Result<(), AppError> {
    let exists = sqlx::query_scalar::<_, String>(
        r#"
        SELECT id
        FROM conversations
        WHERE workspace_id = $1
          AND tenant_client_id = $2
          AND id = $3
        LIMIT 1
        "#,
    )
    .bind(workspace_id)
    .bind(tenant_client_id)
    .bind(conversation_id)
    .fetch_optional(pool)
    .await?;

    if exists.is_none() {
        return Err(AppError::NotFound("conversation not found".to_string()));
    }

    Ok(())
}

pub fn extract_bearer_token(req: &HttpRequest) -> Result<String, AppError> {
    req.headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .ok_or(AppError::Unauthorized)
}

pub fn extract_tenant_session_token(req: &HttpRequest) -> Result<String, AppError> {
    let cookie_name = &app_config(req)?.session_cookie_name;
    req.cookie(cookie_name)
        .map(|cookie| cookie.value().trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or(AppError::Unauthorized)
}

pub fn extract_client_session_token(req: &HttpRequest) -> Result<String, AppError> {
    let cookie_name = &app_config(req)?.client_session_cookie_name;
    req.cookie(cookie_name)
        .map(|cookie| cookie.value().trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or(AppError::Unauthorized)
}

fn request_host(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("x-kasitek-tenant-host")
        .and_then(|header| header.to_str().ok())
        .or_else(|| {
            req.headers()
                .get("x-forwarded-host")
                .and_then(|header| header.to_str().ok())
        })
        .or_else(|| {
            req.headers()
                .get("host")
                .and_then(|header| header.to_str().ok())
        })
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.split(',').next().unwrap_or(value).trim().to_string())
}

fn normalize_host(host: &str) -> String {
    host.trim()
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .split(':')
        .next()
        .unwrap_or(host)
        .trim()
        .to_ascii_lowercase()
}

fn workspace_slug_from_platform_host(host: &str, platform_base_domain: &str) -> Option<String> {
    let host = host.trim().trim_matches('.').to_ascii_lowercase();
    let base = platform_base_domain
        .trim()
        .trim_matches('.')
        .to_ascii_lowercase();

    if host.is_empty() || base.is_empty() {
        return None;
    }

    if host == base {
        return None;
    }

    let suffix = format!(".{base}");
    let slug = host.strip_suffix(&suffix)?.trim();
    if slug.is_empty() || slug.contains('.') {
        return None;
    }

    Some(slug.to_string())
}

pub async fn resolve_workspace_from_request(
    req: &HttpRequest,
) -> Result<TenantResolvedWorkspace, AppError> {
    let pool = db_pool(req)?;
    let config = app_config(req)?;

    if let Some(host) = request_host(req) {
        let host = normalize_host(&host);
        if !host.is_empty() {
            let record = sqlx::query_as::<_, ResolvedWorkspaceRow>(
                r#"
                SELECT
                    w.id AS workspace_id,
                    w.slug AS workspace_slug,
                    w.display_name AS display_name,
                    w.status AS status
                FROM workspace_domains d
                JOIN workspaces w ON w.id = d.workspace_id
                WHERE lower(d.domain) = lower($1)
                LIMIT 1
                "#,
            )
            .bind(&host)
            .fetch_optional(pool)
            .await?;

            if let Some(record) = record {
                if record.status != "active" {
                    return Err(AppError::Forbidden);
                }

                return Ok(TenantResolvedWorkspace {
                    workspace_id: record.workspace_id,
                    workspace_slug: record.workspace_slug,
                    display_name: record.display_name,
                });
            }

            if let Some(platform_base_domain) = &config.platform_base_domain {
                if let Some(workspace_slug) =
                    workspace_slug_from_platform_host(&host, platform_base_domain)
                {
                    return resolve_workspace_by_slug(pool, &workspace_slug).await;
                }
            }
        }
    }

    if let Some(workspace_slug) = &config.default_workspace_slug {
        return resolve_workspace_by_slug(pool, workspace_slug).await;
    }

    Err(AppError::NotFound(
        "workspace could not be resolved from request host".to_string(),
    ))
}

async fn resolve_workspace_by_slug(
    pool: &PgPool,
    workspace_slug: &str,
) -> Result<TenantResolvedWorkspace, AppError> {
    let record = sqlx::query_as::<_, ResolvedWorkspaceRow>(
        r#"
        SELECT
            id AS workspace_id,
            slug AS workspace_slug,
            display_name,
            status
        FROM workspaces
        WHERE slug = $1
        LIMIT 1
        "#,
    )
    .bind(workspace_slug)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("workspace not found".to_string()))?;

    if record.status != "active" {
        return Err(AppError::Forbidden);
    }

    Ok(TenantResolvedWorkspace {
        workspace_id: record.workspace_id,
        workspace_slug: record.workspace_slug,
        display_name: record.display_name,
    })
}

pub async fn extract_workspace_context_from_api_key(
    req: &HttpRequest,
) -> Result<TenantWorkspaceContext, AppError> {
    let token = extract_bearer_token(req)?;
    let token_hash = crypto_utils::sha256_hex(&token);
    let pool = db_pool(req)?;

    let record = sqlx::query_as::<_, WorkspaceApiKeyRow>(
        r#"
        SELECT
            w.id AS workspace_id,
            w.slug AS workspace_slug,
            w.status AS workspace_status,
            k.scopes AS scopes,
            k.id AS key_id,
            k.revoked_at AS revoked_at,
            k.expires_at AS expires_at
        FROM workspace_api_keys k
        JOIN workspaces w ON w.id = k.workspace_id
        WHERE k.key_hash = $1
        LIMIT 1
        "#,
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    if record.workspace_status != "active" {
        return Err(AppError::Forbidden);
    }

    if record.revoked_at.is_some() {
        return Err(AppError::Unauthorized);
    }

    if let Some(expires_at) = record.expires_at {
        if expires_at <= Utc::now() {
            return Err(AppError::Unauthorized);
        }
    }

    sqlx::query(
        r#"
        UPDATE workspace_api_keys
        SET last_used_at = NOW(), updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(&record.key_id)
    .execute(pool)
    .await?;

    Ok(TenantWorkspaceContext {
        workspace_id: record.workspace_id,
        workspace_slug: record.workspace_slug,
        scopes: parse_scopes(record.scopes),
    })
}

pub async fn extract_tenant_user_session(
    req: &HttpRequest,
) -> Result<TenantUserSessionContext, AppError> {
    let session_token = extract_tenant_session_token(req)?;
    let token_hash = crypto_utils::sha256_hex(&session_token);
    let workspace = resolve_workspace_from_request(req).await?;
    let pool = db_pool(req)?;

    let record = sqlx::query_as::<_, TenantSessionRow>(
        r#"
        SELECT
            s.workspace_id,
            w.slug AS workspace_slug,
            s.tenant_user_id,
            u.email,
            u.name,
            u.status AS user_status,
            s.id AS session_id,
            s.expires_at AS expires_at
        FROM tenant_sessions s
        JOIN tenant_users u ON u.id = s.tenant_user_id
        JOIN workspaces w ON w.id = s.workspace_id
        WHERE s.token_hash = $1
          AND s.workspace_id = $2
        LIMIT 1
        "#,
    )
    .bind(token_hash)
    .bind(&workspace.workspace_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    if record.user_status != "active" || record.expires_at <= Utc::now() {
        return Err(AppError::Unauthorized);
    }

    sqlx::query(
        r#"
        UPDATE tenant_sessions
        SET last_used_at = NOW(), updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(&record.session_id)
    .execute(pool)
    .await?;

    let roles = sqlx::query_scalar::<_, String>(
        r#"
        SELECT role
        FROM tenant_user_roles
        WHERE tenant_user_id = $1
        ORDER BY role ASC
        "#,
    )
    .bind(&record.tenant_user_id)
    .fetch_all(pool)
    .await?;

    Ok(TenantUserSessionContext {
        workspace_id: record.workspace_id,
        workspace_slug: record.workspace_slug,
        tenant_user_id: record.tenant_user_id,
        email: record.email,
        name: record.name,
        roles,
    })
}

pub async fn extract_tenant_client_user_session(
    req: &HttpRequest,
) -> Result<TenantClientUserSessionContext, AppError> {
    let session_token = extract_client_session_token(req)?;
    let token_hash = crypto_utils::sha256_hex(&session_token);
    let workspace = resolve_workspace_from_request(req).await?;
    let pool = db_pool(req)?;

    let record = sqlx::query_as::<_, TenantClientSessionRow>(
        r#"
        SELECT
            s.workspace_id,
            w.slug AS workspace_slug,
            c.id AS tenant_client_id,
            c.name AS tenant_client_name,
            s.tenant_client_user_id,
            u.email,
            u.name,
            u.status AS user_status,
            s.id AS session_id,
            s.expires_at AS expires_at
        FROM tenant_client_sessions s
        JOIN tenant_client_users u ON u.id = s.tenant_client_user_id
        JOIN tenant_clients c ON c.id = s.tenant_client_id
        JOIN workspaces w ON w.id = s.workspace_id
        WHERE s.token_hash = $1
          AND s.workspace_id = $2
        LIMIT 1
        "#,
    )
    .bind(token_hash)
    .bind(&workspace.workspace_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    if record.user_status != "active" || record.expires_at <= Utc::now() {
        return Err(AppError::Unauthorized);
    }

    sqlx::query(
        r#"
        UPDATE tenant_client_sessions
        SET last_used_at = NOW(), updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(&record.session_id)
    .execute(pool)
    .await?;

    let roles = sqlx::query_scalar::<_, String>(
        r#"
        SELECT role
        FROM tenant_client_user_roles
        WHERE tenant_client_user_id = $1
        ORDER BY role ASC
        "#,
    )
    .bind(&record.tenant_client_user_id)
    .fetch_all(pool)
    .await?;

    Ok(TenantClientUserSessionContext {
        workspace_id: record.workspace_id,
        workspace_slug: record.workspace_slug,
        tenant_client_id: record.tenant_client_id,
        tenant_client_name: record.tenant_client_name,
        tenant_client_user_id: record.tenant_client_user_id,
        email: record.email,
        name: record.name,
        roles,
    })
}

pub async fn extract_access_context(req: &HttpRequest) -> Result<TenantAccessContext, AppError> {
    if req.cookie(&app_config(req)?.session_cookie_name).is_some() {
        let session = extract_tenant_user_session(req).await?;
        return Ok(TenantAccessContext {
            workspace_id: session.workspace_id,
            workspace_slug: session.workspace_slug,
            scopes: vec!["tenant:portal".to_string()],
            tenant_user_id: Some(session.tenant_user_id),
            tenant_user_roles: session.roles,
        });
    }

    let api_key = extract_workspace_context_from_api_key(req).await?;
    Ok(TenantAccessContext {
        workspace_id: api_key.workspace_id,
        workspace_slug: api_key.workspace_slug,
        scopes: api_key.scopes,
        tenant_user_id: None,
        tenant_user_roles: Vec::new(),
    })
}

fn parse_scopes(value: Value) -> Vec<String> {
    match value {
        Value::Array(items) => items
            .into_iter()
            .filter_map(|item| item.as_str().map(ToString::to_string))
            .collect(),
        _ => Vec::new(),
    }
}
