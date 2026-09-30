//! Tenant workspace and tenant-client customer-analytics reporting gateways.
//!
//! Tenant sessions and roles are resolved by Tenant Platform. Analytics receives only a
//! short-lived grant for the exact workspace or client scope selected here.

use std::collections::BTreeSet;

use actix_web::{web, HttpRequest, HttpResponse};
use analytics_reporting::{
    AnalyticsReportingClient, DescendantMode, ReportQueryInput, ReportingClientError,
    ReportingGrantInput, ReportingGrantIssuer, REPORTING_AUDIENCE,
};
use cuid2::create_id;

use crate::{config::AppConfig, error::AppError, middleware};

const SOURCE_SERVICE: &str = "tenant-platform";
const WORKSPACE_SCOPE_KIND: &str = "tenant_workspace";
const CLIENT_SCOPE_KIND: &str = "tenant_client";
const REPORTING_PERMISSION: &str = "analytics:read";
const DESCENDANT_PERMISSION: &str = "analytics:compare_children";

fn dependencies(
    config: &AppConfig,
) -> Result<(AnalyticsReportingClient, ReportingGrantIssuer), AppError> {
    let resolver_token = config
        .analytics_reporting_resolver_token
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::Internal("Customer analytics is not configured".to_string()))?;
    let private_key = config
        .analytics_reporting_private_key
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::Internal("Customer analytics is not configured".to_string()))?;
    let client = AnalyticsReportingClient::new(
        config.analytics_reporting_url.clone(),
        resolver_token,
        std::time::Duration::from_millis(u64::from(config.analytics_reporting_timeout_ms)),
    )
    .map_err(map_reporting_error)?;
    let issuer = ReportingGrantIssuer::from_pem(
        private_key,
        &config.analytics_reporting_issuer,
        if config.analytics_reporting_audience.trim().is_empty() {
            REPORTING_AUDIENCE
        } else {
            &config.analytics_reporting_audience
        },
        config.analytics_reporting_key_id.clone(),
    )
    .map_err(map_reporting_error)?;
    Ok((client, issuer))
}

async fn issue_report(
    request: &HttpRequest,
    scope_kind: &str,
    external_scope_id: &str,
    principal_id: &str,
    roles: &[String],
    payload: Option<ReportQueryInput>,
) -> Result<HttpResponse, AppError> {
    let config = middleware::app_config(request)?;
    let (client, issuer) = dependencies(config)?;
    let scope = client
        .resolve_scope(SOURCE_SERVICE, scope_kind, external_scope_id)
        .await
        .map_err(map_reporting_error)?;
    if scope.owning_source_service != SOURCE_SERVICE
        || scope.scope_kind != scope_kind
        || scope.external_source_id != external_scope_id
    {
        return Err(AppError::Internal(
            "Customer analytics scope identity mismatch".to_string(),
        ));
    }

    let can_compare_children =
        middleware::tenant_user_has_any_role(roles, &["owner", "admin", "account_manager"]);
    let descendant_mode = if scope_kind == WORKSPACE_SCOPE_KIND && can_compare_children {
        DescendantMode::All
    } else {
        DescendantMode::None
    };
    if payload
        .as_ref()
        .is_some_and(|payload| payload.include_descendants)
        && !matches!(descendant_mode, DescendantMode::All)
    {
        return Err(AppError::Forbidden);
    }
    let mut permissions = BTreeSet::from([REPORTING_PERMISSION.to_string()]);
    if matches!(descendant_mode, DescendantMode::All) {
        permissions.insert(DESCENDANT_PERMISSION.to_string());
    }
    let grant = issuer
        .issue(ReportingGrantInput {
            principal_id: principal_id.to_string(),
            scope_id: scope.scope_id,
            root_scope_id: scope.root_scope_id,
            descendant_mode,
            permissions,
            metric_keys: Vec::new(),
            dimension_keys: Vec::new(),
            correlation_id: Some(format!("tenant-{}", create_id())),
        })
        .map_err(map_reporting_error)?;

    let response = match payload {
        Some(payload) => {
            let report = payload.with_scope(scope.scope_id);
            client
                .report(&grant, &report)
                .await
                .map_err(map_reporting_error)?
        }
        None => client
            .catalog(&grant, scope.scope_id)
            .await
            .map_err(map_reporting_error)?,
    };
    Ok(
        HttpResponse::Ok().json(analytics_reporting::redact_scope_id(
            response,
            scope_kind,
            external_scope_id,
        )),
    )
}

pub async fn workspace_catalog(request: HttpRequest) -> Result<HttpResponse, AppError> {
    let session = middleware::extract_tenant_user_session(&request).await?;
    middleware::require_tenant_roles(
        &session.roles,
        &[
            "owner",
            "admin",
            "account_manager",
            "viewer",
            "support",
            "sales_manager",
            "content_manager",
            "automation_manager",
        ],
    )?;
    issue_report(
        &request,
        WORKSPACE_SCOPE_KIND,
        &session.workspace_id,
        &session.tenant_user_id,
        &session.roles,
        None,
    )
    .await
}

pub async fn workspace_query(
    request: HttpRequest,
    payload: web::Json<ReportQueryInput>,
) -> Result<HttpResponse, AppError> {
    let session = middleware::extract_tenant_user_session(&request).await?;
    middleware::require_tenant_roles(
        &session.roles,
        &[
            "owner",
            "admin",
            "account_manager",
            "viewer",
            "support",
            "sales_manager",
            "content_manager",
            "automation_manager",
        ],
    )?;
    issue_report(
        &request,
        WORKSPACE_SCOPE_KIND,
        &session.workspace_id,
        &session.tenant_user_id,
        &session.roles,
        Some(payload.into_inner()),
    )
    .await
}

pub async fn client_catalog(request: HttpRequest) -> Result<HttpResponse, AppError> {
    let session = middleware::extract_tenant_client_user_session(&request).await?;
    middleware::require_tenant_roles(&session.roles, &["owner", "admin", "member", "viewer"])?;
    issue_report(
        &request,
        CLIENT_SCOPE_KIND,
        &session.tenant_client_id,
        &session.tenant_client_user_id,
        &session.roles,
        None,
    )
    .await
}

pub async fn client_query(
    request: HttpRequest,
    payload: web::Json<ReportQueryInput>,
) -> Result<HttpResponse, AppError> {
    let session = middleware::extract_tenant_client_user_session(&request).await?;
    middleware::require_tenant_roles(&session.roles, &["owner", "admin", "member", "viewer"])?;
    issue_report(
        &request,
        CLIENT_SCOPE_KIND,
        &session.tenant_client_id,
        &session.tenant_client_user_id,
        &session.roles,
        Some(payload.into_inner()),
    )
    .await
}

fn map_reporting_error(error: ReportingClientError) -> AppError {
    match error {
        ReportingClientError::Configuration(_message) => {
            observability::tracing::error!(
                event = "dependency.unavailable",
                dependency = "customer_analytics",
                operation = "configuration",
                error_type = "configuration",
                "tenant analytics gateway configuration failed"
            );
            AppError::Internal("Customer analytics is not configured".to_string())
        }
        ReportingClientError::Signing(_message) => {
            observability::tracing::error!(
                event = "operation.failed",
                operation = "customer_analytics.grant_signing",
                error_type = "signing",
                "tenant analytics grant signing failed"
            );
            AppError::Internal("Customer analytics authorization failed".to_string())
        }
        ReportingClientError::Request(error) => {
            observability::tracing::warn!(
                event = "dependency.unavailable",
                dependency = "customer_analytics",
                operation = "request",
                error_type = %std::any::type_name_of_val(&error),
                "tenant analytics request failed"
            );
            AppError::Internal("Customer analytics is temporarily unavailable".to_string())
        }
        ReportingClientError::Upstream {
            status,
            body: _body,
        } => {
            observability::tracing::warn!(
                event = "provider.call.failed",
                provider = "customer_analytics",
                operation = "report",
                status,
                "tenant analytics upstream rejected gateway request"
            );
            if status == 400 {
                AppError::BadRequest("Customer analytics rejected the report request".to_string())
            } else if status == 401 || status == 403 {
                AppError::Internal("Customer analytics authorization failed".to_string())
            } else {
                AppError::Internal("Customer analytics is temporarily unavailable".to_string())
            }
        }
        ReportingClientError::InvalidResponse(_message) => {
            observability::tracing::error!(
                event = "provider.call.failed",
                provider = "customer_analytics",
                operation = "report",
                error_type = "invalid_response",
                "tenant analytics returned invalid response"
            );
            AppError::Internal("Customer analytics returned an invalid response".to_string())
        }
    }
}
