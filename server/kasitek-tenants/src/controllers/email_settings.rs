use actix_web::{web, HttpRequest, HttpResponse};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::{error::AppError, mail, middleware};

#[derive(Debug, FromRow)]
struct WorkspaceEmailSettingsRow {
    workspace_id: String,
    sending_mode: String,
    sender_name: Option<String>,
    reply_to_email: Option<String>,
    shared_sender_local_part: String,
    custom_domain_id: Option<String>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct WorkspaceEmailDomainRow {
    id: String,
    provider: String,
    provider_domain_id: String,
    domain: String,
    status: String,
    verified_at: Option<chrono::DateTime<Utc>>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct WorkspaceEmailDomainRecordRow {
    id: String,
    email_domain_id: String,
    record_group: String,
    record_type: String,
    record_name: String,
    record_value: String,
    record_status: String,
    ttl: Option<String>,
    priority: Option<i32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EmailDomainResponse {
    id: String,
    provider: String,
    provider_domain_id: String,
    domain: String,
    status: String,
    verified_at: Option<chrono::DateTime<Utc>>,
    created_at: chrono::DateTime<Utc>,
    updated_at: chrono::DateTime<Utc>,
    records: Vec<mail::EmailDomainDnsRecord>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateEmailSettingsRequest {
    sending_mode: String,
    sender_name: Option<String>,
    reply_to_email: Option<String>,
    shared_sender_local_part: Option<String>,
    custom_domain_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateEmailDomainRequest {
    domain: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestEmailRequest {
    to: String,
}

fn require_workspace_admin(access: &crate::types::TenantAccessContext) -> Result<(), AppError> {
    middleware::require_tenant_roles(&access.tenant_user_roles, &["owner", "admin"])
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

fn normalize_local_part(value: Option<&str>) -> String {
    let mut normalized = String::new();
    let mut previous_was_dash = false;

    for ch in value
        .unwrap_or("no-reply")
        .chars()
        .flat_map(char::to_lowercase)
    {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch);
            previous_was_dash = false;
        } else if matches!(ch, '.' | '_' | '-') && !previous_was_dash {
            normalized.push('-');
            previous_was_dash = true;
        }
    }

    let trimmed = normalized.trim_matches('-');
    if trimmed.is_empty() {
        "no-reply".to_string()
    } else {
        trimmed.to_string()
    }
}

fn normalize_optional_email(value: Option<&str>) -> Result<Option<String>, AppError> {
    let value = value.map(str::trim).filter(|value| !value.is_empty());
    if let Some(value) = value {
        if !value.contains('@') || value.contains(' ') {
            return Err(AppError::BadRequest(
                "reply-to email must be valid".to_string(),
            ));
        }
        Ok(Some(value.to_ascii_lowercase()))
    } else {
        Ok(None)
    }
}

async fn ensure_workspace_email_settings(
    pool: &sqlx::PgPool,
    workspace_id: &str,
    workspace_display_name: &str,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO workspace_email_settings (
            workspace_id,
            sender_name,
            created_at,
            updated_at
        )
        VALUES ($1, $2, NOW(), NOW())
        ON CONFLICT (workspace_id) DO NOTHING
        "#,
    )
    .bind(workspace_id)
    .bind(workspace_display_name)
    .execute(pool)
    .await?;

    Ok(())
}

async fn workspace_display_name(
    pool: &sqlx::PgPool,
    workspace_id: &str,
) -> Result<String, AppError> {
    sqlx::query_scalar::<_, String>(r#"SELECT display_name FROM workspaces WHERE id = $1 LIMIT 1"#)
        .bind(workspace_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("workspace not found".to_string()))
}

async fn load_custom_domains(
    pool: &sqlx::PgPool,
    workspace_id: &str,
) -> Result<Vec<EmailDomainResponse>, AppError> {
    let domains = sqlx::query_as::<_, WorkspaceEmailDomainRow>(
        r#"
        SELECT
            id,
            provider,
            provider_domain_id,
            domain,
            status,
            verified_at AS verified_at,
            created_at AS created_at,
            updated_at AS updated_at
        FROM workspace_email_domains
        WHERE workspace_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await?;

    let records = sqlx::query_as::<_, WorkspaceEmailDomainRecordRow>(
        r#"
        SELECT
            id,
            email_domain_id,
            record_group,
            record_type,
            record_name,
            record_value,
            record_status,
            ttl,
            priority
        FROM workspace_email_domain_records
        WHERE email_domain_id IN (
            SELECT id FROM workspace_email_domains WHERE workspace_id = $1
        )
        ORDER BY record_group ASC, record_name ASC
        "#,
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await?;

    Ok(domains
        .into_iter()
        .map(|domain| EmailDomainResponse {
            id: domain.id.clone(),
            provider: domain.provider,
            provider_domain_id: domain.provider_domain_id,
            domain: domain.domain,
            status: domain.status,
            verified_at: domain.verified_at,
            created_at: domain.created_at,
            updated_at: domain.updated_at,
            records: records
                .iter()
                .filter(|record| record.email_domain_id == domain.id)
                .map(|record| mail::EmailDomainDnsRecord {
                    id: record.id.clone(),
                    record_group: record.record_group.clone(),
                    record_type: record.record_type.clone(),
                    record_name: record.record_name.clone(),
                    record_value: record.record_value.clone(),
                    record_status: record.record_status.clone(),
                    ttl: record.ttl.clone(),
                    priority: record.priority,
                })
                .collect(),
        })
        .collect())
}

async fn load_response(
    pool: &sqlx::PgPool,
    config: &crate::config::AppConfig,
    workspace_id: &str,
    workspace_slug: &str,
) -> Result<HttpResponse, AppError> {
    let display_name = workspace_display_name(pool, workspace_id).await?;
    ensure_workspace_email_settings(pool, workspace_id, &display_name).await?;

    let settings = sqlx::query_as::<_, WorkspaceEmailSettingsRow>(
        r#"
        SELECT
            workspace_id,
            sending_mode,
            sender_name,
            reply_to_email,
            shared_sender_local_part,
            custom_domain_id,
            created_at AS created_at,
            updated_at AS updated_at
        FROM workspace_email_settings
        WHERE workspace_id = $1
        LIMIT 1
        "#,
    )
    .bind(workspace_id)
    .fetch_one(pool)
    .await?;

    let sender = mail::resolve_workspace_email_sender(
        pool,
        config,
        workspace_id,
        workspace_slug,
        &display_name,
    )
    .await?;
    let custom_domains = load_custom_domains(pool, workspace_id).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "email": {
            "resendConfigured": config.resend_api_key.is_some(),
            "sharedDomain": config.shared_email_domain,
            "platformFromEmail": mail::platform_from_email(config),
            "sender": sender,
            "settings": {
                "workspaceId": settings.workspace_id,
                "sendingMode": settings.sending_mode,
                "senderName": settings.sender_name,
                "replyToEmail": settings.reply_to_email,
                "sharedSenderLocalPart": settings.shared_sender_local_part,
                "customDomainId": settings.custom_domain_id,
                "createdAt": settings.created_at,
                "updatedAt": settings.updated_at,
            },
            "customDomains": custom_domains,
        }
    })))
}

pub async fn get_email_settings(req: HttpRequest) -> Result<HttpResponse, AppError> {
    let access = middleware::extract_access_context(&req).await?;
    require_workspace_admin(&access)?;
    let pool = middleware::db_pool(&req)?;
    let config = middleware::app_config(&req)?;

    load_response(pool, config, &access.workspace_id, &access.workspace_slug).await
}

pub async fn update_email_settings(
    req: HttpRequest,
    body: web::Json<UpdateEmailSettingsRequest>,
) -> Result<HttpResponse, AppError> {
    let access = middleware::extract_access_context(&req).await?;
    require_workspace_admin(&access)?;
    let pool = middleware::db_pool(&req)?;
    let config = middleware::app_config(&req)?;
    let display_name = workspace_display_name(pool, &access.workspace_id).await?;
    ensure_workspace_email_settings(pool, &access.workspace_id, &display_name).await?;

    let sending_mode = body.sending_mode.trim().to_ascii_lowercase();
    if !matches!(sending_mode.as_str(), "shared" | "custom") {
        return Err(AppError::BadRequest(
            "sending mode must be shared or custom".to_string(),
        ));
    }

    let custom_domain_id = body
        .custom_domain_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    if sending_mode == "custom" {
        let Some(custom_domain_id) = custom_domain_id.as_ref() else {
            return Err(AppError::BadRequest(
                "select a verified custom domain before switching to custom sending".to_string(),
            ));
        };

        let custom_domain_status = sqlx::query_scalar::<_, String>(
            r#"
            SELECT status
            FROM workspace_email_domains
            WHERE workspace_id = $1 AND id = $2
            LIMIT 1
            "#,
        )
        .bind(&access.workspace_id)
        .bind(custom_domain_id)
        .fetch_optional(pool)
        .await?;

        if custom_domain_status.as_deref() != Some("verified") {
            return Err(AppError::BadRequest(
                "custom sending requires a verified domain".to_string(),
            ));
        }
    }

    sqlx::query(
        r#"
        UPDATE workspace_email_settings
        SET sending_mode = $1,
            sender_name = $2,
            reply_to_email = $3,
            shared_sender_local_part = $4,
            custom_domain_id = $5,
            updated_at = NOW()
        WHERE workspace_id = $6
        "#,
    )
    .bind(&sending_mode)
    .bind(
        body.sender_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string),
    )
    .bind(normalize_optional_email(body.reply_to_email.as_deref())?)
    .bind(normalize_local_part(
        body.shared_sender_local_part.as_deref(),
    ))
    .bind(custom_domain_id)
    .bind(&access.workspace_id)
    .execute(pool)
    .await?;

    load_response(pool, config, &access.workspace_id, &access.workspace_slug).await
}

pub async fn create_custom_email_domain(
    req: HttpRequest,
    body: web::Json<CreateEmailDomainRequest>,
) -> Result<HttpResponse, AppError> {
    let access = middleware::extract_access_context(&req).await?;
    require_workspace_admin(&access)?;
    let pool = middleware::db_pool(&req)?;
    let config = middleware::app_config(&req)?;
    let domain = normalize_domain(&body.domain)?;

    let existing = sqlx::query_scalar::<_, String>(
        r#"SELECT id FROM workspace_email_domains WHERE lower(domain) = lower($1) LIMIT 1"#,
    )
    .bind(&domain)
    .fetch_optional(pool)
    .await?;

    if existing.is_some() {
        return Err(AppError::BadRequest(
            "email domain already exists".to_string(),
        ));
    }

    let resend_domain = mail::create_resend_domain(config, &domain).await?;
    let mut tx = pool.begin().await?;
    let _ =
        mail::persist_workspace_email_domain(&mut tx, &access.workspace_id, None, resend_domain)
            .await?;
    tx.commit().await?;

    load_response(pool, config, &access.workspace_id, &access.workspace_slug).await
}

pub async fn verify_custom_email_domain(
    req: HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let access = middleware::extract_access_context(&req).await?;
    require_workspace_admin(&access)?;
    let pool = middleware::db_pool(&req)?;
    let config = middleware::app_config(&req)?;
    let domain_id = path.into_inner();

    let domain = sqlx::query_as::<_, WorkspaceEmailDomainRow>(
        r#"
        SELECT
            id,
            provider,
            provider_domain_id,
            domain,
            status,
            verified_at AS verified_at,
            created_at AS created_at,
            updated_at AS updated_at
        FROM workspace_email_domains
        WHERE workspace_id = $1 AND id = $2
        LIMIT 1
        "#,
    )
    .bind(&access.workspace_id)
    .bind(&domain_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("email domain not found".to_string()))?;

    let resend_domain = mail::verify_resend_domain(config, &domain.provider_domain_id).await?;
    let mut tx = pool.begin().await?;
    let resolved_status = mail::persist_workspace_email_domain(
        &mut tx,
        &access.workspace_id,
        Some(&domain.id),
        resend_domain,
    )
    .await?;

    if resolved_status != "verified" {
        sqlx::query(
            r#"
            UPDATE workspace_email_settings
            SET sending_mode = 'shared',
                updated_at = NOW()
            WHERE workspace_id = $1
              AND custom_domain_id = $2
              AND sending_mode = 'custom'
            "#,
        )
        .bind(&access.workspace_id)
        .bind(&domain.id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    load_response(pool, config, &access.workspace_id, &access.workspace_slug).await
}

pub async fn remove_custom_email_domain(
    req: HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse, AppError> {
    let access = middleware::extract_access_context(&req).await?;
    require_workspace_admin(&access)?;
    let pool = middleware::db_pool(&req)?;
    let config = middleware::app_config(&req)?;
    let domain_id = path.into_inner();

    let domain = sqlx::query_as::<_, WorkspaceEmailDomainRow>(
        r#"
        SELECT
            id,
            provider,
            provider_domain_id,
            domain,
            status,
            verified_at AS verified_at,
            created_at AS created_at,
            updated_at AS updated_at
        FROM workspace_email_domains
        WHERE workspace_id = $1 AND id = $2
        LIMIT 1
        "#,
    )
    .bind(&access.workspace_id)
    .bind(&domain_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("email domain not found".to_string()))?;

    mail::delete_resend_domain(config, &domain.provider_domain_id).await?;

    let mut tx = pool.begin().await?;
    sqlx::query(
        r#"
        UPDATE workspace_email_settings
        SET sending_mode = CASE WHEN custom_domain_id = $2 THEN 'shared' ELSE sending_mode END,
            custom_domain_id = CASE WHEN custom_domain_id = $2 THEN NULL ELSE custom_domain_id END,
            updated_at = NOW()
        WHERE workspace_id = $1
        "#,
    )
    .bind(&access.workspace_id)
    .bind(&domain.id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(r#"DELETE FROM workspace_email_domains WHERE workspace_id = $1 AND id = $2"#)
        .bind(&access.workspace_id)
        .bind(&domain.id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    load_response(pool, config, &access.workspace_id, &access.workspace_slug).await
}

pub async fn send_test_email(
    req: HttpRequest,
    body: web::Json<TestEmailRequest>,
) -> Result<HttpResponse, AppError> {
    let access = middleware::extract_access_context(&req).await?;
    require_workspace_admin(&access)?;
    let pool = middleware::db_pool(&req)?;
    let config = middleware::app_config(&req)?;
    let display_name = workspace_display_name(pool, &access.workspace_id).await?;
    let sender = mail::resolve_workspace_email_sender(
        pool,
        config,
        &access.workspace_id,
        &access.workspace_slug,
        &display_name,
    )
    .await?;

    let to = body.to.trim().to_ascii_lowercase();
    if !to.contains('@') || to.contains(' ') {
        return Err(AppError::BadRequest(
            "recipient email must be valid".to_string(),
        ));
    }

    let subject = format!("KasiTek email sending test for {}", display_name);
    let text = format!(
        "This is a test email from {}.\n\nCurrent sender: {} <{}>\nMode: {}\n",
        display_name, sender.sender_name, sender.from_email, sender.sending_mode
    );
    let html = format!(
        "<p>This is a test email from <strong>{}</strong>.</p><p>Current sender: <strong>{}</strong> &lt;{}&gt;<br/>Mode: {}</p>",
        display_name, sender.sender_name, sender.from_email, sender.sending_mode
    );

    mail::send_email(config, &sender, &[to.clone()], &subject, &text, &html).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "sent": true,
        "to": to,
        "sender": sender,
    })))
}
