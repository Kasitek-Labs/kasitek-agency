use actix_web::{web, HttpRequest, HttpResponse};
use bcrypt::{hash, DEFAULT_COST};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::BTreeSet;
use tokio::net::lookup_host;

use crate::{error::AppError, middleware};

#[derive(Debug, FromRow, Serialize)]
struct WorkspaceRow {
    id: String,
    slug: String,
    display_name: String,
    status: String,
}

#[derive(Debug, FromRow, Serialize)]
struct BrandingRow {
    #[sqlx(rename = "workspace_id")]
    workspace_id: String,
    display_name: String,
    primary_color: Option<String>,
    secondary_color: Option<String>,
    logo_url: Option<String>,
    widget_config: serde_json::Value,
}

#[derive(Debug, FromRow)]
struct DomainRow {
    domain: String,
    is_primary: bool,
}

#[derive(Debug, FromRow, Serialize)]
struct WorkspaceDomainRow {
    id: String,
    domain: String,
    is_primary: bool,
    dns_status: String,
    verified_at: Option<chrono::DateTime<Utc>>,
    created_at: chrono::DateTime<Utc>,
}

fn hosted_workspace_domain(
    workspace_slug: &str,
    platform_base_domain: Option<&str>,
) -> Option<String> {
    platform_base_domain.and_then(|base| {
        let normalized_base = base.trim().trim_matches('.').to_ascii_lowercase();
        if normalized_base.is_empty() {
            None
        } else {
            Some(format!("{workspace_slug}.{normalized_base}"))
        }
    })
}

fn normalize_domain(value: &str) -> Result<String, AppError> {
    let normalized = value
        .trim()
        .to_ascii_lowercase()
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_end_matches('/')
        .split('/')
        .next()
        .unwrap_or("")
        .trim()
        .to_string();

    if normalized.is_empty() {
        return Err(AppError::BadRequest("domain is required".to_string()));
    }

    if normalized.contains(' ') || !normalized.contains('.') {
        return Err(AppError::BadRequest(
            "domain must be a valid hostname".to_string(),
        ));
    }

    Ok(normalized)
}

fn verification_message(status: &str, domain: &str, expected_target_host: Option<&str>) -> String {
    match (status, expected_target_host) {
        ("verified", Some(target)) => format!("{domain} resolves to the shared portal target {target}."),
        ("misconfigured", Some(target)) => format!(
            "{domain} resolves publicly, but it does not currently point to the shared portal target {target}."
        ),
        ("pending", Some(target)) => {
            format!("{domain} does not resolve to the shared portal target {target} yet.")
        }
        ("verified", None) => format!("{domain} resolves publicly."),
        ("misconfigured", None) => format!(
            "{domain} resolves publicly, but verification could not confirm the expected platform target."
        ),
        _ => format!("{domain} is not reachable in DNS yet."),
    }
}

async fn resolve_ips(host: &str) -> BTreeSet<String> {
    lookup_host((host, 443))
        .await
        .map(|records| records.map(|addr| addr.ip().to_string()).collect())
        .unwrap_or_default()
}

async fn sync_workspace_custom_domain(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: &str,
) -> Result<(), AppError> {
    let primary_domain = sqlx::query_scalar::<_, String>(
        r#"
        SELECT domain
        FROM workspace_domains
        WHERE workspace_id = $1 AND is_primary = TRUE
        ORDER BY created_at ASC
        LIMIT 1
        "#,
    )
    .bind(workspace_id)
    .fetch_optional(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE workspace_branding
        SET custom_domain = $1, updated_at = NOW()
        WHERE workspace_id = $2
        "#,
    )
    .bind(primary_domain)
    .bind(workspace_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

fn require_workspace_admin(access: &types::TenantAccessContext) -> Result<(), AppError> {
    middleware::require_tenant_roles(&access.tenant_user_roles, &["owner", "admin"])
}

#[derive(Debug, FromRow)]
struct WorkspaceStaffRow {
    id: String,
    email: String,
    name: Option<String>,
    status: String,
    roles: serde_json::Value,
    created_at: chrono::DateTime<Utc>,
    last_login_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct InviteWorkspaceStaffBody {
    email: String,
    name: Option<String>,
    role: Option<String>,
}

fn normalize_staff_role(value: Option<&str>) -> Result<String, AppError> {
    let role = value.unwrap_or("admin").trim().to_ascii_lowercase();
    if role.is_empty() {
        return Err(AppError::BadRequest("role is required".to_string()));
    }

    match role.as_str() {
        "owner" | "admin" | "account_manager" | "support" | "viewer" => Ok(role),
        _ => Err(AppError::BadRequest("invalid staff role".to_string())),
    }
}

fn normalize_staff_email(value: &str) -> Result<String, AppError> {
    let email = value.trim().to_ascii_lowercase();
    if email.is_empty() {
        return Err(AppError::BadRequest("email is required".to_string()));
    }

    if !email.contains('@') {
        return Err(AppError::BadRequest("email must be valid".to_string()));
    }

    Ok(email)
}

async fn issue_workspace_staff_invite(
    pool: &sqlx::PgPool,
    config: &crate::config::AppConfig,
    access: &types::TenantAccessContext,
    tenant_user_id: &str,
    email: &str,
) -> Result<serde_json::Value, AppError> {
    let workspace = sqlx::query_scalar::<_, String>(
        r#"SELECT display_name FROM workspaces WHERE id = $1 LIMIT 1"#,
    )
    .bind(&access.workspace_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or_else(|| access.workspace_slug.clone());

    let invite = crate::controllers::admin_invites::issue_admin_invite(
        pool,
        config,
        &access.workspace_id,
        &access.workspace_slug,
        &workspace,
        tenant_user_id,
        email,
    )
    .await?;

    Ok(serde_json::json!({
        "activationUrl": invite.activation_url,
        "portalUrl": invite.portal_url,
        "emailSent": invite.email_sent,
        "expiresAt": invite.expires_at,
    }))
}

pub async fn list_workspace_staff(req: HttpRequest) -> Result<HttpResponse, AppError> {
    let access = middleware::extract_access_context(&req).await?;
    require_workspace_admin(&access)?;
    let pool = middleware::db_pool(&req)?;

    let staff = sqlx::query_as::<_, WorkspaceStaffRow>(
        r#"
        SELECT
            u.id,
            u.email,
            u.name,
            u.status,
            COALESCE(
                json_agg(r.role ORDER BY r.created_at) FILTER (WHERE r.role IS NOT NULL),
                '[]'::json
            ) AS roles,
            u.created_at AS created_at,
            u.last_login_at AS last_login_at
        FROM tenant_users u
        LEFT JOIN tenant_user_roles r ON r.tenant_user_id = u.id
        WHERE u.workspace_id = $1
        GROUP BY u.id, u.email, u.name, u.status, u.created_at, u.last_login_at
        ORDER BY u.created_at DESC
        "#,
    )
    .bind(&access.workspace_id)
    .fetch_all(pool)
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "staff": staff.into_iter().map(|row| serde_json::json!({
            "id": row.id,
            "email": row.email,
            "name": row.name,
            "status": row.status,
            "roles": row.roles,
            "createdAt": row.created_at,
            "lastLoginAt": row.last_login_at,
        })).collect::<Vec<_>>()
    })))
}

pub async fn add_workspace_staff(
    req: HttpRequest,
    body: web::Json<InviteWorkspaceStaffBody>,
) -> Result<HttpResponse, AppError> {
    let access = middleware::extract_access_context(&req).await?;
    require_workspace_admin(&access)?;
    let pool = middleware::db_pool(&req)?;
    let config = middleware::app_config(&req)?;

    let email = normalize_staff_email(&body.email)?;
    let role = normalize_staff_role(body.role.as_deref())?;
    let name = body
        .name
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    let existing = sqlx::query_as::<_, WorkspaceStaffRow>(
        r#"
        SELECT
            u.id,
            u.email,
            u.name,
            u.status,
            COALESCE(
                json_agg(r.role ORDER BY r.created_at) FILTER (WHERE r.role IS NOT NULL),
                '[]'::json
            ) AS roles,
            u.created_at AS created_at,
            u.last_login_at AS last_login_at
        FROM tenant_users u
        LEFT JOIN tenant_user_roles r ON r.tenant_user_id = u.id
        WHERE u.workspace_id = $1 AND lower(u.email) = lower($2)
        GROUP BY u.id, u.email, u.name, u.status, u.created_at, u.last_login_at
        LIMIT 1
        "#,
    )
    .bind(&access.workspace_id)
    .bind(&email)
    .fetch_optional(pool)
    .await?;

    let user_id = if let Some(existing) = existing {
        if existing.status == "active" && existing.roles.as_array().is_some() {
            sqlx::query(
                r#"INSERT INTO tenant_user_roles (tenant_user_id, role, created_at)
                   VALUES ($1, $2, NOW())
                   ON CONFLICT DO NOTHING"#,
            )
            .bind(&existing.id)
            .bind(&role)
            .execute(pool)
            .await?;

            return Ok(HttpResponse::Ok().json(serde_json::json!({
                "staff": {
                    "id": existing.id,
                    "email": existing.email,
                    "name": existing.name,
                    "status": existing.status,
                    "roles": existing.roles,
                    "createdAt": existing.created_at,
                    "lastLoginAt": existing.last_login_at,
                },
                "inviteSent": false,
            })));
        }

        let temp_hash = hash(crypto_utils::generate_token(24), DEFAULT_COST)
            .map_err(|e| AppError::Internal(format!("password hash failed: {e}")))?;

        sqlx::query(
            r#"
            UPDATE tenant_users
            SET name = COALESCE($1, name),
                password_hash = $2,
                status = 'pending',
                updated_at = NOW()
            WHERE id = $3 AND workspace_id = $4
            "#,
        )
        .bind(&name)
        .bind(&temp_hash)
        .bind(&existing.id)
        .bind(&access.workspace_id)
        .execute(pool)
        .await?;

        sqlx::query("DELETE FROM tenant_user_roles WHERE tenant_user_id = $1")
            .bind(&existing.id)
            .execute(pool)
            .await?;

        sqlx::query(
            r#"INSERT INTO tenant_user_roles (tenant_user_id, role, created_at) VALUES ($1, $2, NOW())"#,
        )
        .bind(&existing.id)
        .bind(&role)
        .execute(pool)
        .await?;

        existing.id
    } else {
        let temp_hash = hash(crypto_utils::generate_token(24), DEFAULT_COST)
            .map_err(|e| AppError::Internal(format!("password hash failed: {e}")))?;
        let user_id = cuid2::create_id();

        sqlx::query(
            r#"
            INSERT INTO tenant_users (id, workspace_id, email, password_hash, name, status, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, 'pending', NOW(), NOW())
            "#,
        )
        .bind(&user_id)
        .bind(&access.workspace_id)
        .bind(&email)
        .bind(&temp_hash)
        .bind(&name)
        .execute(pool)
        .await?;

        sqlx::query(
            r#"INSERT INTO tenant_user_roles (tenant_user_id, role, created_at) VALUES ($1, $2, NOW())"#,
        )
        .bind(&user_id)
        .bind(&role)
        .execute(pool)
        .await?;

        user_id
    };

    let invite = issue_workspace_staff_invite(pool, config, &access, &user_id, &email).await?;

    let staff = sqlx::query_as::<_, WorkspaceStaffRow>(
        r#"
        SELECT
            u.id,
            u.email,
            u.name,
            u.status,
            COALESCE(
                json_agg(r.role ORDER BY r.created_at) FILTER (WHERE r.role IS NOT NULL),
                '[]'::json
            ) AS roles,
            u.created_at AS created_at,
            u.last_login_at AS last_login_at
        FROM tenant_users u
        LEFT JOIN tenant_user_roles r ON r.tenant_user_id = u.id
        WHERE u.id = $1
        GROUP BY u.id, u.email, u.name, u.status, u.created_at, u.last_login_at
        LIMIT 1
        "#,
    )
    .bind(&user_id)
    .fetch_one(pool)
    .await?;

    Ok(HttpResponse::Created().json(serde_json::json!({
        "staff": {
            "id": staff.id,
            "email": staff.email,
            "name": staff.name,
            "status": staff.status,
            "roles": staff.roles,
            "createdAt": staff.created_at,
            "lastLoginAt": staff.last_login_at,
        },
        "invite": invite,
        "inviteSent": invite["emailSent"].as_bool().unwrap_or(false),
    })))
}

pub async fn remove_workspace_staff(
    req: HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let access = middleware::extract_access_context(&req).await?;
    require_workspace_admin(&access)?;
    let pool = middleware::db_pool(&req)?;
    let user_id = path.into_inner();

    if access.tenant_user_id.as_deref() == Some(user_id.as_str()) {
        return Err(AppError::BadRequest(
            "you cannot remove your own account".to_string(),
        ));
    }

    let owner_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM tenant_users u
        JOIN tenant_user_roles r ON r.tenant_user_id = u.id
        WHERE u.workspace_id = $1
          AND r.role = 'owner'
        "#,
    )
    .bind(&access.workspace_id)
    .fetch_one(pool)
    .await?;

    let target_is_owner = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM tenant_user_roles
        WHERE tenant_user_id = $1 AND role = 'owner'
        "#,
    )
    .bind(&user_id)
    .fetch_one(pool)
    .await?
        > 0;

    if target_is_owner && owner_count <= 1 {
        return Err(AppError::BadRequest(
            "you must keep at least one owner on the workspace".to_string(),
        ));
    }

    let deleted = sqlx::query_scalar::<_, String>(
        r#"DELETE FROM tenant_users WHERE id = $1 AND workspace_id = $2 RETURNING id"#,
    )
    .bind(&user_id)
    .bind(&access.workspace_id)
    .fetch_optional(pool)
    .await?;

    if deleted.is_none() {
        return Err(AppError::NotFound("staff user not found".to_string()));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({ "deleted": true })))
}

pub async fn get_workspace(req: HttpRequest) -> Result<HttpResponse, AppError> {
    let access = middleware::extract_access_context(&req).await?;
    if access.tenant_user_id.is_some() {
        middleware::require_tenant_roles(
            &access.tenant_user_roles,
            &["owner", "admin", "account_manager", "viewer", "support"],
        )?;
    }
    let pool = middleware::db_pool(&req)?;
    let config = middleware::app_config(&req)?;

    let workspace = sqlx::query_as::<_, WorkspaceRow>(
        r#"
        SELECT id, slug, display_name, status
        FROM workspaces
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(&access.workspace_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("workspace not found".to_string()))?;

    let domains = sqlx::query_as::<_, DomainRow>(
        r#"
        SELECT domain, is_primary
        FROM workspace_domains
        WHERE workspace_id = $1
        ORDER BY is_primary DESC, domain ASC
        "#,
    )
    .bind(&access.workspace_id)
    .fetch_all(pool)
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "workspace": {
            "id": workspace.id,
            "slug": workspace.slug,
            "displayName": workspace.display_name,
            "status": workspace.status,
            "hostedDomain": hosted_workspace_domain(&workspace.slug, config.platform_base_domain.as_deref()),
            "domains": domains.iter().map(|domain| serde_json::json!({
                "domain": domain.domain,
                "isPrimary": domain.is_primary,
            })).collect::<Vec<_>>(),
            "domainVerification": {
                "expectedTargetHost": config.portal_host_target,
                "hostedDomainBase": config.platform_base_domain,
            },
            "scopes": access.scopes,
            "tenantUserId": access.tenant_user_id,
            "tenantUserRoles": access.tenant_user_roles,
        }
    })))
}

pub async fn get_branding(req: HttpRequest) -> Result<HttpResponse, AppError> {
    let access = middleware::extract_access_context(&req).await?;
    if access.tenant_user_id.is_some() {
        middleware::require_tenant_roles(
            &access.tenant_user_roles,
            &["owner", "admin", "account_manager", "viewer", "support"],
        )?;
    }
    let pool = middleware::db_pool(&req)?;

    let branding = sqlx::query_as::<_, BrandingRow>(
        r#"
        SELECT workspace_id, display_name, primary_color, secondary_color, logo_url, widget_config
        FROM workspace_branding
        WHERE workspace_id = $1
        LIMIT 1
        "#,
    )
    .bind(&access.workspace_id)
    .fetch_optional(pool)
    .await?;

    let custom_domain = sqlx::query_scalar::<_, String>(
        r#"
        SELECT domain
        FROM workspace_domains
        WHERE workspace_id = $1
        ORDER BY is_primary DESC, domain ASC
        LIMIT 1
        "#,
    )
    .bind(&access.workspace_id)
    .fetch_optional(pool)
    .await?;

    let response = if let Some(branding) = branding {
        serde_json::json!({
            "workspaceId": branding.workspace_id,
            "displayName": branding.display_name,
            "primaryColor": branding.primary_color,
            "secondaryColor": branding.secondary_color,
            "logoUrl": branding.logo_url,
            "customDomain": custom_domain,
            "widgetConfig": branding.widget_config,
        })
    } else {
        serde_json::json!({
            "workspaceId": access.workspace_id,
            "displayName": access.workspace_slug,
            "primaryColor": null,
            "secondaryColor": null,
            "logoUrl": null,
            "customDomain": custom_domain,
            "widgetConfig": {},
        })
    };

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "branding": response
    })))
}

#[derive(Debug, serde::Deserialize)]
pub struct CreateWorkspaceDomainBody {
    domain: String,
    #[serde(rename = "makePrimary")]
    make_primary: Option<bool>,
}

pub async fn list_workspace_domains(req: HttpRequest) -> Result<HttpResponse, AppError> {
    let access = middleware::extract_access_context(&req).await?;
    require_workspace_admin(&access)?;
    let pool = middleware::db_pool(&req)?;
    let config = middleware::app_config(&req)?;

    let domains = sqlx::query_as::<_, WorkspaceDomainRow>(
        r#"
        SELECT
            id,
            domain,
            is_primary,
            dns_status,
            verified_at,
            created_at
        FROM workspace_domains
        WHERE workspace_id = $1
        ORDER BY is_primary DESC, created_at ASC
        "#,
    )
    .bind(&access.workspace_id)
    .fetch_all(pool)
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "hostedDomain": hosted_workspace_domain(&access.workspace_slug, config.platform_base_domain.as_deref()),
        "domains": domains.into_iter().map(|row| serde_json::json!({
            "id": row.id,
            "domain": row.domain.clone(),
            "isPrimary": row.is_primary,
            "dnsStatus": row.dns_status,
            "verifiedAt": row.verified_at,
            "createdAt": row.created_at,
            "verificationMessage": verification_message(&row.dns_status, &row.domain, config.portal_host_target.as_deref()),
        })).collect::<Vec<_>>(),
        "verification": {
            "expectedTargetHost": config.portal_host_target,
            "hostedDomainBase": config.platform_base_domain,
        }
    })))
}

pub async fn add_workspace_domain(
    req: HttpRequest,
    body: actix_web::web::Json<CreateWorkspaceDomainBody>,
) -> Result<HttpResponse, AppError> {
    let access = middleware::extract_access_context(&req).await?;
    require_workspace_admin(&access)?;
    let pool = middleware::db_pool(&req)?;
    let config = middleware::app_config(&req)?;
    let domain = normalize_domain(&body.domain)?;
    let make_primary = body.make_primary.unwrap_or(false);

    let existing = sqlx::query_scalar::<_, String>(
        r#"SELECT id FROM workspace_domains WHERE lower(domain) = lower($1) LIMIT 1"#,
    )
    .bind(&domain)
    .fetch_optional(pool)
    .await?;

    if existing.is_some() {
        return Err(AppError::BadRequest("domain already exists".to_string()));
    }

    let domain_id = cuid2::create_id();
    let mut tx = pool.begin().await?;
    let existing_count = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*) FROM workspace_domains WHERE workspace_id = $1"#,
    )
    .bind(&access.workspace_id)
    .fetch_one(&mut *tx)
    .await?;

    let should_be_primary = make_primary || existing_count == 0;
    if should_be_primary {
        sqlx::query(
            r#"UPDATE workspace_domains SET is_primary = FALSE, updated_at = NOW() WHERE workspace_id = $1"#,
        )
        .bind(&access.workspace_id)
        .execute(&mut *tx)
        .await?;
    }

    sqlx::query(
        r#"
        INSERT INTO workspace_domains (id, workspace_id, domain, is_primary, dns_status, created_at, updated_at)
        VALUES ($1, $2, $3, $4, 'pending', NOW(), NOW())
        "#,
    )
    .bind(&domain_id)
    .bind(&access.workspace_id)
    .bind(&domain)
    .bind(should_be_primary)
    .execute(&mut *tx)
    .await?;

    sync_workspace_custom_domain(&mut tx, &access.workspace_id).await?;
    tx.commit().await?;

    Ok(HttpResponse::Created().json(serde_json::json!({
        "domain": {
            "id": domain_id,
            "domain": domain.clone(),
            "isPrimary": should_be_primary,
            "dnsStatus": "pending",
            "verifiedAt": serde_json::Value::Null,
            "verificationMessage": verification_message("pending", &domain, config.portal_host_target.as_deref()),
        }
    })))
}

pub async fn verify_workspace_domain(
    req: HttpRequest,
    path: actix_web::web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let access = middleware::extract_access_context(&req).await?;
    require_workspace_admin(&access)?;
    let pool = middleware::db_pool(&req)?;
    let config = middleware::app_config(&req)?;
    let domain_id = path.into_inner();

    let record = sqlx::query_as::<_, WorkspaceDomainRow>(
        r#"
        SELECT
            id,
            domain,
            is_primary,
            dns_status,
            verified_at,
            created_at
        FROM workspace_domains
        WHERE workspace_id = $1 AND id = $2
        LIMIT 1
        "#,
    )
    .bind(&access.workspace_id)
    .bind(&domain_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("domain not found".to_string()))?;

    let domain_ips = resolve_ips(&record.domain).await;
    let dns_status = if domain_ips.is_empty() {
        "pending"
    } else if let Some(expected_target_host) = config.portal_host_target.as_deref() {
        let target_ips = resolve_ips(expected_target_host).await;
        if !target_ips.is_empty() && domain_ips.iter().any(|ip| target_ips.contains(ip)) {
            "verified"
        } else {
            "misconfigured"
        }
    } else {
        "verified"
    };

    sqlx::query(
        r#"
        UPDATE workspace_domains
        SET dns_status = $1,
            verified_at = CASE WHEN $1 = 'verified' THEN NOW() ELSE NULL END,
            updated_at = NOW()
        WHERE id = $2 AND workspace_id = $3
        "#,
    )
    .bind(dns_status)
    .bind(&domain_id)
    .bind(&access.workspace_id)
    .execute(pool)
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "domain": {
            "id": record.id,
            "domain": record.domain,
            "isPrimary": record.is_primary,
            "dnsStatus": dns_status,
            "verifiedAt": if dns_status == "verified" { serde_json::json!(Utc::now()) } else { serde_json::Value::Null },
            "verificationMessage": verification_message(dns_status, &record.domain, config.portal_host_target.as_deref()),
        }
    })))
}

pub async fn set_workspace_primary_domain(
    req: HttpRequest,
    path: actix_web::web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let access = middleware::extract_access_context(&req).await?;
    require_workspace_admin(&access)?;
    let pool = middleware::db_pool(&req)?;
    let domain_id = path.into_inner();

    let mut tx = pool.begin().await?;
    let exists = sqlx::query_scalar::<_, String>(
        r#"SELECT id FROM workspace_domains WHERE workspace_id = $1 AND id = $2 LIMIT 1"#,
    )
    .bind(&access.workspace_id)
    .bind(&domain_id)
    .fetch_optional(&mut *tx)
    .await?;

    if exists.is_none() {
        return Err(AppError::NotFound("domain not found".to_string()));
    }

    sqlx::query(
        r#"UPDATE workspace_domains SET is_primary = FALSE, updated_at = NOW() WHERE workspace_id = $1"#,
    )
    .bind(&access.workspace_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"UPDATE workspace_domains SET is_primary = TRUE, updated_at = NOW() WHERE workspace_id = $1 AND id = $2"#,
    )
    .bind(&access.workspace_id)
    .bind(&domain_id)
    .execute(&mut *tx)
    .await?;

    sync_workspace_custom_domain(&mut tx, &access.workspace_id).await?;
    tx.commit().await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "updated": true })))
}

pub async fn remove_workspace_domain(
    req: HttpRequest,
    path: actix_web::web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let access = middleware::extract_access_context(&req).await?;
    require_workspace_admin(&access)?;
    let pool = middleware::db_pool(&req)?;
    let domain_id = path.into_inner();

    let mut tx = pool.begin().await?;
    let domain = sqlx::query_as::<_, WorkspaceDomainRow>(
        r#"
        SELECT
            id,
            domain,
            is_primary,
            dns_status,
            verified_at,
            created_at
        FROM workspace_domains
        WHERE workspace_id = $1 AND id = $2
        LIMIT 1
        "#,
    )
    .bind(&access.workspace_id)
    .bind(&domain_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("domain not found".to_string()))?;

    sqlx::query(r#"DELETE FROM workspace_domains WHERE id = $1 AND workspace_id = $2"#)
        .bind(&domain_id)
        .bind(&access.workspace_id)
        .execute(&mut *tx)
        .await?;

    if domain.is_primary {
        let next_primary = sqlx::query_scalar::<_, String>(
            r#"SELECT id FROM workspace_domains WHERE workspace_id = $1 ORDER BY created_at ASC LIMIT 1"#,
        )
        .bind(&access.workspace_id)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(next_primary) = next_primary {
            sqlx::query(
                r#"UPDATE workspace_domains SET is_primary = TRUE, updated_at = NOW() WHERE id = $1 AND workspace_id = $2"#,
            )
            .bind(&next_primary)
            .bind(&access.workspace_id)
            .execute(&mut *tx)
            .await?;
        }
    }

    sync_workspace_custom_domain(&mut tx, &access.workspace_id).await?;
    tx.commit().await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "deleted": true })))
}

pub async fn get_public_tenant_context(req: HttpRequest) -> Result<HttpResponse, AppError> {
    let workspace = middleware::resolve_workspace_from_request(&req).await?;
    let pool = middleware::db_pool(&req)?;

    let branding = sqlx::query_as::<_, BrandingRow>(
        r#"
        SELECT workspace_id, display_name, primary_color, secondary_color, logo_url, widget_config
        FROM workspace_branding
        WHERE workspace_id = $1
        LIMIT 1
        "#,
    )
    .bind(&workspace.workspace_id)
    .fetch_optional(pool)
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "workspace": {
            "id": workspace.workspace_id,
            "slug": workspace.workspace_slug,
            "displayName": workspace.display_name,
        },
        "branding": branding.map(|row| serde_json::json!({
            "displayName": row.display_name,
            "primaryColor": row.primary_color,
            "secondaryColor": row.secondary_color,
            "logoUrl": row.logo_url,
            "widgetConfig": row.widget_config,
        }))
    })))
}
