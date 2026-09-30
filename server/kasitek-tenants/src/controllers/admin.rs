use actix_web::{web, HttpRequest, HttpResponse};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::BTreeSet;
use tokio::net::lookup_host;

use crate::{analytics, error::AppError, middleware};

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

async fn resolve_workspace_id(pool: &sqlx::PgPool, id_or_slug: &str) -> Result<String, AppError> {
    sqlx::query_scalar::<_, String>(
        r#"SELECT id FROM workspaces WHERE id = $1 OR slug = $1 LIMIT 1"#,
    )
    .bind(id_or_slug)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("workspace not found".to_string()))
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

fn require_admin_secret(req: &HttpRequest) -> Result<(), AppError> {
    let config = middleware::app_config(req)?;
    let secret = match &config.admin_secret {
        Some(s) => s.clone(),
        None => return Ok(()), // no secret configured — admin API open (dev only)
    };
    let provided = req
        .headers()
        .get("x-admin-secret")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if provided != secret {
        Err(AppError::Forbidden)
    } else {
        Ok(())
    }
}

#[derive(Debug, FromRow, Serialize)]
struct WorkspaceSummaryRow {
    id: String,
    slug: String,
    display_name: String,
    status: String,
    client_count: i64,
    created_at: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
pub(crate) struct WorkspaceDomainRow {
    id: String,
    domain: String,
    is_primary: bool,
    dns_status: String,
    verified_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct DomainVerificationMetadata {
    expected_target_host: Option<String>,
    hosted_domain_base: Option<String>,
}

#[derive(Debug, Serialize)]
struct DomainVerificationResult {
    id: String,
    domain: String,
    is_primary: bool,
    dns_status: String,
    verified_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    verification_message: String,
}

pub(crate) fn verification_message(
    status: &str,
    domain: &str,
    expected_target_host: Option<&str>,
) -> String {
    match (status, expected_target_host) {
        ("verified", Some(target)) => format!("{domain} resolves to the shared portal target {target}."),
        ("misconfigured", Some(target)) => format!(
            "{domain} resolves publicly, but it does not currently point to the shared portal target {target}."
        ),
        ("pending", Some(target)) => format!(
            "{domain} does not resolve to the shared portal target {target} yet."
        ),
        ("verified", None) => format!("{domain} resolves publicly."),
        ("misconfigured", None) => format!("{domain} resolves publicly, but verification could not confirm the expected platform target."),
        _ => format!("{domain} is not reachable in DNS yet."),
    }
}

fn to_domain_result(
    row: WorkspaceDomainRow,
    expected_target_host: Option<&str>,
) -> DomainVerificationResult {
    let verification_message =
        verification_message(&row.dns_status, &row.domain, expected_target_host);

    DomainVerificationResult {
        id: row.id,
        domain: row.domain,
        is_primary: row.is_primary,
        dns_status: row.dns_status,
        verified_at: row.verified_at,
        created_at: row.created_at,
        verification_message,
    }
}

pub(crate) async fn resolve_ips(host: &str) -> BTreeSet<String> {
    lookup_host((host, 443))
        .await
        .map(|records| records.map(|addr| addr.ip().to_string()).collect())
        .unwrap_or_default()
}

#[derive(Debug, Deserialize)]
pub struct CreateWorkspaceBody {
    #[serde(rename = "workspaceName")]
    workspace_name: String,
    slug: String,
    #[serde(rename = "adminEmail")]
    admin_email: String,
    #[serde(rename = "primaryColor")]
    primary_color: Option<String>,
    #[serde(rename = "reportingTimezone")]
    reporting_timezone: Option<String>,
    domain: Option<String>,
}

pub(crate) fn normalize_reporting_timezone(value: Option<&str>) -> Result<String, AppError> {
    let value = value.unwrap_or("UTC").trim();
    if value.is_empty()
        || value.len() > 64
        || value.chars().any(char::is_whitespace)
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '/' | '_' | '-' | '+')
        })
    {
        return Err(AppError::BadRequest(
            "reportingTimezone must be a valid timezone identifier".to_string(),
        ));
    }
    Ok(value.to_string())
}

pub async fn list_workspaces(req: HttpRequest) -> Result<HttpResponse, AppError> {
    require_admin_secret(&req)?;
    let pool = middleware::db_pool(&req)?;

    let workspaces = sqlx::query_as::<_, WorkspaceSummaryRow>(
        r#"
        SELECT
            w.id,
            w.slug,
            w.display_name,
            w.status,
            COUNT(DISTINCT c.id) AS client_count,
            w.created_at AS created_at
        FROM workspaces w
        LEFT JOIN tenant_clients c ON c.workspace_id = w.id
        GROUP BY w.id, w.slug, w.display_name, w.status, w.created_at
        ORDER BY w.created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    let workspaces = workspaces
        .into_iter()
        .map(|workspace| {
            serde_json::json!({
                "id": workspace.id,
                "slug": workspace.slug,
                "displayName": workspace.display_name,
                "status": workspace.status,
                "clientCount": workspace.client_count,
                "createdAt": workspace.created_at,
            })
        })
        .collect::<Vec<_>>();

    Ok(HttpResponse::Ok().json(serde_json::json!({ "workspaces": workspaces })))
}

pub async fn get_workspace(
    req: HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    require_admin_secret(&req)?;
    let pool = middleware::db_pool(&req)?;
    let config = middleware::app_config(&req)?;
    let id_or_slug = path.into_inner();

    let workspace = sqlx::query_as::<_, WorkspaceSummaryRow>(
        r#"
        SELECT
            w.id,
            w.slug,
            w.display_name,
            w.status,
            COUNT(DISTINCT c.id) AS client_count,
            w.created_at AS created_at
        FROM workspaces w
        LEFT JOIN tenant_clients c ON c.workspace_id = w.id
        WHERE w.id = $1 OR w.slug = $1
        GROUP BY w.id, w.slug, w.display_name, w.status, w.created_at
        LIMIT 1
        "#,
    )
    .bind(&id_or_slug)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("workspace not found".to_string()))?;

    let domains = sqlx::query_as::<_, WorkspaceDomainRow>(
        r#"
        SELECT
            id,
            domain,
            is_primary,
            dns_status,
            verified_at AS verified_at,
            created_at AS created_at
        FROM workspace_domains
        WHERE workspace_id = $1
        ORDER BY is_primary DESC, created_at ASC
        "#,
    )
    .bind(&workspace.id)
    .fetch_all(pool)
    .await?;

    let hosted_domain =
        hosted_workspace_domain(&workspace.slug, config.platform_base_domain.as_deref());

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "workspace": {
            "id": workspace.id,
            "slug": workspace.slug,
            "displayName": workspace.display_name,
            "status": workspace.status,
            "hostedDomain": hosted_domain,
            "clientCount": workspace.client_count,
            "domains": domains.iter().map(|domain| domain.domain.clone()).collect::<Vec<_>>(),
            "domainRecords": domains
                .into_iter()
                .map(|row| to_domain_result(row, config.portal_host_target.as_deref()))
                .collect::<Vec<_>>(),
            "createdAt": workspace.created_at,
            "domainVerification": {
                "expectedTargetHost": config.portal_host_target,
                "hostedDomainBase": config.platform_base_domain,
            }
        }
    })))
}

pub async fn update_workspace(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<serde_json::Value>,
) -> Result<HttpResponse, AppError> {
    require_admin_secret(&req)?;
    let pool = middleware::db_pool(&req)?;
    let id_or_slug = path.into_inner();

    // Resolve to actual id
    let workspace_id = sqlx::query_scalar::<_, String>(
        r#"SELECT id FROM workspaces WHERE id = $1 OR slug = $1 LIMIT 1"#,
    )
    .bind(&id_or_slug)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("workspace not found".to_string()))?;

    // Update workspace status / display_name if provided
    if let Some(status) = body.get("status").and_then(|v| v.as_str()) {
        sqlx::query(r#"UPDATE workspaces SET status = $1, updated_at = NOW() WHERE id = $2"#)
            .bind(status)
            .bind(&workspace_id)
            .execute(pool)
            .await?;
    }
    if let Some(name) = body.get("displayName").and_then(|v| v.as_str()) {
        sqlx::query(r#"UPDATE workspaces SET display_name = $1, updated_at = NOW() WHERE id = $2"#)
            .bind(name)
            .bind(&workspace_id)
            .execute(pool)
            .await?;
        sqlx::query(
            r#"UPDATE workspace_branding SET display_name = $1, updated_at = NOW() WHERE workspace_id = $2"#,
        )
        .bind(name)
        .bind(&workspace_id)
        .execute(pool)
        .await?;
    }

    // Return updated workspace
    get_workspace(req, web::Path::from(workspace_id)).await
}

// ── Staff ──────────────────────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow, Serialize)]
struct StaffRow {
    id: String,
    email: String,
    status: String,
    roles: serde_json::Value,
    created_at: DateTime<Utc>,
}

pub async fn list_workspace_staff(
    req: HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    require_admin_secret(&req)?;
    let pool = middleware::db_pool(&req)?;
    let id_or_slug = path.into_inner();

    let workspace_id = sqlx::query_scalar::<_, String>(
        r#"SELECT id FROM workspaces WHERE id = $1 OR slug = $1 LIMIT 1"#,
    )
    .bind(&id_or_slug)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("workspace not found".to_string()))?;

    let staff = sqlx::query_as::<_, StaffRow>(
        r#"
        SELECT
            u.id,
            u.email,
            u.status,
            COALESCE(
                json_agg(r.role ORDER BY r.created_at) FILTER (WHERE r.role IS NOT NULL),
                '[]'::json
            ) AS roles,
            u.created_at AS created_at
        FROM tenant_users u
        LEFT JOIN tenant_user_roles r ON r.tenant_user_id = u.id
        WHERE u.workspace_id = $1
        GROUP BY u.id, u.email, u.status, u.created_at
        ORDER BY u.created_at DESC
        "#,
    )
    .bind(&workspace_id)
    .fetch_all(pool)
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "staff": staff })))
}

#[derive(Debug, Deserialize)]
pub struct AddStaffBody {
    email: String,
    password: String,
    role: Option<String>,
}

pub async fn add_workspace_staff(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<AddStaffBody>,
) -> Result<HttpResponse, AppError> {
    require_admin_secret(&req)?;
    let pool = middleware::db_pool(&req)?;
    let id_or_slug = path.into_inner();

    let workspace_id = sqlx::query_scalar::<_, String>(
        r#"SELECT id FROM workspaces WHERE id = $1 OR slug = $1 LIMIT 1"#,
    )
    .bind(&id_or_slug)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("workspace not found".to_string()))?;

    let email = body.email.trim().to_ascii_lowercase();
    let role = body.role.as_deref().unwrap_or("admin").trim().to_string();

    let existing = sqlx::query_scalar::<_, String>(
        r#"SELECT id FROM tenant_users WHERE workspace_id = $1 AND lower(email) = $2 LIMIT 1"#,
    )
    .bind(&workspace_id)
    .bind(&email)
    .fetch_optional(pool)
    .await?;

    if existing.is_some() {
        return Err(AppError::BadRequest(
            "staff user already exists in this workspace".to_string(),
        ));
    }

    let password_hash = bcrypt::hash(body.password.trim(), bcrypt::DEFAULT_COST)
        .map_err(|e| AppError::Internal(format!("hash failed: {e}")))?;

    let user_id = cuid2::create_id();
    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"INSERT INTO tenant_users (id, workspace_id, email, password_hash, status, created_at, updated_at)
           VALUES ($1, $2, $3, $4, 'active', NOW(), NOW())"#,
    )
    .bind(&user_id)
    .bind(&workspace_id)
    .bind(&email)
    .bind(&password_hash)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"INSERT INTO tenant_user_roles (tenant_user_id, role, created_at) VALUES ($1, $2, NOW())"#,
    )
    .bind(&user_id)
    .bind(&role)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(HttpResponse::Created().json(serde_json::json!({
        "user": { "id": user_id, "email": email, "role": role, "status": "active" }
    })))
}

pub async fn remove_workspace_staff(
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, AppError> {
    require_admin_secret(&req)?;
    let pool = middleware::db_pool(&req)?;
    let (id_or_slug, user_id) = path.into_inner();

    let workspace_id = sqlx::query_scalar::<_, String>(
        r#"SELECT id FROM workspaces WHERE id = $1 OR slug = $1 LIMIT 1"#,
    )
    .bind(&id_or_slug)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("workspace not found".to_string()))?;

    let deleted = sqlx::query_scalar::<_, String>(
        r#"DELETE FROM tenant_users WHERE id = $1 AND workspace_id = $2 RETURNING id"#,
    )
    .bind(&user_id)
    .bind(&workspace_id)
    .fetch_optional(pool)
    .await?;

    if deleted.is_none() {
        return Err(AppError::NotFound("staff user not found".to_string()));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({ "deleted": true })))
}

// ── Clients (admin view) ───────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow, Serialize)]
struct AdminClientRow {
    id: String,
    name: String,
    slug: String,
    status: String,
    user_count: i64,
    created_at: DateTime<Utc>,
}

pub async fn list_workspace_clients(
    req: HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    require_admin_secret(&req)?;
    let pool = middleware::db_pool(&req)?;
    let id_or_slug = path.into_inner();

    let workspace_id = sqlx::query_scalar::<_, String>(
        r#"SELECT id FROM workspaces WHERE id = $1 OR slug = $1 LIMIT 1"#,
    )
    .bind(&id_or_slug)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("workspace not found".to_string()))?;

    let clients = sqlx::query_as::<_, AdminClientRow>(
        r#"
        SELECT
            c.id,
            c.name,
            c.slug,
            c.status,
            COUNT(DISTINCT u.id) AS user_count,
            c.created_at AS created_at
        FROM tenant_clients c
        LEFT JOIN tenant_client_users u ON u.tenant_client_id = c.id AND u.status != 'revoked'
        WHERE c.workspace_id = $1
        GROUP BY c.id, c.name, c.slug, c.status, c.created_at
        ORDER BY c.created_at DESC
        "#,
    )
    .bind(&workspace_id)
    .fetch_all(pool)
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "clients": clients })))
}

// ── API Keys ───────────────────────────────────────────────────────────────────

#[derive(Debug, sqlx::FromRow, Serialize)]
struct ApiKeyRow {
    id: String,
    label: String,
    key_prefix: String,
    scopes: serde_json::Value,
    revoked_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

pub async fn list_workspace_api_keys(
    req: HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    require_admin_secret(&req)?;
    let pool = middleware::db_pool(&req)?;
    let id_or_slug = path.into_inner();

    let workspace_id = sqlx::query_scalar::<_, String>(
        r#"SELECT id FROM workspaces WHERE id = $1 OR slug = $1 LIMIT 1"#,
    )
    .bind(&id_or_slug)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("workspace not found".to_string()))?;

    let keys = sqlx::query_as::<_, ApiKeyRow>(
        r#"
        SELECT
            id,
            label,
            key_prefix,
            scopes,
            revoked_at AS revoked_at,
            created_at AS created_at
        FROM workspace_api_keys
        WHERE workspace_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(&workspace_id)
    .fetch_all(pool)
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "apiKeys": keys })))
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyBody {
    label: String,
    scopes: Option<serde_json::Value>,
}

pub async fn create_workspace_api_key(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<CreateApiKeyBody>,
) -> Result<HttpResponse, AppError> {
    require_admin_secret(&req)?;
    let pool = middleware::db_pool(&req)?;
    let id_or_slug = path.into_inner();

    let workspace_id = sqlx::query_scalar::<_, String>(
        r#"SELECT id FROM workspaces WHERE id = $1 OR slug = $1 LIMIT 1"#,
    )
    .bind(&id_or_slug)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("workspace not found".to_string()))?;

    let label = body.label.trim();
    if label.is_empty() {
        return Err(AppError::BadRequest("label is required".to_string()));
    }

    let default_scopes = serde_json::json!(["workspace:read", "clients:read", "portal:bootstrap"]);
    let scopes = body.scopes.as_ref().unwrap_or(&default_scopes);
    let key_plain = format!("ktnt_{}", crypto_utils::generate_token(32));
    let key_prefix = key_plain.chars().take(12).collect::<String>();
    let key_hash = crypto_utils::sha256_hex(&key_plain);
    let key_id = cuid2::create_id();

    sqlx::query(
        r#"INSERT INTO workspace_api_keys (id, workspace_id, label, key_prefix, key_hash, scopes, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())"#,
    )
    .bind(&key_id)
    .bind(&workspace_id)
    .bind(label)
    .bind(&key_prefix)
    .bind(&key_hash)
    .bind(scopes)
    .execute(pool)
    .await?;

    Ok(HttpResponse::Created().json(serde_json::json!({
        "apiKey": {
            "id": key_id,
            "label": label,
            "key": key_plain,
            "prefix": key_prefix,
            "scopes": scopes,
        }
    })))
}

pub async fn revoke_workspace_api_key(
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, AppError> {
    require_admin_secret(&req)?;
    let pool = middleware::db_pool(&req)?;
    let (id_or_slug, key_id) = path.into_inner();

    let workspace_id = sqlx::query_scalar::<_, String>(
        r#"SELECT id FROM workspaces WHERE id = $1 OR slug = $1 LIMIT 1"#,
    )
    .bind(&id_or_slug)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("workspace not found".to_string()))?;

    let revoked = sqlx::query_scalar::<_, String>(
        r#"UPDATE workspace_api_keys
           SET revoked_at = NOW(), updated_at = NOW()
           WHERE id = $1 AND workspace_id = $2 AND revoked_at IS NULL
           RETURNING id"#,
    )
    .bind(&key_id)
    .bind(&workspace_id)
    .fetch_optional(pool)
    .await?;

    if revoked.is_none() {
        return Err(AppError::NotFound(
            "api key not found or already revoked".to_string(),
        ));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({ "revoked": true })))
}

#[derive(Debug, Deserialize)]
pub struct CreateWorkspaceDomainBody {
    domain: String,
    #[serde(rename = "makePrimary")]
    make_primary: Option<bool>,
}

pub async fn list_workspace_domains(
    req: HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    require_admin_secret(&req)?;
    let pool = middleware::db_pool(&req)?;
    let config = middleware::app_config(&req)?;
    let workspace_id = resolve_workspace_id(pool, &path.into_inner()).await?;

    let domains = sqlx::query_as::<_, WorkspaceDomainRow>(
        r#"
        SELECT
            id,
            domain,
            is_primary,
            dns_status,
            verified_at AS verified_at,
            created_at AS created_at
        FROM workspace_domains
        WHERE workspace_id = $1
        ORDER BY is_primary DESC, created_at ASC
        "#,
    )
    .bind(&workspace_id)
    .fetch_all(pool)
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "domains": domains
            .into_iter()
            .map(|row| to_domain_result(row, config.portal_host_target.as_deref()))
            .collect::<Vec<_>>(),
        "verification": DomainVerificationMetadata {
            expected_target_host: config.portal_host_target.clone(),
            hosted_domain_base: config.platform_base_domain.clone(),
        }
    })))
}

pub async fn add_workspace_domain(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<CreateWorkspaceDomainBody>,
) -> Result<HttpResponse, AppError> {
    require_admin_secret(&req)?;
    let pool = middleware::db_pool(&req)?;
    let config = middleware::app_config(&req)?;
    let workspace_id = resolve_workspace_id(pool, &path.into_inner()).await?;
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
    .bind(&workspace_id)
    .fetch_one(&mut *tx)
    .await?;

    let should_be_primary = make_primary || existing_count == 0;
    if should_be_primary {
        sqlx::query(
            r#"UPDATE workspace_domains SET is_primary = FALSE, updated_at = NOW() WHERE workspace_id = $1"#,
        )
        .bind(&workspace_id)
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
    .bind(&workspace_id)
    .bind(&domain)
    .bind(should_be_primary)
    .execute(&mut *tx)
    .await?;

    sync_workspace_custom_domain(&mut tx, &workspace_id).await?;
    tx.commit().await?;

    Ok(HttpResponse::Created().json(serde_json::json!({
        "domain": DomainVerificationResult {
            id: domain_id,
            domain: domain.clone(),
            is_primary: should_be_primary,
            dns_status: "pending".to_string(),
            verified_at: None,
            created_at: Utc::now(),
            verification_message: verification_message("pending", &domain, config.portal_host_target.as_deref()),
        },
        "verification": DomainVerificationMetadata {
            expected_target_host: config.portal_host_target.clone(),
            hosted_domain_base: config.platform_base_domain.clone(),
        }
    })))
}

pub async fn verify_workspace_domain(
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, AppError> {
    require_admin_secret(&req)?;
    let pool = middleware::db_pool(&req)?;
    let config = middleware::app_config(&req)?;
    let (id_or_slug, domain_id) = path.into_inner();
    let workspace_id = resolve_workspace_id(pool, &id_or_slug).await?;

    let record = sqlx::query_as::<_, WorkspaceDomainRow>(
        r#"
        SELECT
            id,
            domain,
            is_primary,
            dns_status,
            verified_at AS verified_at,
            created_at AS created_at
        FROM workspace_domains
        WHERE workspace_id = $1 AND id = $2
        LIMIT 1
        "#,
    )
    .bind(&workspace_id)
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
    .bind(&workspace_id)
    .execute(pool)
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "domain": {
            "id": record.id,
            "domain": record.domain,
            "isPrimary": record.is_primary,
            "dnsStatus": dns_status,
            "verifiedAt": if dns_status == "verified" { serde_json::json!(Utc::now()) } else { serde_json::Value::Null },
            "verificationMessage": verification_message(
                dns_status,
                &record.domain,
                config.portal_host_target.as_deref(),
            ),
        },
        "verification": {
            "expectedTargetHost": config.portal_host_target,
            "hostedDomainBase": config.platform_base_domain,
            "resolvedIps": domain_ips,
        }
    })))
}

pub async fn set_workspace_primary_domain(
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, AppError> {
    require_admin_secret(&req)?;
    let pool = middleware::db_pool(&req)?;
    let (id_or_slug, domain_id) = path.into_inner();
    let workspace_id = resolve_workspace_id(pool, &id_or_slug).await?;

    let mut tx = pool.begin().await?;
    let exists = sqlx::query_scalar::<_, String>(
        r#"SELECT id FROM workspace_domains WHERE workspace_id = $1 AND id = $2 LIMIT 1"#,
    )
    .bind(&workspace_id)
    .bind(&domain_id)
    .fetch_optional(&mut *tx)
    .await?;

    if exists.is_none() {
        return Err(AppError::NotFound("domain not found".to_string()));
    }

    sqlx::query(
        r#"UPDATE workspace_domains SET is_primary = FALSE, updated_at = NOW() WHERE workspace_id = $1"#,
    )
    .bind(&workspace_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"UPDATE workspace_domains SET is_primary = TRUE, updated_at = NOW() WHERE workspace_id = $1 AND id = $2"#,
    )
    .bind(&workspace_id)
    .bind(&domain_id)
    .execute(&mut *tx)
    .await?;

    sync_workspace_custom_domain(&mut tx, &workspace_id).await?;
    tx.commit().await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "updated": true })))
}

pub async fn remove_workspace_domain(
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, AppError> {
    require_admin_secret(&req)?;
    let pool = middleware::db_pool(&req)?;
    let (id_or_slug, domain_id) = path.into_inner();
    let workspace_id = resolve_workspace_id(pool, &id_or_slug).await?;

    let mut tx = pool.begin().await?;
    let domain = sqlx::query_as::<_, WorkspaceDomainRow>(
        r#"
        SELECT
            id,
            domain,
            is_primary,
            dns_status,
            verified_at AS verified_at,
            created_at AS created_at
        FROM workspace_domains
        WHERE workspace_id = $1 AND id = $2
        LIMIT 1
        "#,
    )
    .bind(&workspace_id)
    .bind(&domain_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("domain not found".to_string()))?;

    sqlx::query(r#"DELETE FROM workspace_domains WHERE id = $1 AND workspace_id = $2"#)
        .bind(&domain_id)
        .bind(&workspace_id)
        .execute(&mut *tx)
        .await?;

    if domain.is_primary {
        let next_primary = sqlx::query_scalar::<_, String>(
            r#"SELECT id FROM workspace_domains WHERE workspace_id = $1 ORDER BY created_at ASC LIMIT 1"#,
        )
        .bind(&workspace_id)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(next_primary) = next_primary {
            sqlx::query(
                r#"UPDATE workspace_domains SET is_primary = TRUE, updated_at = NOW() WHERE id = $1 AND workspace_id = $2"#,
            )
            .bind(&next_primary)
            .bind(&workspace_id)
            .execute(&mut *tx)
            .await?;
        }
    }

    sync_workspace_custom_domain(&mut tx, &workspace_id).await?;
    tx.commit().await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "deleted": true })))
}

pub async fn create_workspace(
    req: HttpRequest,
    body: web::Json<CreateWorkspaceBody>,
) -> Result<HttpResponse, AppError> {
    require_admin_secret(&req)?;
    let pool = middleware::db_pool(&req)?;
    let config = middleware::app_config(&req)?;

    let workspace_name = body.workspace_name.trim();
    let slug = body.slug.trim().to_ascii_lowercase();
    let admin_email = body.admin_email.trim().to_ascii_lowercase();

    if workspace_name.is_empty() || slug.is_empty() || admin_email.is_empty() {
        return Err(AppError::BadRequest(
            "workspaceName, slug, and adminEmail are required".to_string(),
        ));
    }

    let slug_exists =
        sqlx::query_scalar::<_, String>(r#"SELECT id FROM workspaces WHERE slug = $1 LIMIT 1"#)
            .bind(&slug)
            .fetch_optional(pool)
            .await?;

    if slug_exists.is_some() {
        return Err(AppError::BadRequest(format!(
            "workspace slug already exists: {slug}"
        )));
    }

    let workspace_id = cuid2::create_id();
    let tenant_user_id = cuid2::create_id();
    let api_key_id = cuid2::create_id();
    let api_key_plain = format!("ktnt_{}", crypto_utils::generate_token(32));
    let api_key_prefix = api_key_plain.chars().take(12).collect::<String>();
    let api_key_hash = crypto_utils::sha256_hex(&api_key_plain);
    let scopes = serde_json::json!([
        "workspace:read",
        "workspace:write",
        "clients:read",
        "clients:write",
        "portal:bootstrap"
    ]);
    let now = Utc::now();
    let hosted_domain = hosted_workspace_domain(&slug, config.platform_base_domain.as_deref());
    let reporting_timezone = normalize_reporting_timezone(body.reporting_timezone.as_deref())?;

    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"INSERT INTO workspaces (id, slug, display_name, status, reporting_timezone, created_at, updated_at)
           VALUES ($1, $2, $3, 'active', $4, NOW(), NOW())"#,
    )
    .bind(&workspace_id)
    .bind(&slug)
    .bind(workspace_name)
    .bind(&reporting_timezone)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"INSERT INTO workspace_branding (workspace_id, display_name, primary_color, custom_domain, created_at, updated_at)
           VALUES ($1, $2, $3, $4, NOW(), NOW())"#,
    )
    .bind(&workspace_id)
    .bind(workspace_name)
    .bind(&body.primary_color)
    .bind(&body.domain)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"INSERT INTO workspace_usage_limits (workspace_id, requests_per_month, tokens_per_month, conversations_per_month, created_at, updated_at)
           VALUES ($1, 0, 0, 0, NOW(), NOW())"#,
    )
    .bind(&workspace_id)
    .execute(&mut *tx)
    .await?;

    if let Some(domain) = &body.domain {
        let domain_trimmed = normalize_domain(domain)?;
        if !domain_trimmed.is_empty() {
            sqlx::query(
                r#"INSERT INTO workspace_domains (id, workspace_id, domain, is_primary, dns_status, created_at, updated_at)
                   VALUES ($1, $2, $3, TRUE, 'pending', NOW(), NOW())"#,
            )
            .bind(cuid2::create_id())
            .bind(&workspace_id)
            .bind(&domain_trimmed)
            .execute(&mut *tx)
            .await?;
        }
    }

    sqlx::query(
        r#"INSERT INTO tenant_users (id, workspace_id, email, password_hash, status, created_at, updated_at)
           VALUES ($1, $2, $3, '', 'invited', NOW(), NOW())"#,
    )
    .bind(&tenant_user_id)
    .bind(&workspace_id)
    .bind(&admin_email)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"INSERT INTO tenant_user_roles (tenant_user_id, role, created_at) VALUES ($1, 'owner', NOW())"#,
    )
    .bind(&tenant_user_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"INSERT INTO workspace_api_keys (id, workspace_id, label, key_prefix, key_hash, scopes, created_at, updated_at)
           VALUES ($1, $2, 'initial-provisioning-key', $3, $4, $5, NOW(), NOW())"#,
    )
    .bind(&api_key_id)
    .bind(&workspace_id)
    .bind(&api_key_prefix)
    .bind(&api_key_hash)
    .bind(scopes)
    .execute(&mut *tx)
    .await?;

    analytics::enqueue_workspace_scope_definition(
        &mut tx,
        &workspace_id,
        now.timestamp_millis(),
        &reporting_timezone,
    )
    .await
    .map_err(|error| {
        AppError::Internal(format!("analytics workspace scope enqueue failed: {error}"))
    })?;
    analytics::enqueue_scope_manifest(&mut tx, &workspace_id, 1, now)
        .await
        .map_err(|error| {
            AppError::Internal(format!(
                "analytics workspace manifest enqueue failed: {error}"
            ))
        })?;

    tx.commit().await?;
    let admin_invite = crate::controllers::admin_invites::issue_admin_invite(
        pool,
        config,
        &workspace_id,
        &slug,
        workspace_name,
        &tenant_user_id,
        &admin_email,
    )
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "workspace": {
            "id": workspace_id,
            "slug": slug,
            "displayName": workspace_name,
            "status": "active",
            "hostedDomain": hosted_domain,
            "domains": body
                .domain
                .as_ref()
                .map(|domain| domain.trim())
                .filter(|domain| !domain.is_empty())
                .map(|domain| vec![domain.to_string()])
                .unwrap_or_default(),
            "createdAt": now,
        },
        "adminUser": {
            "id": tenant_user_id,
            "email": admin_email,
            "role": "owner",
            "status": "invited",
        },
        "apiKey": {
            "id": api_key_id,
            "key": api_key_plain,
            "prefix": api_key_prefix,
        },
        "adminInvite": {
            "activationUrl": admin_invite.activation_url,
            "portalUrl": admin_invite.portal_url,
            "emailSent": admin_invite.email_sent,
            "expiresAt": admin_invite.expires_at,
        }
    })))
}
