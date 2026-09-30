use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TenantWorkspaceContext {
    pub workspace_id: String,
    pub workspace_slug: String,
    pub scopes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TenantResolvedWorkspace {
    pub workspace_id: String,
    pub workspace_slug: String,
    pub display_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TenantUserSessionContext {
    pub workspace_id: String,
    pub workspace_slug: String,
    pub tenant_user_id: String,
    pub email: String,
    pub name: Option<String>,
    pub roles: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TenantAccessContext {
    pub workspace_id: String,
    pub workspace_slug: String,
    pub scopes: Vec<String>,
    pub tenant_user_id: Option<String>,
    pub tenant_user_roles: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TenantClientUserSessionContext {
    pub workspace_id: String,
    pub workspace_slug: String,
    pub tenant_client_id: String,
    pub tenant_client_name: String,
    pub tenant_client_user_id: String,
    pub email: String,
    pub name: Option<String>,
    pub roles: Vec<String>,
}
