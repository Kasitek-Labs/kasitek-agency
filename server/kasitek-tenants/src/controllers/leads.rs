use actix_web::{web, HttpRequest, HttpResponse};
use chrono::{Datelike, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::{analytics, error::AppError, middleware};

#[derive(Debug, Deserialize)]
pub struct CreateLeadRequest {
    #[serde(rename = "tenantClientId")]
    pub tenant_client_id: String,
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub company: Option<String>,
    pub source: String,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLeadStatusRequest {
    pub status: String,
    #[serde(rename = "reasonCode")]
    pub reason_code: Option<String>,
}

#[derive(Debug, FromRow, Serialize)]
struct LeadRow {
    id: String,
    tenant_client_id: String,
    tenant_client_name: String,
    name: String,
    email: String,
    phone: Option<String>,
    company: Option<String>,
    source: String,
    status: String,
    score: i32,
    notes: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, FromRow)]
struct LeadStatsRow {
    total_leads: i64,
    qualified_leads: i64,
    converted_leads: i64,
}

#[derive(Debug, Serialize)]
struct WeeklyTrendPoint {
    date: String,
    count: i64,
}

#[derive(Debug, FromRow, Serialize)]
struct StatusCount {
    status: String,
    count: i64,
}

fn ensure_lead_write_roles(roles: &[String]) -> Result<(), AppError> {
    middleware::require_tenant_roles(
        roles,
        &[
            "owner",
            "admin",
            "account_manager",
            "sales_manager",
            "support",
        ],
    )
}

fn normalize_optional(value: &Option<String>) -> Option<&str> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn normalize_source(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn calculate_score(source: &str, has_phone: bool, has_company: bool, notes: bool) -> i32 {
    let mut score = match source {
        "voice" => 85,
        "whatsapp" => 78,
        "instagram" => 64,
        "web_chat" => 72,
        "email" => 60,
        _ => 50,
    };

    if has_phone {
        score += 10;
    }

    if has_company {
        score += 8;
    }

    if notes {
        score += 4;
    }

    score.clamp(0, 100)
}

fn analytics_source(source: &str) -> &'static str {
    match source {
        "voice" => "voice",
        "whatsapp" => "whatsapp",
        "instagram" => "instagram",
        "web_chat" => "web_chat",
        "email" => "email",
        "manual" => "manual",
        _ => "other",
    }
}

fn score_band(score: i32) -> &'static str {
    match score {
        0..=49 => "low",
        50..=74 => "medium",
        _ => "high",
    }
}

fn normalize_lead_status(value: &str) -> Result<&'static str, AppError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "new" => Ok("new"),
        "qualified" => Ok("qualified"),
        "converted" => Ok("converted"),
        "lost" => Ok("lost"),
        "disqualified" => Ok("disqualified"),
        _ => Err(AppError::BadRequest(
            "status must be new, qualified, converted, lost, or disqualified".to_string(),
        )),
    }
}

fn normalize_lead_reason(status: &str, value: Option<&str>) -> Result<&'static str, AppError> {
    let default = match status {
        "new" => "reopened",
        "qualified" => "qualified",
        "converted" => "converted",
        "lost" => "lost",
        "disqualified" => "disqualified",
        _ => "manual_update",
    };
    let reason = value.unwrap_or(default).trim().to_ascii_lowercase();
    match reason.as_str() {
        "qualified" | "converted" | "lost" | "disqualified" | "reopened" | "manual_update" => {
            Ok(match reason.as_str() {
                "qualified" => "qualified",
                "converted" => "converted",
                "lost" => "lost",
                "disqualified" => "disqualified",
                "reopened" => "reopened",
                _ => "manual_update",
            })
        }
        _ => Err(AppError::BadRequest(
            "reasonCode must be a bounded lead status reason".to_string(),
        )),
    }
}

async fn change_lead_status(
    pool: &sqlx::PgPool,
    workspace_id: &str,
    tenant_client_id: Option<&str>,
    lead_id: &str,
    requested_status: &str,
    reason_code: Option<&str>,
    actor_type: &str,
    tenant_user_id: Option<&str>,
    client_user_id: Option<&str>,
) -> Result<(String, String, bool), AppError> {
    let status = normalize_lead_status(requested_status)?;
    let reason_code = normalize_lead_reason(status, reason_code)?;
    let mut transaction = pool.begin().await?;
    let lead = sqlx::query_as::<_, (String, String, String, i32)>(
        r#"SELECT status, tenant_client_id, source, score
           FROM tenant_leads
           WHERE id = $1 AND workspace_id = $2
             AND ($3::TEXT IS NULL OR tenant_client_id = $3)
           FOR UPDATE"#,
    )
    .bind(lead_id)
    .bind(workspace_id)
    .bind(tenant_client_id)
    .fetch_optional(&mut *transaction)
    .await?
    .ok_or_else(|| AppError::NotFound("lead not found".to_string()))?;
    if lead.0 == status {
        transaction.rollback().await?;
        return Ok((lead.1, status.to_string(), false));
    }
    let history_id = cuid2::create_id();
    let occurred_at = Utc::now();
    sqlx::query(
        r#"INSERT INTO tenant_lead_status_history (
             id, workspace_id, tenant_client_id, lead_id, from_status, to_status,
             reason_code, actor_type, changed_by_tenant_user_id, changed_by_client_user_id, occurred_at
           ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)"#,
    )
    .bind(&history_id)
    .bind(workspace_id)
    .bind(&lead.1)
    .bind(lead_id)
    .bind(&lead.0)
    .bind(status)
    .bind(reason_code)
    .bind(actor_type)
    .bind(tenant_user_id)
    .bind(client_user_id)
    .bind(occurred_at)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        r#"UPDATE tenant_leads SET status = $1, updated_at = $2
           WHERE id = $3 AND workspace_id = $4"#,
    )
    .bind(status)
    .bind(occurred_at)
    .bind(lead_id)
    .bind(workspace_id)
    .execute(&mut *transaction)
    .await?;
    analytics::enqueue_lead_status_changed(
        &mut transaction,
        workspace_id,
        &lead.1,
        lead_id,
        &lead.0,
        status,
        reason_code,
        actor_type,
        analytics_source(&lead.2),
        score_band(lead.3),
        &history_id,
        occurred_at,
    )
    .await
    .map_err(|error| {
        AppError::Internal(format!("analytics lead status enqueue failed: {error}"))
    })?;
    let lead_event_count = analytics::workspace_event_count(
        &mut transaction,
        &format!("workspace:{workspace_id}:leads"),
    )
    .await?;
    analytics::enqueue_lead_manifest(
        &mut transaction,
        workspace_id,
        lead_event_count,
        occurred_at,
    )
    .await
    .map_err(|error| {
        AppError::Internal(format!("analytics lead manifest enqueue failed: {error}"))
    })?;
    transaction.commit().await?;
    Ok((lead.1, status.to_string(), true))
}

pub async fn list_leads(req: HttpRequest) -> Result<HttpResponse, AppError> {
    let session = middleware::extract_tenant_user_session(&req).await?;
    middleware::require_tenant_roles(
        &session.roles,
        &[
            "owner",
            "admin",
            "account_manager",
            "sales_manager",
            "support",
        ],
    )?;
    let pool = middleware::db_pool(&req)?;

    let leads = sqlx::query_as::<_, LeadRow>(
        r#"
        SELECT
            l.id,
            l.tenant_client_id,
            c.name AS tenant_client_name,
            l.name,
            l.email,
            l.phone,
            l.company,
            l.source,
            l.status,
            l.score,
            l.notes,
            l.created_at,
            l.updated_at
        FROM tenant_leads l
        JOIN tenant_clients c ON c.id = l.tenant_client_id
        WHERE l.workspace_id = $1
        ORDER BY l.created_at DESC
        "#,
    )
    .bind(&session.workspace_id)
    .fetch_all(pool)
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "leads": leads
    })))
}

pub async fn get_lead_stats(req: HttpRequest) -> Result<HttpResponse, AppError> {
    let session = middleware::extract_tenant_user_session(&req).await?;
    middleware::require_tenant_roles(
        &session.roles,
        &[
            "owner",
            "admin",
            "account_manager",
            "sales_manager",
            "support",
        ],
    )?;
    let pool = middleware::db_pool(&req)?;

    let stats = sqlx::query_as::<_, LeadStatsRow>(
        r#"
        SELECT
            COUNT(*) AS total_leads,
            COUNT(*) FILTER (WHERE status = 'qualified') AS qualified_leads,
            COUNT(*) FILTER (WHERE status = 'converted') AS converted_leads
        FROM tenant_leads
        WHERE workspace_id = $1
        "#,
    )
    .bind(&session.workspace_id)
    .fetch_one(pool)
    .await?;

    let raw_weekly = sqlx::query_as::<_, (chrono::NaiveDate, i64)>(
        r#"
        SELECT
            DATE(created_at AT TIME ZONE 'UTC') AS date,
            COUNT(*) AS count
        FROM tenant_leads
        WHERE workspace_id = $1
          AND created_at >= NOW() - INTERVAL '6 days'
        GROUP BY date
        ORDER BY date ASC
        "#,
    )
    .bind(&session.workspace_id)
    .fetch_all(pool)
    .await?;

    let weekly_trend = (0..7)
        .map(|offset| {
            let day = (Utc::now() - Duration::days(i64::from(6 - offset))).date_naive();
            let count = raw_weekly
                .iter()
                .find(|(date, _)| *date == day)
                .map(|(_, count)| *count)
                .unwrap_or(0);

            WeeklyTrendPoint {
                date: format!("{:02}/{:02}", day.month(), day.day()),
                count,
            }
        })
        .collect::<Vec<_>>();

    let by_status = sqlx::query_as::<_, StatusCount>(
        r#"
        SELECT status, COUNT(*) AS count
        FROM tenant_leads
        WHERE workspace_id = $1
        GROUP BY status
        ORDER BY count DESC, status ASC
        "#,
    )
    .bind(&session.workspace_id)
    .fetch_all(pool)
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "stats": {
            "total": stats.total_leads,
            "qualified": stats.qualified_leads,
            "converted": stats.converted_leads,
            "weeklyTrend": weekly_trend,
            "byStatus": by_status,
        }
    })))
}

pub async fn create_lead(
    req: HttpRequest,
    body: web::Json<CreateLeadRequest>,
) -> Result<HttpResponse, AppError> {
    let session = middleware::extract_tenant_user_session(&req).await?;
    ensure_lead_write_roles(&session.roles)?;
    let pool = middleware::db_pool(&req)?;

    let name = body.name.trim();
    let email = body.email.trim().to_ascii_lowercase();
    let source = normalize_source(&body.source);

    if body.tenant_client_id.trim().is_empty() || name.is_empty() || email.is_empty() {
        return Err(AppError::BadRequest(
            "tenantClientId, name, and email are required".to_string(),
        ));
    }

    let tenant_client = middleware::require_active_tenant_client(
        pool,
        &session.workspace_id,
        body.tenant_client_id.trim(),
    )
    .await
    .map_err(|_| AppError::BadRequest("tenant client not found".to_string()))?;

    let score = calculate_score(
        &source,
        normalize_optional(&body.phone).is_some(),
        normalize_optional(&body.company).is_some(),
        normalize_optional(&body.notes).is_some(),
    );
    let lead_id = cuid2::create_id();

    let mut transaction = pool.begin().await?;
    sqlx::query(
        r#"
        INSERT INTO tenant_leads (
            id,
            workspace_id,
            tenant_client_id,
            created_by_tenant_user_id,
            name,
            email,
            phone,
            company,
            source,
            status,
            score,
            notes,
            created_at,
            updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'new', $10, $11, NOW(), NOW())
        "#,
    )
    .bind(&lead_id)
    .bind(&session.workspace_id)
    .bind(body.tenant_client_id.trim())
    .bind(&session.tenant_user_id)
    .bind(name)
    .bind(&email)
    .bind(normalize_optional(&body.phone))
    .bind(normalize_optional(&body.company))
    .bind(&source)
    .bind(score)
    .bind(normalize_optional(&body.notes))
    .execute(&mut *transaction)
    .await?;

    analytics::enqueue_lead_created(
        &mut transaction,
        &session.workspace_id,
        body.tenant_client_id.trim(),
        &lead_id,
        analytics_source(&source),
        score_band(score),
        Utc::now(),
    )
    .await
    .map_err(|error| AppError::Internal(format!("analytics lead event enqueue failed: {error}")))?;
    let lead_event_count = analytics::workspace_event_count(
        &mut transaction,
        &format!("workspace:{}:leads", session.workspace_id),
    )
    .await?;
    analytics::enqueue_lead_manifest(
        &mut transaction,
        &session.workspace_id,
        lead_event_count,
        Utc::now(),
    )
    .await
    .map_err(|error| {
        AppError::Internal(format!("analytics lead manifest enqueue failed: {error}"))
    })?;
    transaction.commit().await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "lead": {
            "id": lead_id,
            "tenantClientId": body.tenant_client_id.trim(),
            "tenantClientName": tenant_client.name,
            "name": name,
            "email": email,
            "phone": normalize_optional(&body.phone),
            "company": normalize_optional(&body.company),
            "source": source,
            "status": "new",
            "score": score,
            "notes": normalize_optional(&body.notes),
        }
    })))
}

pub async fn update_lead_status(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<UpdateLeadStatusRequest>,
) -> Result<HttpResponse, AppError> {
    let session = middleware::extract_tenant_user_session(&req).await?;
    ensure_lead_write_roles(&session.roles)?;
    let pool = middleware::db_pool(&req)?;
    let (tenant_client_id, status, changed) = change_lead_status(
        pool,
        &session.workspace_id,
        None,
        &path.into_inner(),
        &body.status,
        body.reason_code.as_deref(),
        "tenant_user",
        Some(&session.tenant_user_id),
        None,
    )
    .await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "tenantClientId": tenant_client_id,
        "status": status,
        "changed": changed
    })))
}

pub async fn list_client_leads(req: HttpRequest) -> Result<HttpResponse, AppError> {
    let session = middleware::extract_tenant_client_user_session(&req).await?;
    let pool = middleware::db_pool(&req)?;

    let leads = sqlx::query_as::<_, LeadRow>(
        r#"
        SELECT
            l.id,
            l.tenant_client_id,
            c.name AS tenant_client_name,
            l.name,
            l.email,
            l.phone,
            l.company,
            l.source,
            l.status,
            l.score,
            l.notes,
            l.created_at,
            l.updated_at
        FROM tenant_leads l
        JOIN tenant_clients c ON c.id = l.tenant_client_id
        WHERE l.workspace_id = $1
          AND l.tenant_client_id = $2
        ORDER BY l.created_at DESC
        "#,
    )
    .bind(&session.workspace_id)
    .bind(&session.tenant_client_id)
    .fetch_all(pool)
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "tenantClientId": session.tenant_client_id,
        "leads": leads
    })))
}

pub async fn update_client_lead_status(
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<UpdateLeadStatusRequest>,
) -> Result<HttpResponse, AppError> {
    let session = middleware::extract_tenant_client_user_session(&req).await?;
    let pool = middleware::db_pool(&req)?;
    let (tenant_client_id, status, changed) = change_lead_status(
        pool,
        &session.workspace_id,
        Some(&session.tenant_client_id),
        &path.into_inner(),
        &body.status,
        body.reason_code.as_deref(),
        "client_user",
        None,
        Some(&session.tenant_client_user_id),
    )
    .await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "tenantClientId": tenant_client_id,
        "status": status,
        "changed": changed
    })))
}

pub async fn get_client_lead_stats(req: HttpRequest) -> Result<HttpResponse, AppError> {
    let session = middleware::extract_tenant_client_user_session(&req).await?;
    let pool = middleware::db_pool(&req)?;

    let stats = sqlx::query_as::<_, LeadStatsRow>(
        r#"
        SELECT
            COUNT(*) AS total_leads,
            COUNT(*) FILTER (WHERE status = 'qualified') AS qualified_leads,
            COUNT(*) FILTER (WHERE status = 'converted') AS converted_leads
        FROM tenant_leads
        WHERE workspace_id = $1
          AND tenant_client_id = $2
        "#,
    )
    .bind(&session.workspace_id)
    .bind(&session.tenant_client_id)
    .fetch_one(pool)
    .await?;

    let by_status = sqlx::query_as::<_, StatusCount>(
        r#"
        SELECT status, COUNT(*) AS count
        FROM tenant_leads
        WHERE workspace_id = $1
          AND tenant_client_id = $2
        GROUP BY status
        ORDER BY count DESC, status ASC
        "#,
    )
    .bind(&session.workspace_id)
    .bind(&session.tenant_client_id)
    .fetch_all(pool)
    .await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "stats": {
            "total": stats.total_leads,
            "qualified": stats.qualified_leads,
            "converted": stats.converted_leads,
            "byStatus": by_status,
        }
    })))
}

#[cfg(test)]
mod tests {
    use super::{
        analytics_source, calculate_score, normalize_lead_reason, normalize_lead_status, score_band,
    };

    #[test]
    fn analytics_source_is_bounded() {
        assert_eq!(analytics_source("whatsapp"), "whatsapp");
        assert_eq!(analytics_source("paid_search"), "other");
        assert_eq!(analytics_source(""), "other");
    }

    #[test]
    fn score_band_does_not_export_raw_score() {
        assert_eq!(
            score_band(calculate_score("manual", false, false, false)),
            "medium"
        );
        assert_eq!(score_band(49), "low");
        assert_eq!(score_band(75), "high");
    }

    #[test]
    fn lead_status_changes_use_bounded_reason_codes() {
        assert_eq!(normalize_lead_status("qualified").unwrap(), "qualified");
        assert_eq!(normalize_lead_reason("new", None).unwrap(), "reopened");
        assert_eq!(
            normalize_lead_reason("converted", Some("converted")).unwrap(),
            "converted"
        );
        assert!(normalize_lead_status("in_progress").is_err());
        assert!(normalize_lead_reason("qualified", Some("free-form")).is_err());
    }
}
