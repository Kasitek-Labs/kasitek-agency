use actix_web::{
    cookie::{
        time::{Duration, OffsetDateTime},
        Cookie, SameSite,
    },
    web, HttpRequest, HttpResponse,
};
use bcrypt::verify;
use chrono::{Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::{config::AppConfig, error::AppError, middleware};

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, FromRow)]
struct TenantUserRow {
    id: String,
    workspace_id: String,
    email: String,
    password_hash: String,
    name: Option<String>,
    status: String,
}

#[derive(Debug, Serialize)]
pub struct TenantUserPayload {
    pub id: String,
    #[serde(rename = "workspaceId")]
    pub workspace_id: String,
    #[serde(rename = "workspaceSlug")]
    pub workspace_slug: String,
    pub email: String,
    pub name: Option<String>,
    pub roles: Vec<String>,
}

fn same_site(config: &AppConfig) -> SameSite {
    match config.session_cookie_same_site {
        crate::config::CookieSameSitePolicy::Lax => SameSite::Lax,
        crate::config::CookieSameSitePolicy::Strict => SameSite::Strict,
        crate::config::CookieSameSitePolicy::None => SameSite::None,
    }
}

pub(crate) fn build_session_cookie(config: &AppConfig, value: &str) -> Cookie<'static> {
    let mut cookie = Cookie::build(config.session_cookie_name.clone(), value.to_string())
        .path("/")
        .http_only(true)
        .same_site(same_site(config))
        .secure(config.session_cookie_secure)
        .expires(OffsetDateTime::now_utc() + Duration::hours(i64::from(config.session_ttl_hours)))
        .finish();

    if let Some(domain) = &config.session_cookie_domain {
        cookie.set_domain(domain.clone());
    }

    cookie
}

pub(crate) fn clear_session_cookie(config: &AppConfig) -> Cookie<'static> {
    let mut cookie = Cookie::build(config.session_cookie_name.clone(), "")
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

pub(crate) async fn authenticate_tenant_user(
    req: &HttpRequest,
    email: &str,
    password: &str,
) -> Result<Option<(TenantUserPayload, String)>, AppError> {
    let workspace = middleware::resolve_workspace_from_request(req).await?;
    let pool = middleware::db_pool(req)?;
    let config = middleware::app_config(req)?;

    let user = sqlx::query_as::<_, TenantUserRow>(
        r#"
        SELECT id, workspace_id, email, password_hash, name, status
        FROM tenant_users
        WHERE workspace_id = $1 AND lower(email) = lower($2)
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

    if user.status != "active" || !verify(password, &user.password_hash)? {
        return Ok(None);
    }

    let session_token = crypto_utils::generate_token(48);
    let session_token_hash = crypto_utils::sha256_hex(&session_token);
    let session_id = cuid2::create_id();
    let expires_at = Utc::now() + ChronoDuration::hours(i64::from(config.session_ttl_hours));
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
    .bind(&workspace.workspace_id)
    .bind(&user.id)
    .bind(&session_token_hash)
    .bind(expires_at)
    .bind(ip_address)
    .bind(user_agent)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        UPDATE tenant_users
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
        FROM tenant_user_roles
        WHERE tenant_user_id = $1
        ORDER BY role ASC
        "#,
    )
    .bind(&user.id)
    .fetch_all(pool)
    .await?;

    Ok(Some((
        TenantUserPayload {
            id: user.id,
            workspace_id: user.workspace_id,
            workspace_slug: workspace.workspace_slug,
            email: user.email,
            name: user.name,
            roles,
        },
        session_token,
    )))
}

pub async fn login(
    req: HttpRequest,
    body: web::Json<LoginRequest>,
) -> Result<HttpResponse, AppError> {
    let config = middleware::app_config(&req)?;
    let email = body.email.trim().to_ascii_lowercase();
    let password = body.password.trim();

    if email.is_empty() || password.is_empty() {
        return Err(AppError::BadRequest(
            "email and password are required".to_string(),
        ));
    }

    let Some((user, session_token)) = authenticate_tenant_user(&req, &email, password).await?
    else {
        return Err(AppError::Unauthorized);
    };

    Ok(HttpResponse::Ok()
        .cookie(build_session_cookie(config, &session_token))
        .json(serde_json::json!({
            "user": user
        })))
}

pub async fn logout(req: HttpRequest) -> Result<HttpResponse, AppError> {
    let pool = middleware::db_pool(&req)?;
    let config = middleware::app_config(&req)?;

    if let Ok(session_token) = middleware::extract_tenant_session_token(&req) {
        let token_hash = crypto_utils::sha256_hex(&session_token);
        sqlx::query("DELETE FROM tenant_sessions WHERE token_hash = $1")
            .bind(token_hash)
            .execute(pool)
            .await?;
    }

    Ok(HttpResponse::Ok()
        .cookie(clear_session_cookie(config))
        .json(serde_json::json!({
            "message": "logged out"
        })))
}

pub async fn me(req: HttpRequest) -> Result<HttpResponse, AppError> {
    let session = middleware::extract_tenant_user_session(&req).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "user": {
            "id": session.tenant_user_id,
            "workspaceId": session.workspace_id,
            "workspaceSlug": session.workspace_slug,
            "email": session.email,
            "name": session.name,
            "roles": session.roles,
        }
    })))
}
