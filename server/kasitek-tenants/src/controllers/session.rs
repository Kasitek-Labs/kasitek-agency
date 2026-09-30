use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;

use crate::{
    controllers::{
        auth::{authenticate_tenant_user, build_session_cookie, clear_session_cookie},
        client_auth::{
            authenticate_client_user, build_client_session_cookie, clear_client_session_cookie,
        },
    },
    error::AppError,
    middleware,
};

#[derive(Debug, Deserialize)]
pub struct PortalLoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct ContinueWithEmailRequest {
    pub email: String,
}

#[derive(Debug, sqlx::FromRow)]
struct TenantUserSetupRow {
    id: String,
    workspace_id: String,
    workspace_slug: String,
    workspace_display_name: String,
    email: String,
    password_hash: String,
    status: String,
}

#[derive(Debug, sqlx::FromRow)]
struct ClientUserSetupRow {
    id: String,
    workspace_id: String,
    workspace_slug: String,
    tenant_client_id: String,
    email: String,
    password_hash: Option<String>,
    status: String,
}

pub async fn continue_with_email(
    req: HttpRequest,
    body: web::Json<ContinueWithEmailRequest>,
) -> Result<HttpResponse, AppError> {
    let workspace = middleware::resolve_workspace_from_request(&req).await?;
    let pool = middleware::db_pool(&req)?;
    let config = middleware::app_config(&req)?;
    let email = body.email.trim().to_ascii_lowercase();

    if email.is_empty() {
        return Err(AppError::BadRequest("email is required".to_string()));
    }

    let tenant_user = sqlx::query_as::<_, TenantUserSetupRow>(
        r#"
        SELECT
            u.id,
            u.workspace_id,
            w.slug AS workspace_slug,
            w.display_name AS workspace_display_name,
            u.email,
            u.password_hash,
            u.status
        FROM tenant_users u
        JOIN workspaces w ON w.id = u.workspace_id
        WHERE u.workspace_id = $1 AND lower(u.email) = lower($2)
        LIMIT 1
        "#,
    )
    .bind(&workspace.workspace_id)
    .bind(&email)
    .fetch_optional(pool)
    .await?;

    if let Some(user) = tenant_user {
        let needs_setup = user.password_hash.trim().is_empty() || user.status != "active";
        if needs_setup {
            let invite = crate::controllers::admin_invites::issue_admin_invite(
                pool,
                config,
                &user.workspace_id,
                &user.workspace_slug,
                &user.workspace_display_name,
                &user.id,
                &user.email,
            )
            .await?;

            return Ok(HttpResponse::Ok().json(serde_json::json!({
                "audience": "tenant",
                "nextStep": "setup_link_sent",
                "email": user.email,
                "activationUrl": invite.activation_url,
                "portalUrl": invite.portal_url,
                "expiresAt": invite.expires_at,
            })));
        }

        return Ok(HttpResponse::Ok().json(serde_json::json!({
            "audience": "tenant",
            "nextStep": "password",
            "email": user.email,
        })));
    }

    let client_user = sqlx::query_as::<_, ClientUserSetupRow>(
        r#"
        SELECT
            u.id,
            u.workspace_id,
            w.slug AS workspace_slug,
            u.tenant_client_id,
            u.email,
            u.password_hash,
            u.status
        FROM tenant_client_users u
        JOIN workspaces w ON w.id = u.workspace_id
        WHERE u.workspace_id = $1 AND lower(u.email) = lower($2)
        LIMIT 1
        "#,
    )
    .bind(&workspace.workspace_id)
    .bind(&email)
    .fetch_optional(pool)
    .await?;

    if let Some(user) = client_user {
        let needs_setup = user
            .password_hash
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
            || user.status != "active";
        if needs_setup {
            let invite = crate::controllers::client_auth::issue_client_invite(
                pool,
                config,
                &user.workspace_id,
                &user.workspace_slug,
                &user.tenant_client_id,
                &user.id,
                &user.email,
                None,
            )
            .await?;

            return Ok(HttpResponse::Ok().json(serde_json::json!({
                "audience": "client",
                "nextStep": "setup_link_sent",
                "email": user.email,
                "activationUrl": invite.activation_url,
                "portalUrl": invite.portal_url,
                "expiresAt": invite.expires_at,
            })));
        }

        return Ok(HttpResponse::Ok().json(serde_json::json!({
            "audience": "client",
            "nextStep": "password",
            "email": user.email,
        })));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "nextStep": "not_found"
    })))
}

pub async fn login(
    req: HttpRequest,
    body: web::Json<PortalLoginRequest>,
) -> Result<HttpResponse, AppError> {
    let config = middleware::app_config(&req)?;
    let email = body.email.trim().to_ascii_lowercase();
    let password = body.password.trim();

    if email.is_empty() || password.is_empty() {
        return Err(AppError::BadRequest(
            "email and password are required".to_string(),
        ));
    }

    if let Some((user, session_token)) = authenticate_tenant_user(&req, &email, password).await? {
        return Ok(HttpResponse::Ok()
            .cookie(build_session_cookie(config, &session_token))
            .cookie(clear_client_session_cookie(config))
            .json(serde_json::json!({
                "audience": "tenant",
                "user": user,
            })));
    }

    if let Some((client_user, session_token)) =
        authenticate_client_user(&req, &email, password).await?
    {
        return Ok(HttpResponse::Ok()
            .cookie(build_client_session_cookie(config, &session_token))
            .cookie(clear_session_cookie(config))
            .json(serde_json::json!({
                "audience": "client",
                "clientUser": client_user,
            })));
    }

    Err(AppError::Unauthorized)
}

pub async fn get(req: HttpRequest) -> Result<HttpResponse, AppError> {
    if let Ok(session) = middleware::extract_tenant_user_session(&req).await {
        return Ok(HttpResponse::Ok().json(serde_json::json!({
            "audience": "tenant",
            "user": {
                "id": session.tenant_user_id,
                "workspaceId": session.workspace_id,
                "workspaceSlug": session.workspace_slug,
                "email": session.email,
                "name": session.name,
                "roles": session.roles,
            }
        })));
    }

    if let Ok(session) = middleware::extract_tenant_client_user_session(&req).await {
        return Ok(HttpResponse::Ok().json(serde_json::json!({
            "audience": "client",
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
        })));
    }

    Err(AppError::Unauthorized)
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

    if let Ok(session_token) = middleware::extract_client_session_token(&req) {
        let token_hash = crypto_utils::sha256_hex(&session_token);
        sqlx::query("DELETE FROM tenant_client_sessions WHERE token_hash = $1")
            .bind(token_hash)
            .execute(pool)
            .await?;
    }

    Ok(HttpResponse::Ok()
        .cookie(clear_session_cookie(config))
        .cookie(clear_client_session_cookie(config))
        .json(serde_json::json!({
            "message": "logged out"
        })))
}
