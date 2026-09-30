use actix_web::{
    cookie::{
        time::{Duration, OffsetDateTime},
        Cookie, SameSite,
    },
    web, HttpRequest, HttpResponse,
};
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::{
    config::AppConfig, controllers::auth::clear_session_cookie, error::AppError, middleware,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IssuedClientInvite {
    pub invite_id: String,
    pub activation_url: Option<String>,
    pub portal_url: Option<String>,
    pub email_sent: bool,
    pub expires_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct AcceptInviteRequest {
    pub token: String,
    pub password: String,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ClientLoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, FromRow)]
struct ClientUserRow {
    id: String,
    workspace_id: String,
    tenant_client_id: String,
    tenant_client_name: String,
    email: String,
    password_hash: Option<String>,
    name: Option<String>,
    status: String,
}

#[derive(Debug, FromRow)]
struct InvitePreviewRow {
    id: String,
    workspace_id: String,
    workspace_slug: String,
    workspace_display_name: String,
    tenant_client_id: String,
    tenant_client_name: String,
    tenant_client_user_id: String,
    invite_email: String,
    user_name: Option<String>,
    revoked_at: Option<chrono::DateTime<chrono::Utc>>,
    accepted_at: Option<chrono::DateTime<chrono::Utc>>,
    expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct ClientUserPayload {
    pub id: String,
    #[serde(rename = "workspaceId")]
    pub workspace_id: String,
    #[serde(rename = "workspaceSlug")]
    pub workspace_slug: String,
    #[serde(rename = "tenantClientId")]
    pub tenant_client_id: String,
    #[serde(rename = "tenantClientName")]
    pub tenant_client_name: String,
    pub email: String,
    pub name: Option<String>,
    pub roles: Vec<String>,
}

fn same_site(config: &AppConfig) -> SameSite {
    match config.session_cookie_same_site {
        config::CookieSameSitePolicy::Lax => SameSite::Lax,
        config::CookieSameSitePolicy::Strict => SameSite::Strict,
        config::CookieSameSitePolicy::None => SameSite::None,
    }
}

pub(crate) fn build_client_session_cookie(config: &AppConfig, value: &str) -> Cookie<'static> {
    let mut cookie = Cookie::build(config.client_session_cookie_name.clone(), value.to_string())
        .path("/")
        .http_only(true)
        .same_site(same_site(config))
        .secure(config.session_cookie_secure)
        .expires(
            OffsetDateTime::now_utc() + Duration::hours(i64::from(config.client_session_ttl_hours)),
        )
        .finish();

    if let Some(domain) = &config.session_cookie_domain {
        cookie.set_domain(domain.clone());
    }

    cookie
}

pub(crate) fn clear_client_session_cookie(config: &AppConfig) -> Cookie<'static> {
    let mut cookie = Cookie::build(config.client_session_cookie_name.clone(), "")
        .path("/")
        .http_only(true)
        .same_site(same_site(config))
        .secure(config.session_cookie_secure)
        .expires(OffsetDateTime::now_utc() - Duration::days(1))
        .finish();

    if let Some(domain) = &config.session_cookie_domain {
        cookie.set_domain(domain.clone());
    }

    cookie
}

async fn find_invite_by_token(
    req: &HttpRequest,
    token: &str,
) -> Result<Option<InvitePreviewRow>, AppError> {
    let workspace = middleware::resolve_workspace_from_request(req).await?;
    let token_hash = crypto_utils::sha256_hex(token);
    let pool = middleware::db_pool(req)?;

    let invite = sqlx::query_as::<_, InvitePreviewRow>(
        r#"
        SELECT
            i.id,
            i.workspace_id,
            w.slug AS workspace_slug,
            w.display_name AS workspace_display_name,
            i.tenant_client_id,
            c.name AS tenant_client_name,
            i.tenant_client_user_id,
            i.email AS invite_email,
            u.name AS user_name,
            i.revoked_at AS revoked_at,
            i.accepted_at AS accepted_at,
            i.expires_at AS expires_at
        FROM tenant_client_invites i
        JOIN tenant_client_users u ON u.id = i.tenant_client_user_id
        JOIN tenant_clients c ON c.id = i.tenant_client_id
        JOIN workspaces w ON w.id = i.workspace_id
        WHERE i.token_hash = $1
          AND i.workspace_id = $2
        LIMIT 1
        "#,
    )
    .bind(token_hash)
    .bind(&workspace.workspace_id)
    .fetch_optional(pool)
    .await?;

    Ok(invite)
}

pub async fn issue_client_invite(
    pool: &sqlx::PgPool,
    config: &AppConfig,
    workspace_id: &str,
    workspace_slug: &str,
    tenant_client_id: &str,
    tenant_client_user_id: &str,
    email: &str,
    invited_by_tenant_user_id: Option<&str>,
) -> Result<IssuedClientInvite, AppError> {
    sqlx::query(
        r#"
        UPDATE tenant_client_invites
        SET revoked_at = NOW(), updated_at = NOW()
        WHERE tenant_client_user_id = $1
          AND accepted_at IS NULL
          AND revoked_at IS NULL
        "#,
    )
    .bind(tenant_client_user_id)
    .execute(pool)
    .await?;

    let invite_id = cuid2::create_id();
    let invite_token = crypto_utils::generate_token(48);
    let invite_token_hash = crypto_utils::sha256_hex(&invite_token);
    let expires_at = Utc::now() + ChronoDuration::hours(i64::from(config.invite_ttl_hours));
    let portal_url = crate::mail::workspace_portal_url(config, workspace_slug);
    let activation_url = portal_url.as_ref().map(|url| {
        format!(
            "{}/activate?token={}",
            url.trim_end_matches('/'),
            invite_token
        )
    });

    sqlx::query(
        r#"
        INSERT INTO tenant_client_invites (
            id,
            workspace_id,
            tenant_client_id,
            tenant_client_user_id,
            email,
            token_hash,
            invited_by_tenant_user_id,
            expires_at,
            created_at,
            updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), NOW())
        "#,
    )
    .bind(&invite_id)
    .bind(workspace_id)
    .bind(tenant_client_id)
    .bind(tenant_client_user_id)
    .bind(email)
    .bind(invite_token_hash)
    .bind(invited_by_tenant_user_id)
    .bind(expires_at)
    .execute(pool)
    .await?;

    let mail_context = sqlx::query_as::<_, InvitePreviewRow>(
        r#"
        SELECT
            i.id,
            i.workspace_id,
            w.slug AS workspace_slug,
            w.display_name AS workspace_display_name,
            i.tenant_client_id,
            c.name AS tenant_client_name,
            i.tenant_client_user_id,
            i.email AS invite_email,
            u.name AS user_name,
            i.revoked_at AS revoked_at,
            i.accepted_at AS accepted_at,
            i.expires_at AS expires_at
        FROM tenant_client_invites i
        JOIN tenant_client_users u ON u.id = i.tenant_client_user_id
        JOIN tenant_clients c ON c.id = i.tenant_client_id
        JOIN workspaces w ON w.id = i.workspace_id
        WHERE i.id = $1
        LIMIT 1
        "#,
    )
    .bind(&invite_id)
    .fetch_one(pool)
    .await?;

    let email_sent = if let Some(url) = activation_url.as_deref() {
        crate::mail::send_client_invite_email(
            pool,
            config,
            workspace_id,
            workspace_slug,
            &mail_context.workspace_display_name,
            email,
            &mail_context.tenant_client_name,
            url,
            portal_url.as_deref(),
            expires_at,
        )
        .await?
    } else {
        false
    };

    Ok(IssuedClientInvite {
        invite_id,
        activation_url,
        portal_url,
        email_sent,
        expires_at,
    })
}

pub(crate) async fn authenticate_client_user(
    req: &HttpRequest,
    email: &str,
    password: &str,
) -> Result<Option<(ClientUserPayload, String)>, AppError> {
    let workspace = middleware::resolve_workspace_from_request(req).await?;
    let pool = middleware::db_pool(req)?;
    let config = middleware::app_config(req)?;

    let user = sqlx::query_as::<_, ClientUserRow>(
        r#"
        SELECT
            u.id,
            u.workspace_id,
            u.tenant_client_id,
            c.name AS tenant_client_name,
            u.email,
            u.password_hash,
            u.name,
            u.status
        FROM tenant_client_users u
        JOIN tenant_clients c ON c.id = u.tenant_client_id
        WHERE u.workspace_id = $1 AND lower(u.email) = lower($2)
        LIMIT 1
        "#,
    )
    .bind(&workspace.workspace_id)
    .bind(email)
    .fetch_optional(pool)
    .await?;

    let Some(user) = user else {
        return Ok(None);
    };

    let Some(stored_hash) = user.password_hash.clone() else {
        return Ok(None);
    };

    if user.status != "active" || !verify(password, &stored_hash)? {
        return Ok(None);
    }

    let session_token = crypto_utils::generate_token(48);
    let session_token_hash = crypto_utils::sha256_hex(&session_token);
    let session_id = cuid2::create_id();
    let expires_at = Utc::now() + ChronoDuration::hours(i64::from(config.client_session_ttl_hours));
    let ip_address = req.peer_addr().map(|addr| addr.ip().to_string());
    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|header| header.to_str().ok())
        .map(ToString::to_string);

    sqlx::query(
        r#"
        INSERT INTO tenant_client_sessions (
            id,
            workspace_id,
            tenant_client_id,
            tenant_client_user_id,
            token_hash,
            expires_at,
            ip_address,
            user_agent,
            last_used_at,
            created_at,
            updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), NOW(), NOW())
        "#,
    )
    .bind(&session_id)
    .bind(&workspace.workspace_id)
    .bind(&user.tenant_client_id)
    .bind(&user.id)
    .bind(&session_token_hash)
    .bind(expires_at)
    .bind(ip_address)
    .bind(user_agent)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        UPDATE tenant_client_users
        SET last_login_at = NOW(), updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(&user.id)
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
    .bind(&user.id)
    .fetch_all(pool)
    .await?;

    Ok(Some((
        ClientUserPayload {
            id: user.id,
            workspace_id: user.workspace_id,
            workspace_slug: workspace.workspace_slug,
            tenant_client_id: user.tenant_client_id,
            tenant_client_name: user.tenant_client_name,
            email: user.email,
            name: user.name,
            roles,
        },
        session_token,
    )))
}

pub async fn get_invite(req: HttpRequest) -> Result<HttpResponse, AppError> {
    let token =
        web::Query::<std::collections::HashMap<String, String>>::from_query(req.query_string())
            .map_err(|_| AppError::BadRequest("invalid invite query".to_string()))?
            .get("token")
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| AppError::BadRequest("token is required".to_string()))?;

    let invite = find_invite_by_token(&req, &token)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let is_valid = invite.revoked_at.is_none()
        && invite.accepted_at.is_none()
        && invite.expires_at > Utc::now();

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "invite": {
            "workspaceId": invite.workspace_id,
            "workspaceSlug": invite.workspace_slug,
            "workspaceDisplayName": invite.workspace_display_name,
            "tenantClientId": invite.tenant_client_id,
            "tenantClientName": invite.tenant_client_name,
            "tenantClientUserId": invite.tenant_client_user_id,
            "email": invite.invite_email,
            "name": invite.user_name,
            "expiresAt": invite.expires_at,
            "isValid": is_valid,
            "acceptedAt": invite.accepted_at,
            "revokedAt": invite.revoked_at,
        }
    })))
}

pub async fn accept_invite(
    req: HttpRequest,
    body: web::Json<AcceptInviteRequest>,
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

    let invite = find_invite_by_token(&req, token)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if invite.revoked_at.is_some()
        || invite.accepted_at.is_some()
        || invite.expires_at <= Utc::now()
    {
        return Err(AppError::Unauthorized);
    }

    let password_hash = hash(password, DEFAULT_COST)?;
    let name = body
        .name
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .or(invite.user_name.clone());

    sqlx::query(
        r#"
        UPDATE tenant_client_users
        SET
            password_hash = $1,
            name = $2,
            status = 'active',
            updated_at = NOW()
        WHERE id = $3
        "#,
    )
    .bind(&password_hash)
    .bind(&name)
    .bind(&invite.tenant_client_user_id)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        UPDATE tenant_client_invites
        SET accepted_at = NOW(), updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(&invite.id)
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
    .bind(&invite.tenant_client_user_id)
    .fetch_all(pool)
    .await?;

    let session_token = crypto_utils::generate_token(48);
    let session_token_hash = crypto_utils::sha256_hex(&session_token);
    let session_id = cuid2::create_id();
    let expires_at = Utc::now() + ChronoDuration::hours(i64::from(config.client_session_ttl_hours));
    let ip_address = req.peer_addr().map(|addr| addr.ip().to_string());
    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|header| header.to_str().ok())
        .map(ToString::to_string);

    sqlx::query(
        r#"
        INSERT INTO tenant_client_sessions (
            id,
            workspace_id,
            tenant_client_id,
            tenant_client_user_id,
            token_hash,
            expires_at,
            ip_address,
            user_agent,
            last_used_at,
            created_at,
            updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), NOW(), NOW())
        "#,
    )
    .bind(&session_id)
    .bind(&invite.workspace_id)
    .bind(&invite.tenant_client_id)
    .bind(&invite.tenant_client_user_id)
    .bind(&session_token_hash)
    .bind(expires_at)
    .bind(ip_address)
    .bind(user_agent)
    .execute(pool)
    .await?;

    Ok(HttpResponse::Ok()
        .cookie(clear_session_cookie(config))
        .cookie(build_client_session_cookie(config, &session_token))
        .json(serde_json::json!({
            "audience": "client",
            "clientUser": {
                "id": invite.tenant_client_user_id,
                "workspaceId": invite.workspace_id,
                "workspaceSlug": invite.workspace_slug,
                "tenantClientId": invite.tenant_client_id,
                "tenantClientName": invite.tenant_client_name,
                "email": invite.invite_email,
                "name": name,
                "roles": roles,
            }
        })))
}

pub async fn login(
    req: HttpRequest,
    body: web::Json<ClientLoginRequest>,
) -> Result<HttpResponse, AppError> {
    let config = middleware::app_config(&req)?;
    let email = body.email.trim().to_ascii_lowercase();
    let password = body.password.trim();

    if email.is_empty() || password.is_empty() {
        return Err(AppError::BadRequest(
            "email and password are required".to_string(),
        ));
    }

    let Some((client_user, session_token)) =
        authenticate_client_user(&req, &email, password).await?
    else {
        return Err(AppError::Unauthorized);
    };

    Ok(HttpResponse::Ok()
        .cookie(build_client_session_cookie(config, &session_token))
        .cookie(clear_session_cookie(config))
        .json(serde_json::json!({
            "clientUser": client_user
        })))
}

pub async fn logout(req: HttpRequest) -> Result<HttpResponse, AppError> {
    let pool = middleware::db_pool(&req)?;
    let config = middleware::app_config(&req)?;

    if let Ok(session_token) = middleware::extract_client_session_token(&req) {
        let token_hash = crypto_utils::sha256_hex(&session_token);
        sqlx::query("DELETE FROM tenant_client_sessions WHERE token_hash = $1")
            .bind(token_hash)
            .execute(pool)
            .await?;
    }

    Ok(HttpResponse::Ok()
        .cookie(clear_client_session_cookie(config))
        .json(serde_json::json!({
            "message": "logged out"
        })))
}

pub async fn me(req: HttpRequest) -> Result<HttpResponse, AppError> {
    let session = middleware::extract_tenant_client_user_session(&req).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "clientUser": {
            "id": session.tenant_client_user_id,
            "workspaceId": session.workspace_id,
            "workspaceSlug": session.workspace_slug,
            "tenantClientId": session.tenant_client_id,
            "tenantClientName": session.tenant_client_name,
            "email": session.email,
            "name": session.name,
            "roles": session.roles,
        }
    })))
}
