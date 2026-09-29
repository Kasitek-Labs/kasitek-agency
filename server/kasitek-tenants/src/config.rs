use ::config::{env_csv, env_optional, env_string_or, ConfigError};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CookieSameSitePolicy {
    Lax,
    Strict,
    None,
}

impl CookieSameSitePolicy {
    pub fn from_env(name: &str, default: Self) -> Self {
        match env_optional(name)
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "strict" => Self::Strict,
            "none" => Self::None,
            "lax" => Self::Lax,
            _ => default,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TenantServerConfig {
    pub port: u16,
    pub environment: String,
    pub database_url: String,
    pub database_max_connections: u32,
    pub allowed_origins: Vec<String>,
    pub cors_allow_credentials: bool,
    pub app_base_url: Option<String>,
    pub portal_host_target: Option<String>,
    pub platform_base_domain: Option<String>,
    pub default_workspace_slug: Option<String>,
    pub session_cookie_name: String,
    pub session_cookie_domain: Option<String>,
    pub session_cookie_secure: bool,
    pub session_cookie_same_site: CookieSameSitePolicy,
    pub session_ttl_hours: u32,
    pub client_session_cookie_name: String,
    pub client_session_ttl_hours: u32,
    pub invite_ttl_hours: u32,
    pub admin_secret: Option<String>,
    pub resend_api_key: Option<String>,
    pub resend_from_email: String,
    pub shared_email_domain: String,
    /// Private Analytics reporting gateway settings. The browser never receives these values.
    pub analytics_reporting_url: String,
    pub analytics_reporting_private_key: Option<String>,
    pub analytics_reporting_key_id: Option<String>,
    pub analytics_reporting_resolver_token: Option<String>,
    pub analytics_reporting_issuer: String,
    pub analytics_reporting_audience: String,
    pub analytics_reporting_timeout_ms: u32,
}

impl TenantServerConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let port = env_optional("TENANT_SERVER_PORT")
            .and_then(|value| value.parse::<u16>().ok())
            .or_else(|| env_optional("PORT").and_then(|value| value.parse::<u16>().ok()))
            .unwrap_or(8090);

        let environment = env_string_or("TENANT_SERVER_ENVIRONMENT", "development");

        let database_url = env_optional("KASITEK_TENANTS_DATABASE_URL")
            .or_else(|| env_optional("TENANT_DATABASE_URL"))
            .ok_or(ConfigError::MissingVar(
                "tenant database url (KASITEK_TENANTS_DATABASE_URL or TENANT_DATABASE_URL)",
            ))?;

        let database_max_connections = env_optional("TENANT_DATABASE_MAX_CONNECTIONS")
            .and_then(|value| value.parse::<u32>().ok())
            .or_else(|| {
                env_optional("DATABASE_MAX_CONNECTIONS").and_then(|value| value.parse::<u32>().ok())
            })
            .unwrap_or(10);

        let allowed_origins = {
            let values = env_csv("TENANT_ALLOWED_ORIGINS");
            if values.is_empty() {
                vec![
                    "http://localhost:3000".to_string(),
                    "http://localhost:3001".to_string(),
                ]
            } else {
                values
            }
        };

        let app_base_url = env_optional("TENANT_APP_BASE_URL");
        let portal_host_target = env_optional("TENANT_PORTAL_HOST_TARGET").or_else(|| {
            app_base_url.as_ref().and_then(|url| {
                let trimmed = url
                    .trim()
                    .trim_start_matches("http://")
                    .trim_start_matches("https://");
                let host = trimmed
                    .split('/')
                    .next()
                    .unwrap_or("")
                    .split(':')
                    .next()
                    .unwrap_or("");
                if host.is_empty() {
                    None
                } else {
                    Some(host.to_ascii_lowercase())
                }
            })
        });
        let platform_base_domain = env_optional("TENANT_PLATFORM_BASE_DOMAIN")
            .map(|value| value.trim().trim_matches('.').to_ascii_lowercase())
            .filter(|value| !value.is_empty());
        let default_workspace_slug = env_optional("TENANT_DEFAULT_WORKSPACE_SLUG");
        let cors_allow_credentials = env_optional("TENANT_CORS_ALLOW_CREDENTIALS")
            .map(|value| {
                matches!(
                    value.to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
            .unwrap_or(true);
        let session_cookie_name = env_string_or("TENANT_SESSION_COOKIE_NAME", "kasitenant.session");
        let session_cookie_domain = env_optional("TENANT_SESSION_COOKIE_DOMAIN");
        let session_cookie_secure = env_optional("TENANT_SESSION_COOKIE_SECURE")
            .map(|value| {
                matches!(
                    value.to_ascii_lowercase().as_str(),
                    "1" | "true" | "yes" | "on"
                )
            })
            .unwrap_or_else(|| environment.eq_ignore_ascii_case("production"));
        let session_cookie_same_site = CookieSameSitePolicy::from_env(
            "TENANT_SESSION_COOKIE_SAME_SITE",
            CookieSameSitePolicy::Lax,
        );
        let session_ttl_hours = env_optional("TENANT_SESSION_TTL_HOURS")
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(24 * 14);
        let client_session_cookie_name = env_string_or(
            "TENANT_CLIENT_SESSION_COOKIE_NAME",
            "kasitenant.client.session",
        );
        let client_session_ttl_hours = env_optional("TENANT_CLIENT_SESSION_TTL_HOURS")
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(24 * 14);
        let invite_ttl_hours = env_optional("TENANT_INVITE_TTL_HOURS")
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(24 * 7);

        let admin_secret = env_optional("TENANT_ADMIN_SECRET");
        let resend_api_key =
            env_optional("TENANT_RESEND_API_KEY").or_else(|| env_optional("AUTH_RESEND_KEY"));
        let resend_from_email = env_optional("TENANT_RESEND_FROM_EMAIL")
            .or_else(|| env_optional("RESEND_FROM_EMAIL"))
            .unwrap_or_else(|| "KasiTek <onboarding@resend.dev>".to_string());
        let shared_email_domain = env_string_or("TENANT_SHARED_EMAIL_DOMAIN", "send.kasitek.co.za")
            .trim()
            .trim_matches('.')
            .to_ascii_lowercase();
        let analytics_reporting_url =
            env_string_or("ANALYTICS_REPORTING_URL", "http://127.0.0.1:4300");
        let analytics_reporting_private_key = env_optional("ANALYTICS_REPORTING_PRIVATE_KEY");
        let analytics_reporting_key_id = env_optional("ANALYTICS_REPORTING_KEY_ID");
        let analytics_reporting_resolver_token = env_optional("ANALYTICS_REPORTING_RESOLVER_TOKEN");
        let analytics_reporting_issuer =
            env_string_or("ANALYTICS_REPORTING_ISSUER", "kasitek-tenant-platform");
        let analytics_reporting_audience = env_string_or(
            "ANALYTICS_REPORTING_AUDIENCE",
            "kasitek-analytics-reporting",
        );
        let analytics_reporting_timeout_ms = env_optional("ANALYTICS_REPORTING_TIMEOUT_MS")
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(5_000)
            .clamp(250, 120_000);

        Ok(Self {
            port,
            environment,
            database_url,
            database_max_connections,
            allowed_origins,
            cors_allow_credentials,
            app_base_url,
            portal_host_target,
            platform_base_domain,
            default_workspace_slug,
            session_cookie_name,
            session_cookie_domain,
            session_cookie_secure,
            session_cookie_same_site,
            session_ttl_hours,
            client_session_cookie_name,
            client_session_ttl_hours,
            invite_ttl_hours,
            admin_secret,
            resend_api_key,
            resend_from_email,
            shared_email_domain,
            analytics_reporting_url,
            analytics_reporting_private_key,
            analytics_reporting_key_id,
            analytics_reporting_resolver_token,
            analytics_reporting_issuer,
            analytics_reporting_audience,
            analytics_reporting_timeout_ms,
        })
    }

    pub fn is_production(&self) -> bool {
        self.environment.eq_ignore_ascii_case("production")
    }
}

pub use self::TenantServerConfig as AppConfig;
