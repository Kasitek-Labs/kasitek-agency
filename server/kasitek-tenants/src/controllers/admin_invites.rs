use actix_web::{web, HttpRequest, HttpResponse};
use bcrypt::{hash, DEFAULT_COST};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::{error::AppError, middleware};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssuedAdminInvite {
    pub activation_url: Option<String>,
    pub portal_url: Option<String>,
    pub email_sent: bool,
    pub expires_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct AdminInviteQuery {
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct AcceptAdminInviteRequest {
    pub token: String,
    pub password: String,
    pub name: Option<String>,
}

#[derive(Debug, FromRow)]
struct AdminInviteRow {
    invite_id: String,
    tenant_user_id: String,
    email: String,
    name: Option<String>,
    workspace_id: String,
    workspace_display_name: String,
    workspace_slug: String,
    expires_at: chrono::DateTime<Utc>,
    accepted_at: Option<chrono::DateTime<Utc>>,
    revoked_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AdminInvitePayload {
    email: String,
    name: Option<String>,
    workspace_display_name: String,
    workspace_slug: String,
    is_valid: bool,
    expires_at: chrono::DateTime<Utc>,
}

async fn load_admin_invite(
    pool: &sqlx::PgPool,
    token: &str,
) -> Result<Option<AdminInviteRow>, AppError> {
    let token_hash = crypto_utils::sha256_hex(token);

    sqlx::query_as::<_, AdminInviteRow>(
        r#"
        SELECT
            i.id AS invite_id,
            i.tenant_user_id,
            i.email,
            u.name,
            i.workspace_id,
            w.display_name AS workspace_display_name,
            w.slug AS workspace_slug,
            i.expires_at AS expires_at,
            i.accepted_at AS accepted_at,
            i.revoked_at AS revoked_at
        FROM tenant_user_invites i
        JOIN tenant_users u ON u.id = i.tenant_user_id
        JOIN workspaces w ON w.id = i.workspace_id
        WHERE i.token_hash = $1
        LIMIT 1
        "#,
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
    .map_err(AppError::from)
}

pub async fn issue_admin_invite(
    pool: &sqlx::PgPool,
    config: &crate::config::AppConfig,
    workspace_id: &str,
    workspace_slug: &str,
    workspace_name: &str,
    tenant_user_id: &str,
    email: &str,
) -> Result<IssuedAdminInvite, AppError> {
    sqlx::query(
        r#"
        UPDATE tenant_user_invites
        SET revoked_at = NOW(), updated_at = NOW()
        WHERE tenant_user_id = $1
          AND accepted_at IS NULL
          AND revoked_at IS NULL
        "#,
    )
    .bind(tenant_user_id)
    .execute(pool)
    .await?;

    let invite_id = cuid2::create_id();
    let invite_token = crypto_utils::generate_token(48);
    let invite_token_hash = crypto_utils::sha256_hex(&invite_token);
    let expires_at = Utc::now() + chrono::Duration::hours(i64::from(config.invite_ttl_hours));
    let portal_url = crate::mail::workspace_portal_url(config, workspace_slug);
    let activation_url = portal_url
        .as_ref()
        .map(|url| format!("{}/setup?token={}", url.trim_end_matches('/'), invite_token));

    sqlx::query(
        r#"
        INSERT INTO tenant_user_invites (
            id,
            workspace_id,
            tenant_user_id,
            email,
            token_hash,
            expires_at,
            created_at,
            updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())
        "#,
    )
    .bind(&invite_id)
    .bind(workspace_id)
    .bind(tenant_user_id)
    .bind(email)
    .bind(invite_token_hash)
    .bind(expires_at)
    .execute(pool)
    .await?;

    let email_sent = if let Some(url) = activation_url.as_deref() {
        crate::mail::send_tenant_admin_invite_email(
            config,
            email,
            workspace_name,
            url,
            portal_url.as_deref(),
            expires_at,
        )
        .await?
    } else {
        false
    };

    Ok(IssuedAdminInvite {
        activation_url,
        portal_url,
        email_sent,
        expires_at,
    })
}

pub async fn get_admin_invite(
    req: HttpRequest,
    query: web::Query<AdminInviteQuery>,
) -> Result<HttpResponse, AppError> {
    let pool = middleware::db_pool(&req)?;
    let token = query.token.trim();
    if token.is_empty() {
        return Err(AppError::BadRequest("token is required".to_string()));
    }

    let invite = load_admin_invite(pool, token)
        .await?
        .ok_or_else(|| AppError::NotFound("invite not found".to_string()))?;

    let is_valid = invite.accepted_at.is_none()
        && invite.revoked_at.is_none()
        && invite.expires_at > Utc::now();

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "invite": AdminInvitePayload {
            email: invite.email,
            name: invite.name,
            workspace_display_name: invite.workspace_display_name,
            workspace_slug: invite.workspace_slug,
            is_valid,
            expires_at: invite.expires_at,
        }
    })))
}

pub async fn accept_admin_invite(
    req: HttpRequest,
    body: web::Json<AcceptAdminInviteRequest>,
) -> Result<HttpResponse, AppError> {
    let pool = middleware::db_pool(&req)?;
    let config = middleware::app_config(&req)?;
    let token = body.token.trim();
    let password = body.password.trim();

    if token.is_empty() || password.is_empty() {
        return Err(AppError::BadRequest(
            "token and password are required".to_string(),
        ));
    }

    if password.len() < 8 {
        return Err(AppError::BadRequest(
            "password must be at least 8 characters".to_string(),
        ));
    }

    let invite = load_admin_invite(pool, token)
        .await?
        .ok_or_else(|| AppError::NotFound("invite not found".to_string()))?;

    if invite.accepted_at.is_some()
        || invite.revoked_at.is_some()
        || invite.expires_at <= Utc::now()
    {
        return Err(AppError::BadRequest(
            "This invite is no longer valid".to_string(),
        ));
    }

    let password_hash = hash(password, DEFAULT_COST)
        .map_err(|e| AppError::Internal(format!("password hash failed: {e}")))?;

    let mut tx = pool.begin().await?;
    sqlx::query(
        r#"
        UPDATE tenant_users
        SET password_hash = $1,
            name = COALESCE($2, name),
            status = 'active',
            updated_at = NOW()
        WHERE id = $3
        "#,
    )
    .bind(&password_hash)
    .bind(
        body.name
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty()),
    )
    .bind(&invite.tenant_user_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE tenant_user_invites
        SET accepted_at = NOW(),
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(&invite.invite_id)
    .execute(&mut *tx)
    .await?;

    let session_token = crypto_utils::generate_token(48);
    let session_token_hash = crypto_utils::sha256_hex(&session_token);
    let session_id = cuid2::create_id();
    let expires_at = Utc::now() + chrono::Duration::hours(i64::from(config.session_ttl_hours));
    let ip_address = req.peer_addr().map(|addr| addr.ip().to_string());
    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|header| header.to_str().ok())
        .map(ToString::to_string);

    sqlx::query(
        r#"
        INSERT INTO tenant_sessions (
            id,
            workspace_id,
            tenant_user_id,
            token_hash,
            expires_at,
            ip_address,
            user_agent,
            last_used_at,
            created_at,
            updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW(), NOW())
        "#,
    )
    .bind(&session_id)
    .bind(&invite.workspace_id)
    .bind(&invite.tenant_user_id)
    .bind(&session_token_hash)
    .bind(expires_at)
    .bind(ip_address)
    .bind(user_agent)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE tenant_users
        SET last_login_at = NOW(), updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(&invite.tenant_user_id)
    .execute(&mut *tx)
    .await?;

    let roles = sqlx::query_scalar::<_, String>(
        r#"
        SELECT role
        FROM tenant_user_roles
        WHERE tenant_user_id = $1
        ORDER BY role ASC
        "#,
    )
    .bind(&invite.tenant_user_id)
    .fetch_all(&mut *tx)
    .await?;

    tx.commit().await?;

    let cookie = crate::controllers::auth::build_session_cookie(config, &session_token);

    Ok(HttpResponse::Ok()
        .cookie(cookie)
        .json(serde_json::json!({
            "user": {
                "id": invite.tenant_user_id,
                "workspaceId": invite.workspace_id,
                "workspaceSlug": invite.workspace_slug,
                "email": invite.email,
                "name": body.name.as_ref().map(|value| value.trim()).filter(|value| !value.is_empty()).or(invite.name.as_deref()),
                "roles": roles,
            }
        })))
}
