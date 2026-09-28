export const ROLE_OWNER = "owner";
export const ROLE_ADMIN = "admin";
export const ROLE_ACCOUNT_MANAGER = "account_manager";
export const ROLE_VIEWER = "viewer";
export const ROLE_SUPPORT = "support";
export const ROLE_SALES_MANAGER = "sales_manager";
export const ROLE_CONTENT_MANAGER = "content_manager";
export const ROLE_AUTOMATION_MANAGER = "automation_manager";

export const CLIENT_ADMIN_ROLES = [ROLE_OWNER, ROLE_ADMIN, ROLE_ACCOUNT_MANAGER];
export const LEAD_OPERATOR_ROLES = [
  ROLE_OWNER,
  ROLE_ADMIN,
  ROLE_ACCOUNT_MANAGER,
  ROLE_SALES_MANAGER,
  ROLE_SUPPORT,
];
export const REPORTING_ROLES = [ROLE_OWNER, ROLE_ADMIN, ROLE_ACCOUNT_MANAGER, ROLE_VIEWER];
export const CONTENT_ROLES = [ROLE_OWNER, ROLE_ADMIN, ROLE_ACCOUNT_MANAGER, ROLE_CONTENT_MANAGER];
export const AUTOMATION_ROLES = [
  ROLE_OWNER,
  ROLE_ADMIN,
  ROLE_ACCOUNT_MANAGER,
  ROLE_AUTOMATION_MANAGER,
  ROLE_SUPPORT,
];
export const VOICE_ROLES = [ROLE_OWNER, ROLE_ADMIN, ROLE_AUTOMATION_MANAGER, ROLE_SUPPORT];
export const REPUTATION_ROLES = [ROLE_OWNER, ROLE_ADMIN, ROLE_ACCOUNT_MANAGER, ROLE_SUPPORT];
export const SETTINGS_ROLES = [ROLE_OWNER, ROLE_ADMIN];
export const DASHBOARD_ROLES = [
  ROLE_OWNER,
  ROLE_ADMIN,
  ROLE_ACCOUNT_MANAGER,
  ROLE_VIEWER,
  ROLE_SUPPORT,
  ROLE_SALES_MANAGER,
  ROLE_CONTENT_MANAGER,
  ROLE_AUTOMATION_MANAGER,
];

export type TenantNavItem = {
  href: string;
  label: string;
  icon: string;
  allowedRoles: string[];
};

export const TENANT_ROUTE_ACCESS = {
  dashboard: DASHBOARD_ROLES,
  clients: CLIENT_ADMIN_ROLES,
  leads: LEAD_OPERATOR_ROLES,
  content: CONTENT_ROLES,
  reporting: REPORTING_ROLES,
  nurturing: AUTOMATION_ROLES,
  aeo: CONTENT_ROLES,
  voice: VOICE_ROLES,
  reviews: REPUTATION_ROLES,
  adcreative: CONTENT_ROLES,
  settings: SETTINGS_ROLES,
} as const;

export function hasAnyTenantRole(userRoles: string[] | undefined, allowedRoles: string[]) {
  if (!allowedRoles.length) {
    return true;
  }

  return allowedRoles.some((role) => userRoles?.includes(role));
}
