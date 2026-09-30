use chrono::{DateTime, Utc};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::{config::AppConfig, error::AppError};

const RESEND_API_BASE_URL: &str = "https://api.resend.com";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceEmailSender {
    pub sending_mode: String,
    pub sender_name: String,
    pub from_email: String,
    pub reply_to_email: Option<String>,
    pub domain_status: String,
    pub active_domain_id: Option<String>,
    pub active_domain: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailDomainDnsRecord {
    pub id: String,
    pub record_group: String,
    pub record_type: String,
    pub record_name: String,
    pub record_value: String,
    pub record_status: String,
    pub ttl: Option<String>,
    pub priority: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct NewWorkspaceEmailDomain {
    pub provider_domain_id: String,
    pub domain: String,
    pub status: String,
    pub region: Option<String>,
    pub capabilities: serde_json::Value,
    pub verified_at: Option<DateTime<Utc>>,
    pub records: Vec<EmailDomainDnsRecord>,
}

#[derive(Debug, Deserialize)]
struct ResendDomainCapabilityState {
    sending: Option<String>,
    receiving: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResendDomainRecord {
    record: String,
    name: String,
    #[serde(rename = "type")]
    record_type: String,
    value: String,
    status: String,
    ttl: Option<String>,
    priority: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct ResendDomainResponse {
    id: String,
    name: String,
    status: String,
    region: Option<String>,
    capabilities: Option<ResendDomainCapabilityState>,
    records: Vec<ResendDomainRecord>,
    verified_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct ResendDomainEnvelope {
    data: ResendDomainResponse,
}

#[derive(Debug, Deserialize)]
struct ResendErrorEnvelope {
    message: Option<String>,
    error: Option<String>,
}

#[derive(Debug, FromRow)]
struct WorkspaceEmailSettingsRow {
    sending_mode: String,
    sender_name: Option<String>,
    reply_to_email: Option<String>,
    shared_sender_local_part: String,
    custom_domain_id: Option<String>,
}

#[derive(Debug, FromRow)]
struct WorkspaceEmailDomainRow {
    id: String,
    domain: String,
    status: String,
}

fn normalize_local_part(value: &str) -> String {
    let mut normalized = String::with_capacity(value.len());
    let mut previous_was_dash = false;

    for ch in value.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch);
            previous_was_dash = false;
        } else if matches!(ch, '.' | '_' | '-') && !previous_was_dash {
            normalized.push('-');
            previous_was_dash = true;
        }
    }

    normalized.trim_matches('-').to_string()
}

fn fallback_sender_name(workspace_display_name: &str) -> String {
    let trimmed = workspace_display_name.trim();
    if trimmed.is_empty() {
        "Tenant Workspace".to_string()
    } else {
        trimmed.to_string()
    }
}

fn shared_from_email(config: &AppConfig, workspace_slug: &str, shared_local_part: &str) -> String {
    let local_part = normalize_local_part(shared_local_part);
    let workspace_slug = normalize_local_part(workspace_slug);
    let workspace_fragment = if workspace_slug.is_empty() {
        "workspace".to_string()
    } else {
        workspace_slug
    };
    let sender_fragment = if local_part.is_empty() {
        "no-reply".to_string()
    } else {
        local_part
    };

    format!(
        "{}-{}@{}",
        workspace_fragment, sender_fragment, config.shared_email_domain
    )
}

fn parse_sender_email(value: &str) -> String {
    let trimmed = value.trim();
    if let (Some(start), Some(end)) = (trimmed.rfind('<'), trimmed.rfind('>')) {
        if start < end {
            return trimmed[start + 1..end].trim().to_string();
        }
    }
    trimmed.to_string()
}

fn url_for_host(host: &str, path: &str) -> String {
    let protocol = if host == "localhost"
        || host.ends_with(".local")
        || host.starts_with("127.")
        || host.chars().all(|ch| ch.is_ascii_digit() || ch == '.')
    {
        "http"
    } else {
        "https"
    };

    format!("{protocol}://{}{}", host.trim_matches('/'), path)
}

pub fn workspace_portal_url(config: &AppConfig, workspace_slug: &str) -> Option<String> {
    if let Some(base_domain) = config.platform_base_domain.as_ref() {
        let normalized = base_domain.trim().trim_matches('.').to_ascii_lowercase();
        if !normalized.is_empty() {
            return Some(url_for_host(&format!("{workspace_slug}.{normalized}"), ""));
        }
    }

    config
        .app_base_url
        .as_ref()
        .map(|url| url.trim_end_matches('/').to_string())
}

async fn resend_request<T: for<'de> Deserialize<'de>>(
    config: &AppConfig,
    method: reqwest::Method,
    path: &str,
    body: Option<serde_json::Value>,
) -> Result<T, AppError> {
    let Some(api_key) = config.resend_api_key.as_ref() else {
        return Err(AppError::BadRequest(
            "Resend is not configured for this environment".to_string(),
        ));
    };

    let client = reqwest::Client::new();
    let mut request = client
        .request(method, format!("{RESEND_API_BASE_URL}{path}"))
        .bearer_auth(api_key);

    if let Some(body) = body {
        request = request.json(&body);
    }

    let response = request.send().await?;
    if response.status().is_success() {
        return response.json::<T>().await.map_err(AppError::from);
    }

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let parsed = serde_json::from_str::<ResendErrorEnvelope>(&body).ok();
    let message = parsed
        .and_then(|payload| payload.message.or(payload.error))
        .unwrap_or_else(|| format!("Resend request failed with status {status}"));

    if matches!(
        status,
        StatusCode::BAD_REQUEST
            | StatusCode::UNAUTHORIZED
            | StatusCode::FORBIDDEN
            | StatusCode::NOT_FOUND
            | StatusCode::CONFLICT
            | StatusCode::UNPROCESSABLE_ENTITY
    ) {
        Err(AppError::BadRequest(message))
    } else {
        Err(AppError::Internal(message))
    }
}

fn map_domain_response(domain: ResendDomainResponse) -> NewWorkspaceEmailDomain {
    let capabilities = domain.capabilities.map_or_else(
        || serde_json::json!({}),
        |capabilities| {
            serde_json::json!({
                "sending": capabilities.sending,
                "receiving": capabilities.receiving,
            })
        },
    );

    NewWorkspaceEmailDomain {
        provider_domain_id: domain.id,
        domain: domain.name,
        status: domain.status.clone(),
        region: domain.region,
        capabilities,
        verified_at: domain.verified_at.or_else(|| {
            if domain.status == "verified" {
                Some(Utc::now())
            } else {
                None
            }
        }),
        records: domain
            .records
            .into_iter()
            .map(|record| EmailDomainDnsRecord {
                id: cuid2::create_id(),
                record_group: record.record,
                record_type: record.record_type,
                record_name: record.name,
                record_value: record.value,
                record_status: record.status,
                ttl: record.ttl,
                priority: record.priority,
            })
            .collect(),
    }
}

pub async fn create_resend_domain(
    config: &AppConfig,
    domain: &str,
) -> Result<NewWorkspaceEmailDomain, AppError> {
    let response = resend_request::<ResendDomainEnvelope>(
        config,
        reqwest::Method::POST,
        "/domains",
        Some(serde_json::json!({
            "name": domain,
            "region": "eu-west-1",
        })),
    )
    .await?;

    Ok(map_domain_response(response.data))
}

pub async fn retrieve_resend_domain(
    config: &AppConfig,
    provider_domain_id: &str,
) -> Result<NewWorkspaceEmailDomain, AppError> {
    let response = resend_request::<ResendDomainEnvelope>(
        config,
        reqwest::Method::GET,
        &format!("/domains/{provider_domain_id}"),
        None,
    )
    .await?;

    Ok(map_domain_response(response.data))
}

pub async fn verify_resend_domain(
    config: &AppConfig,
    provider_domain_id: &str,
) -> Result<NewWorkspaceEmailDomain, AppError> {
    let _: serde_json::Value = resend_request(
        config,
        reqwest::Method::POST,
        &format!("/domains/{provider_domain_id}/verify"),
        None,
    )
    .await?;

    retrieve_resend_domain(config, provider_domain_id).await
}

pub async fn delete_resend_domain(
    config: &AppConfig,
    provider_domain_id: &str,
) -> Result<(), AppError> {
    let _: serde_json::Value = resend_request(
        config,
        reqwest::Method::DELETE,
        &format!("/domains/{provider_domain_id}"),
        None,
    )
    .await?;

    Ok(())
}

pub async fn persist_workspace_email_domain(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workspace_id: &str,
    existing_domain_id: Option<&str>,
    domain: NewWorkspaceEmailDomain,
) -> Result<String, AppError> {
    let domain_id = existing_domain_id
        .map(ToString::to_string)
        .unwrap_or_else(cuid2::create_id);

    sqlx::query(
        r#"
        INSERT INTO workspace_email_domains (
            id,
            workspace_id,
            provider,
            provider_domain_id,
            domain,
            status,
            region,
            capabilities,
            verified_at,
            last_synced_at,
            created_at,
            updated_at
        )
        VALUES ($1, $2, 'resend', $3, $4, $5, $6, $7, $8, NOW(), NOW(), NOW())
        ON CONFLICT (id) DO UPDATE
        SET provider_domain_id = EXCLUDED.provider_domain_id,
            domain = EXCLUDED.domain,
            status = EXCLUDED.status,
            region = EXCLUDED.region,
            capabilities = EXCLUDED.capabilities,
            verified_at = EXCLUDED.verified_at,
            last_synced_at = NOW(),
            updated_at = NOW()
        "#,
    )
    .bind(&domain_id)
    .bind(workspace_id)
    .bind(&domain.provider_domain_id)
    .bind(&domain.domain)
    .bind(&domain.status)
    .bind(&domain.region)
    .bind(&domain.capabilities)
    .bind(domain.verified_at)
    .execute(&mut **tx)
    .await?;

    sqlx::query(r#"DELETE FROM workspace_email_domain_records WHERE email_domain_id = $1"#)
        .bind(&domain_id)
        .execute(&mut **tx)
        .await?;

    for record in &domain.records {
        sqlx::query(
            r#"
            INSERT INTO workspace_email_domain_records (
                id,
                email_domain_id,
                record_group,
                record_type,
                record_name,
                record_value,
                record_status,
                ttl,
                priority,
                created_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW(), NOW())
            "#,
        )
        .bind(&record.id)
        .bind(&domain_id)
        .bind(&record.record_group)
        .bind(&record.record_type)
        .bind(&record.record_name)
        .bind(&record.record_value)
        .bind(&record.record_status)
        .bind(&record.ttl)
        .bind(record.priority)
        .execute(&mut **tx)
        .await?;
    }

    Ok(domain.status)
}

pub async fn resolve_workspace_email_sender(
    pool: &sqlx::PgPool,
    config: &AppConfig,
    workspace_id: &str,
    workspace_slug: &str,
    workspace_display_name: &str,
) -> Result<WorkspaceEmailSender, AppError> {
    let settings = sqlx::query_as::<_, WorkspaceEmailSettingsRow>(
        r#"
        SELECT
            workspace_id,
            sending_mode,
            sender_name,
            reply_to_email,
            shared_sender_local_part,
            custom_domain_id
        FROM workspace_email_settings
        WHERE workspace_id = $1
        LIMIT 1
        "#,
    )
    .bind(workspace_id)
    .fetch_optional(pool)
    .await?;

    let settings = if let Some(settings) = settings {
        settings
    } else {
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

        WorkspaceEmailSettingsRow {
            sending_mode: "shared".to_string(),
            sender_name: Some(workspace_display_name.to_string()),
            reply_to_email: None,
            shared_sender_local_part: "no-reply".to_string(),
            custom_domain_id: None,
        }
    };

    let sender_name = settings
        .sender_name
        .as_deref()
        .map(fallback_sender_name)
        .unwrap_or_else(|| fallback_sender_name(workspace_display_name));

    if settings.sending_mode == "custom" {
        if let Some(custom_domain_id) = settings.custom_domain_id.as_ref() {
            let custom_domain = sqlx::query_as::<_, WorkspaceEmailDomainRow>(
                r#"
                SELECT
                    id,
                    domain,
                    status,
                    verified_at AT TIME ZONE 'UTC' AS verified_at
                FROM workspace_email_domains
                WHERE workspace_id = $1 AND id = $2
                LIMIT 1
                "#,
            )
            .bind(workspace_id)
            .bind(custom_domain_id)
            .fetch_optional(pool)
            .await?;

            if let Some(custom_domain) = custom_domain.filter(|domain| domain.status == "verified")
            {
                let local_part = normalize_local_part(&settings.shared_sender_local_part);
                let sender_local_part = if local_part.is_empty() {
                    "no-reply".to_string()
                } else {
                    local_part
                };

                return Ok(WorkspaceEmailSender {
                    sending_mode: "custom".to_string(),
                    sender_name,
                    from_email: format!("{sender_local_part}@{}", custom_domain.domain),
                    reply_to_email: settings.reply_to_email,
                    domain_status: custom_domain.status,
                    active_domain_id: Some(custom_domain.id),
                    active_domain: Some(custom_domain.domain),
                });
            }
        }
    }

    Ok(WorkspaceEmailSender {
        sending_mode: "shared".to_string(),
        sender_name,
        from_email: shared_from_email(config, workspace_slug, &settings.shared_sender_local_part),
        reply_to_email: settings.reply_to_email,
        domain_status: "shared".to_string(),
        active_domain_id: None,
        active_domain: Some(config.shared_email_domain.clone()),
    })
}

pub async fn send_email(
    config: &AppConfig,
    sender: &WorkspaceEmailSender,
    to: &[String],
    subject: &str,
    text: &str,
    html: &str,
) -> Result<(), AppError> {
    let Some(api_key) = config.resend_api_key.as_ref() else {
        return Err(AppError::BadRequest(
            "Resend is not configured for this environment".to_string(),
        ));
    };

    let response = reqwest::Client::new()
        .post(format!("{RESEND_API_BASE_URL}/emails"))
        .bearer_auth(api_key)
        .json(&serde_json::json!({
            "from": format!("{} <{}>", sender.sender_name, sender.from_email),
            "to": to,
            "reply_to": sender.reply_to_email.as_deref(),
            "subject": subject,
            "text": text,
            "html": html,
        }))
        .send()
        .await?;

    if response.status() != StatusCode::OK && response.status() != StatusCode::CREATED {
        observability::tracing::error!(
            event = "provider.call.failed",
            provider = "resend",
            operation = "workspace.email.send",
            status = response.status().as_u16(),
            "email provider rejected workspace email"
        );
        return Err(AppError::Internal("Failed to send email".to_string()));
    }

    Ok(())
}

pub async fn send_client_invite_email(
    pool: &sqlx::PgPool,
    config: &AppConfig,
    workspace_id: &str,
    workspace_slug: &str,
    workspace_name: &str,
    to: &str,
    _tenant_client_name: &str,
    activation_url: &str,
    portal_url: Option<&str>,
    expires_at: DateTime<Utc>,
) -> Result<bool, AppError> {
    if config.resend_api_key.is_none() {
        observability::tracing::warn!(
            event = "dependency.unavailable",
            dependency = "resend",
            operation = "tenant_invite.send",
            reason = "missing_configuration",
            "tenant invite email not sent"
        );
        return Ok(false);
    }

    let sender =
        resolve_workspace_email_sender(pool, config, workspace_id, workspace_slug, workspace_name)
            .await?;

    let expires_label = expires_at.format("%Y-%m-%d %H:%M UTC");
    let subject = format!("Verify your email for {workspace_name}");
    let portal_line = portal_url
        .map(|url| format!("Portal link: {url}\n"))
        .unwrap_or_default();
    let text = format!(
        "Verify your email to finish setting up your access.\n\nOpen the link below to confirm this address and create your password.\n\nVerify link:\n{activation_url}\n\n{portal_line}What happens next:\n1. Verify your email\n2. Create your password\n3. Sign in and start using the portal\n\nThis link expires on {expires_label}.\n"
    );
    let portal_html = portal_url
        .map(|url| format!("<p><strong>Portal link:</strong> <a href=\"{url}\">{url}</a></p>"))
        .unwrap_or_default();
    let html = format!(
        "<p><strong>Verify your email to finish setting up your access.</strong></p><p>Open the link below to confirm this address and create your password.</p><p><a href=\"{activation_url}\">Verify email</a></p>{portal_html}<p><strong>What happens next:</strong></p><ol><li>Verify your email</li><li>Create your password</li><li>Sign in and start using the portal</li></ol><p>This link expires on {expires_label}.</p>"
    );

    send_email(config, &sender, &[to.to_string()], &subject, &text, &html).await?;
    Ok(true)
}

pub fn platform_from_email(config: &AppConfig) -> String {
    parse_sender_email(&config.resend_from_email)
}

pub async fn send_tenant_admin_invite_email(
    config: &AppConfig,
    to: &str,
    workspace_name: &str,
    activation_url: &str,
    portal_url: Option<&str>,
    expires_at: DateTime<Utc>,
) -> Result<bool, AppError> {
    let Some(api_key) = config.resend_api_key.as_ref() else {
        observability::tracing::warn!(
            event = "dependency.unavailable",
            dependency = "resend",
            operation = "tenant_admin_invite.send",
            reason = "missing_configuration",
            "tenant admin invite email not sent"
        );
        return Ok(false);
    };

    let expires_label = expires_at.format("%Y-%m-%d %H:%M UTC");
    let subject = format!("Verify your email to access {workspace_name}");
    let portal_line = portal_url
        .map(|url| format!("Portal link: {url}\n"))
        .unwrap_or_default();
    let text = format!(
        "Verify your email to finish setting up your admin access.\n\nOpen the link below to confirm this address and create your password.\n\nVerify link:\n{activation_url}\n\n{portal_line}What happens next:\n1. Verify your email\n2. Create your password\n3. Sign in and complete portal setup\n4. Start onboarding clients\n\nThis link expires on {expires_label}.\n"
    );
    let portal_html = portal_url
        .map(|url| format!("<p><strong>Portal link:</strong> <a href=\"{url}\">{url}</a></p>"))
        .unwrap_or_default();
    let html = format!(
        "<p><strong>Verify your email to finish setting up your admin access.</strong></p><p>Open the link below to confirm this address and create your password.</p><p><a href=\"{activation_url}\">Verify email</a></p>{portal_html}<p><strong>What happens next:</strong></p><ol><li>Verify your email</li><li>Create your password</li><li>Sign in and complete portal setup</li><li>Start onboarding clients</li></ol><p>This link expires on {expires_label}.</p>"
    );

    let response = reqwest::Client::new()
        .post(format!("{RESEND_API_BASE_URL}/emails"))
        .bearer_auth(api_key)
        .json(&serde_json::json!({
            "from": config.resend_from_email,
            "to": [to],
            "subject": subject,
            "text": text,
            "html": html,
        }))
        .send()
        .await?;

    if response.status() != StatusCode::OK && response.status() != StatusCode::CREATED {
        observability::tracing::error!(
            event = "provider.call.failed",
            provider = "resend",
            operation = "tenant_admin_invite.send",
            status = response.status().as_u16(),
            "email provider rejected tenant admin invite"
        );
        return Err(AppError::Internal(
            "Failed to send tenant admin invite email".to_string(),
        ));
    }

    Ok(true)
}
