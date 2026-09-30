use actix_web::{web, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::{
    analytics,
    config::AppConfig,
    controllers::{admin::normalize_reporting_timezone, client_auth},
    error::AppError,
    middleware,
};

#[derive(Debug, Deserialize)]
pub struct CreateTenantClientRequest {
    pub name: String,
    pub slug: Option<String>,
    pub external_ref: Option<String>,
    #[serde(rename = "adminEmail")]
    pub admin_email: String,
    #[serde(rename = "reportingTimezone")]
    pub reporting_timezone: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTenantClientStatusRequest {
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct InviteTenantClientUserRequest {
    pub email: String,
    pub name: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTenantClientAssignmentRequest {
    #[serde(rename = "tenantUserId")]
    pub tenant_user_id: String,
    #[serde(rename = "assignmentRole")]
    pub assignment_role: Option<String>,
}

#[derive(Debug, FromRow, Serialize)]
struct TenantClientRow {
    id: String,
    name: String,
    slug: String,
    status: String,
    user_count: i64,
    external_ref: Option<String>,
    metadata: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, FromRow, Serialize)]
struct TenantClientUserRow {
    id: String,
    tenant_client_id: String,
    email: String,
    name: Option<String>,
    status: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct TenantClientAssignmentRow {
    id: String,
    tenant_user_id: String,
    email: String,
    name: Option<String>,
    assignment_role: String,
    status: String,
    assigned_at: chrono::DateTime<chrono::Utc>,
    revoked_at: Option<chrono::DateTime<chrono::Utc>>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

fn normalize_slug(input: &str) -> String {
    let mut slug = String::with_capacity(input.len());
    let mut last_dash = false;

    for ch in input.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }

    slug.trim_matches('-').to_string()
}

fn ensure_client_management_roles(roles: &[String]) -> Result<(), AppError> {
    middleware::require_tenant_roles(roles, &["owner", "admin", "account_manager"])
}

fn normalize_assignment_role(value: Option<&str>) -> Result<String, AppError> {
    let role = value.unwrap_or("member").trim().to_ascii_lowercase();
    match role.as_str() {
        "owner" | "admin" | "member" | "viewer" => Ok(role),
        _ => Err(AppError::BadRequest(
            "assignmentRole must be owner, admin, member, or viewer".to_string(),
        )),
    }
}

async fn provision_client_user_invite(
    pool: &sqlx::PgPool,
    config: &AppConfig,
    workspace_id: &str,
    workspace_slug: &str,
    tenant_client_id: &str,
    email: &str,
    name: Option<&str>,
    role: &str,
    invited_by_tenant_user_id: Option<&str>,
) -> Result<(String, client_auth::IssuedClientInvite), AppError> {
    middleware::require_active_tenant_client(pool, workspace_id, tenant_client_id).await?;

    let email = email.trim().to_ascii_lowercase();
    if email.is_empty() {
        return Err(AppError::BadRequest("email is required".to_string()));
    }

    let existing_user = sqlx::query_scalar::<_, String>(
        r#"
        SELECT id
        FROM tenant_client_users
        WHERE tenant_client_id = $1 AND lower(email) = lower($2) AND status = 'active'
        LIMIT 1
        "#,
    )
    .bind(tenant_client_id)
    .bind(&email)
    .fetch_optional(pool)
    .await?;

    if existing_user.is_some() {
        return Err(AppError::BadRequest(
            "client user is already active".to_string(),
        ));
    }

    let tenant_client_user_id = sqlx::query_scalar::<_, String>(
        r#"
        INSERT INTO tenant_client_users (
            id,
            workspace_id,
            tenant_client_id,
            email,
            name,
            status,
            created_at,
            updated_at
        )
        VALUES ($1, $2, $3, $4, $5, 'invited', NOW(), NOW())
        ON CONFLICT (tenant_client_id, email)
        DO UPDATE SET
            name = EXCLUDED.name,
            status = 'invited',
            updated_at = NOW()
        RETURNING id
        "#,
    )
    .bind(cuid2::create_id())
    .bind(workspace_id)
    .bind(tenant_client_id)
    .bind(&email)
    .bind(
        name.map(|value| value.trim())
            .filter(|value| !value.is_empty()),
    )
    .fetch_one(pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO tenant_client_user_roles (tenant_client_user_id, role, created_at)
        VALUES ($1, $2, NOW())
        ON CONFLICT (tenant_client_user_id, role) DO NOTHING
        "#,
    )
    .bind(&tenant_client_user_id)
    .bind(role)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        UPDATE tenant_client_invites
        SET revoked_at = NOW(), updated_at = NOW()
        WHERE tenant_client_user_id = $1
          AND accepted_at IS NULL
          AND revoked_at IS NULL
        "#,
    )
    .bind(&tenant_client_user_id)
    .execute(pool)
    .await?;

    let invite = client_auth::issue_client_invite(
        pool,
        config,
        workspace_id,
        workspace_slug,
        tenant_client_id,
        &tenant_client_user_id,
        &email,
        invited_by_tenant_user_id,
    )
    .await?;

    Ok((tenant_client_user_id, invite))
}

pub async fn list_clients(req: HttpRequest) -> Result<HttpResponse, AppError> {
    let session = middleware::extract_tenant_user_session(&req).await?;
    let pool = middleware::db_pool(&req)?;

    let clients = sqlx::query_as::<_, TenantClientRow>(
        r#"
        SELECT
            c.id,
            c.name,
            c.slug,
            c.status,
            COUNT(DISTINCT u.id)::bigint AS user_count,
            c.external_ref,
            c.metadata,
            c.created_at,
            c.updated_at
        FROM tenant_clients c
        LEFT JOIN tenant_client_users u
          ON u.workspace_id = c.workspace_id
         AND u.tenant_client_id = c.id
        WHERE c.workspace_id = $1
        GROUP BY c.id, c.name, c.slug, c.status, c.external_ref, c.metadata, c.created_at, c.updated_at
        ORDER BY c.updated_at DESC, c.created_at DESC
        "#,
    )
    .bind(&session.workspace_id)
    .fetch_all(pool)
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "workspaceId": session.workspace_id,
        "clients": clients
    })))
}

pub async fn create_client(
    req: HttpRequest,
    body: web::Json<CreateTenantClientRequest>,
) -> Result<HttpResponse, AppError> {
    let session = middleware::extract_tenant_user_session(&req).await?;
    ensure_client_management_roles(&session.roles)?;
    let pool = middleware::db_pool(&req)?;
    let config = middleware::app_config(&req)?;
    let name = body.name.trim();
    let slug = body
        .slug
        .as_ref()
        .map(|value| normalize_slug(value))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| normalize_slug(name));

    if name.is_empty() || slug.is_empty() {
        return Err(AppError::BadRequest(
            "name and slug are required".to_string(),
        ));
    }

    let admin_email = body.admin_email.trim().to_ascii_lowercase();
    if admin_email.is_empty() || !admin_email.contains('@') || admin_email.contains(' ') {
        return Err(AppError::BadRequest(
            "adminEmail must be a valid email address".to_string(),
        ));
    }
    let reporting_timezone = body
        .reporting_timezone
        .as_deref()
        .map(|value| normalize_reporting_timezone(Some(value)))
        .transpose()?
        .unwrap_or_else(|| "UTC".to_string());

    let id = cuid2::create_id();
    let mut transaction = pool.begin().await?;
    let result = sqlx::query(
        r#"
        INSERT INTO tenant_clients (
            id,
            workspace_id,
            name,
            slug,
            external_ref,
            reporting_timezone,
            created_at,
            updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())
        "#,
    )
    .bind(&id)
    .bind(&session.workspace_id)
    .bind(name)
    .bind(&slug)
    .bind(
        body.external_ref
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty()),
    )
    .bind(&reporting_timezone)
    .execute(&mut *transaction)
    .await;

    if result.is_err() {
        return Err(AppError::BadRequest(
            "client slug already exists for this workspace".to_string(),
        ));
    }

    let occurred_at = chrono::Utc::now();
    analytics::enqueue_client_scope_definition(
        &mut transaction,
        &session.workspace_id,
        &id,
        occurred_at.timestamp_millis(),
        &reporting_timezone,
        "active",
    )
    .await
    .map_err(|error| {
        AppError::Internal(format!("analytics client scope enqueue failed: {error}"))
    })?;
    analytics::enqueue_client_lifecycle(
        &mut transaction,
        &session.workspace_id,
        &id,
        "created",
        "active",
        occurred_at,
    )
    .await
    .map_err(|error| {
        AppError::Internal(format!(
            "analytics client lifecycle enqueue failed: {error}"
        ))
    })?;
    let client_event_count = analytics::workspace_event_count(
        &mut transaction,
        &format!("workspace:{}:clients", session.workspace_id),
    )
    .await?;
    analytics::enqueue_client_manifest(
        &mut transaction,
        &session.workspace_id,
        client_event_count,
        occurred_at,
    )
    .await
    .map_err(|error| {
        AppError::Internal(format!("analytics client manifest enqueue failed: {error}"))
    })?;
    transaction.commit().await?;

    let (_, admin_invite) = provision_client_user_invite(
        pool,
        config,
        &session.workspace_id,
        &session.workspace_slug,
        &id,
        &admin_email,
        None,
        "owner",
        Some(&session.tenant_user_id),
    )
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "client": {
            "id": id,
            "workspaceId": session.workspace_id,
            "name": name,
            "slug": slug,
            "externalRef": body.external_ref,
            "status": "active",
        },
        "adminInvite": {
            "email": admin_email,
            "emailSent": admin_invite.email_sent,
            "activationUrl": admin_invite.activation_url,
            "portalUrl": admin_invite.portal_url,
            "expiresAt": admin_invite.expires_at,
        }
    })))
}

pub async fn update_client_status(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<UpdateTenantClientStatusRequest>,
) -> Result<HttpResponse, AppError> {
    let session = middleware::extract_tenant_user_session(&req).await?;
    ensure_client_management_roles(&session.roles)?;
    let pool = middleware::db_pool(&req)?;
    let client_id = path.into_inner();
    let requested_status = body.status.trim().to_ascii_lowercase();
    let (operational_status, analytics_status, action) = match requested_status.as_str() {
        "active" => ("active", "active", "activated"),
        "suspended" => ("suspended", "suspended", "suspended"),
        "archived" | "deleted" => ("archived", "deleted", "deleted"),
        _ => {
            return Err(AppError::BadRequest(
                "status must be active, suspended, or archived".to_string(),
            ))
        }
    };
    let mut transaction = pool.begin().await?;
    let client = sqlx::query_as::<_, (String, String)>(
        r#"SELECT c.status, COALESCE(c.reporting_timezone, w.reporting_timezone)
           FROM tenant_clients c
           JOIN workspaces w ON w.id = c.workspace_id
           WHERE c.id = $1 AND c.workspace_id = $2
           FOR UPDATE"#,
    )
    .bind(&client_id)
    .bind(&session.workspace_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| AppError::NotFound("tenant client not found".to_string()))?;
    if client.0 == operational_status {
        transaction.rollback().await?;
        return Ok(HttpResponse::Ok().json(serde_json::json!({
            "clientId": client_id,
            "status": requested_status,
            "changed": false
        })));
    }
    sqlx::query(
        r#"UPDATE tenant_clients SET status = $1, updated_at = NOW()
           WHERE id = $2 AND workspace_id = $3"#,
    )
    .bind(operational_status)
    .bind(&client_id)
    .bind(&session.workspace_id)
    .execute(&mut *transaction)
    .await?;
    let occurred_at = chrono::Utc::now();
    analytics::enqueue_client_scope_definition(
        &mut transaction,
        &session.workspace_id,
        &client_id,
        occurred_at.timestamp_millis(),
        &client.1,
        analytics_status,
    )
    .await
    .map_err(|error| {
        AppError::Internal(format!("analytics client scope enqueue failed: {error}"))
    })?;
    analytics::enqueue_client_lifecycle(
        &mut transaction,
        &session.workspace_id,
        &client_id,
        action,
        analytics_status,
        occurred_at,
    )
    .await
    .map_err(|error| {
        AppError::Internal(format!(
            "analytics client lifecycle enqueue failed: {error}"
        ))
    })?;
    let client_event_count = analytics::workspace_event_count(
        &mut transaction,
        &format!("workspace:{}:clients", session.workspace_id),
    )
    .await?;
    analytics::enqueue_client_manifest(
        &mut transaction,
        &session.workspace_id,
        client_event_count,
        occurred_at,
    )
    .await
    .map_err(|error| {
        AppError::Internal(format!("analytics client manifest enqueue failed: {error}"))
    })?;
    transaction.commit().await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "clientId": client_id,
        "status": requested_status,
        "changed": true
    })))
}

pub async fn list_client_users(
    req: HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let session = middleware::extract_tenant_user_session(&req).await?;
    ensure_client_management_roles(&session.roles)?;
    let pool = middleware::db_pool(&req)?;
    let tenant_client_id = path.into_inner();

    let users = sqlx::query_as::<_, TenantClientUserRow>(
        r#"
        SELECT
            id,
            tenant_client_id,
            email,
            name,
            status,
            created_at,
            updated_at
        FROM tenant_client_users
        WHERE workspace_id = $1 AND tenant_client_id = $2
        ORDER BY updated_at DESC, created_at DESC
        "#,
    )
    .bind(&session.workspace_id)
    .bind(&tenant_client_id)
    .fetch_all(pool)
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "workspaceId": session.workspace_id,
        "tenantClientId": tenant_client_id,
        "users": users
    })))
}

pub async fn invite_client_user(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<InviteTenantClientUserRequest>,
) -> Result<HttpResponse, AppError> {
    let session = middleware::extract_tenant_user_session(&req).await?;
    ensure_client_management_roles(&session.roles)?;
    let pool = middleware::db_pool(&req)?;
    let config = middleware::app_config(&req)?;
    let tenant_client_id = path.into_inner();
    let email = body.email.trim().to_ascii_lowercase();
    let role = body
        .role
        .as_deref()
        .unwrap_or("member")
        .trim()
        .to_ascii_lowercase();

    if email.is_empty() {
        return Err(AppError::BadRequest("email is required".to_string()));
    }

    let (tenant_client_user_id, invite) = provision_client_user_invite(
        pool,
        config,
        &session.workspace_id,
        &session.workspace_slug,
        &tenant_client_id,
        &email,
        body.name.as_deref(),
        &role,
        Some(&session.tenant_user_id),
    )
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "invite": {
            "id": invite.invite_id,
            "tenantClientId": tenant_client_id,
            "tenantClientUserId": tenant_client_user_id,
            "email": email,
            "role": role,
            "expiresAt": invite.expires_at,
            "activationUrl": invite.activation_url,
            "portalUrl": invite.portal_url,
            "emailSent": invite.email_sent,
        }
    })))
}

pub async fn list_client_assignments(
    req: HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let session = middleware::extract_tenant_user_session(&req).await?;
    ensure_client_management_roles(&session.roles)?;
    let pool = middleware::db_pool(&req)?;
    let tenant_client_id = path.into_inner();

    let client_exists = sqlx::query_scalar::<_, i64>(
        "SELECT 1 FROM tenant_clients WHERE id = $1 AND workspace_id = $2",
    )
    .bind(&tenant_client_id)
    .bind(&session.workspace_id)
    .fetch_optional(pool)
    .await?
    .is_some();
    if !client_exists {
        return Err(AppError::NotFound("tenant client not found".to_string()));
    }

    let assignments = sqlx::query_as::<_, TenantClientAssignmentRow>(
        r#"
        SELECT
            a.id,
            a.tenant_user_id,
            u.email,
            u.name,
            a.assignment_role,
            a.status,
            a.assigned_at,
            a.revoked_at,
            a.created_at,
            a.updated_at
        FROM tenant_client_assignments a
        JOIN tenant_users u ON u.id = a.tenant_user_id
        WHERE a.workspace_id = $1 AND a.tenant_client_id = $2
        ORDER BY a.status ASC, a.updated_at DESC, a.created_at DESC
        "#,
    )
    .bind(&session.workspace_id)
    .bind(&tenant_client_id)
    .fetch_all(pool)
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "workspaceId": session.workspace_id,
        "tenantClientId": tenant_client_id,
        "assignments": assignments,
    })))
}

pub async fn assign_client_user(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<CreateTenantClientAssignmentRequest>,
) -> Result<HttpResponse, AppError> {
    let session = middleware::extract_tenant_user_session(&req).await?;
    ensure_client_management_roles(&session.roles)?;
    let pool = middleware::db_pool(&req)?;
    let tenant_client_id = path.into_inner();
    let tenant_user_id = body.tenant_user_id.trim().to_string();
    if tenant_user_id.is_empty() {
        return Err(AppError::BadRequest("tenantUserId is required".to_string()));
    }
    let assignment_role = normalize_assignment_role(body.assignment_role.as_deref())?;
    let mut transaction = pool.begin().await?;

    let client_exists = sqlx::query_scalar::<_, i64>(
        "SELECT 1 FROM tenant_clients WHERE id = $1 AND workspace_id = $2 AND status <> 'archived'",
    )
    .bind(&tenant_client_id)
    .bind(&session.workspace_id)
    .fetch_optional(&mut *transaction)
    .await?
    .is_some();
    if !client_exists {
        return Err(AppError::NotFound(
            "active tenant client not found".to_string(),
        ));
    }

    let user_exists = sqlx::query_scalar::<_, i64>(
        "SELECT 1 FROM tenant_users WHERE id = $1 AND workspace_id = $2 AND status = 'active'",
    )
    .bind(&tenant_user_id)
    .bind(&session.workspace_id)
    .fetch_optional(&mut *transaction)
    .await?
    .is_some();
    if !user_exists {
        return Err(AppError::NotFound(
            "active workspace staff member not found".to_string(),
        ));
    }

    let existing = sqlx::query_as::<_, (String, String, String)>(
        r#"SELECT id, status, assignment_role
           FROM tenant_client_assignments
           WHERE workspace_id = $1 AND tenant_client_id = $2 AND tenant_user_id = $3
           FOR UPDATE"#,
    )
    .bind(&session.workspace_id)
    .bind(&tenant_client_id)
    .bind(&tenant_user_id)
    .fetch_optional(&mut *transaction)
    .await?;

    let (assignment_id, action, status) = match existing {
        None => {
            let assignment_id = cuid2::create_id();
            sqlx::query(
                r#"INSERT INTO tenant_client_assignments
                   (id, workspace_id, tenant_client_id, tenant_user_id, assignment_role, status)
                   VALUES ($1, $2, $3, $4, $5, 'active')"#,
            )
            .bind(&assignment_id)
            .bind(&session.workspace_id)
            .bind(&tenant_client_id)
            .bind(&tenant_user_id)
            .bind(&assignment_role)
            .execute(&mut *transaction)
            .await?;
            (assignment_id, "assigned", "active")
        }
        Some((assignment_id, current_status, current_role)) if current_status == "revoked" => {
            sqlx::query(
                r#"UPDATE tenant_client_assignments
                   SET assignment_role = $1, status = 'active', revoked_at = NULL,
                       assigned_at = NOW(), updated_at = NOW()
                   WHERE id = $2"#,
            )
            .bind(&assignment_role)
            .bind(&assignment_id)
            .execute(&mut *transaction)
            .await?;
            let _ = current_role;
            (assignment_id, "assigned", "active")
        }
        Some((_, _, current_role)) if current_role == assignment_role => {
            transaction.rollback().await?;
            return Ok(HttpResponse::Ok().json(serde_json::json!({
                "tenantClientId": tenant_client_id,
                "tenantUserId": tenant_user_id,
                "assignmentRole": assignment_role,
                "status": "active",
                "changed": false,
            })));
        }
        Some((assignment_id, _, _)) => {
            sqlx::query(
                "UPDATE tenant_client_assignments SET assignment_role = $1, updated_at = NOW() WHERE id = $2",
            )
            .bind(&assignment_role)
            .bind(&assignment_id)
            .execute(&mut *transaction)
            .await?;
            (assignment_id, "role_changed", "active")
        }
    };

    let history_id = cuid2::create_id();
    let occurred_at = chrono::Utc::now();
    sqlx::query(
        r#"INSERT INTO tenant_client_assignment_history
           (id, workspace_id, tenant_client_id, tenant_user_id, assignment_id, action,
            assignment_role, actor_tenant_user_id, occurred_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
    )
    .bind(&history_id)
    .bind(&session.workspace_id)
    .bind(&tenant_client_id)
    .bind(&tenant_user_id)
    .bind(&assignment_id)
    .bind(action)
    .bind(&assignment_role)
    .bind(&session.tenant_user_id)
    .bind(occurred_at)
    .execute(&mut *transaction)
    .await?;
    analytics::enqueue_client_assignment_lifecycle(
        &mut transaction,
        &session.workspace_id,
        &tenant_client_id,
        &tenant_user_id,
        &assignment_id,
        action,
        status,
        &assignment_role,
        &history_id,
        occurred_at,
    )
    .await
    .map_err(|error| AppError::Internal(format!("analytics assignment enqueue failed: {error}")))?;
    let assignment_event_count = analytics::workspace_event_count(
        &mut transaction,
        &format!("workspace:{}:assignments", session.workspace_id),
    )
    .await?;
    analytics::enqueue_assignment_manifest(
        &mut transaction,
        &session.workspace_id,
        assignment_event_count,
        occurred_at,
    )
    .await
    .map_err(|error| {
        AppError::Internal(format!(
            "analytics assignment manifest enqueue failed: {error}"
        ))
    })?;
    transaction.commit().await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "tenantClientId": tenant_client_id,
        "tenantUserId": tenant_user_id,
        "assignmentId": assignment_id,
        "action": action,
        "assignmentRole": assignment_role,
        "status": status,
        "changed": true,
    })))
}

pub async fn remove_client_assignment(
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, AppError> {
    let session = middleware::extract_tenant_user_session(&req).await?;
    ensure_client_management_roles(&session.roles)?;
    let pool = middleware::db_pool(&req)?;
    let (tenant_client_id, tenant_user_id) = path.into_inner();
    let mut transaction = pool.begin().await?;
    let existing = sqlx::query_as::<_, (String, String, String)>(
        r#"SELECT id, status, assignment_role
           FROM tenant_client_assignments
           WHERE workspace_id = $1 AND tenant_client_id = $2 AND tenant_user_id = $3
           FOR UPDATE"#,
    )
    .bind(&session.workspace_id)
    .bind(&tenant_client_id)
    .bind(&tenant_user_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| AppError::NotFound("client assignment not found".to_string()))?;
    if existing.1 == "revoked" {
        transaction.rollback().await?;
        return Ok(HttpResponse::Ok().json(serde_json::json!({
            "tenantClientId": tenant_client_id,
            "tenantUserId": tenant_user_id,
            "status": "revoked",
            "changed": false,
        })));
    }

    sqlx::query(
        "UPDATE tenant_client_assignments SET status = 'revoked', revoked_at = NOW(), updated_at = NOW() WHERE id = $1",
    )
    .bind(&existing.0)
    .execute(&mut *transaction)
    .await?;
    let history_id = cuid2::create_id();
    let occurred_at = chrono::Utc::now();
    sqlx::query(
        r#"INSERT INTO tenant_client_assignment_history
           (id, workspace_id, tenant_client_id, tenant_user_id, assignment_id, action,
            assignment_role, actor_tenant_user_id, occurred_at)
           VALUES ($1, $2, $3, $4, $5, 'unassigned', $6, $7, $8)"#,
    )
    .bind(&history_id)
    .bind(&session.workspace_id)
    .bind(&tenant_client_id)
    .bind(&tenant_user_id)
    .bind(&existing.0)
    .bind(&existing.2)
    .bind(&session.tenant_user_id)
    .bind(occurred_at)
    .execute(&mut *transaction)
    .await?;
    analytics::enqueue_client_assignment_lifecycle(
        &mut transaction,
        &session.workspace_id,
        &tenant_client_id,
        &tenant_user_id,
        &existing.0,
        "unassigned",
        "revoked",
        &existing.2,
        &history_id,
        occurred_at,
    )
    .await
    .map_err(|error| AppError::Internal(format!("analytics assignment enqueue failed: {error}")))?;
    let assignment_event_count = analytics::workspace_event_count(
        &mut transaction,
        &format!("workspace:{}:assignments", session.workspace_id),
    )
    .await?;
    analytics::enqueue_assignment_manifest(
        &mut transaction,
        &session.workspace_id,
        assignment_event_count,
        occurred_at,
    )
    .await
    .map_err(|error| {
        AppError::Internal(format!(
            "analytics assignment manifest enqueue failed: {error}"
        ))
    })?;
    transaction.commit().await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "tenantClientId": tenant_client_id,
        "tenantUserId": tenant_user_id,
        "assignmentId": existing.0,
        "action": "unassigned",
        "status": "revoked",
        "changed": true,
    })))
}
