import { tenantFetch } from "@/lib/api";

export type TenantAnalyticsAudience = "workspace" | "client";

export type TenantAnalyticsScope = {
  kind: string;
  externalId: string;
};

export type TenantAnalyticsCatalog = {
  scope: TenantAnalyticsScope;
  scope_kind: string;
  timezone: string;
  registry_version: string;
  metrics: Array<{ key: string; version: number; description: string; unit: string }>;
};

export type TenantAnalyticsMetric = {
  key: string;
  version: number;
  unit: string;
  exactness: string;
  availability: { state: string; reason: string | null };
  coverage: { state: string; included_points: number; expected_points: number | null };
  freshness: { state: string; latest_source_timestamp: string | null; max_age_seconds: number };
  warnings: string[];
  series: Array<{ bucket_start: string; value: number | null }>;
};

export type TenantAnalyticsReport = {
  scope: TenantAnalyticsScope;
  scope_kind: string;
  range_start: string;
  range_end: string;
  timezone: string;
  generated_at: string;
  data_as_of: string | null;
  generation_id: number | null;
  projection_version: string;
  metrics: TenantAnalyticsMetric[];
};

function pathFor(audience: TenantAnalyticsAudience, resource: "catalog" | "query") {
  const prefix = audience === "client" ? "/api/client/analytics" : "/api/analytics";
  return `${prefix}/${resource}`;
}

export function createTenantAnalyticsDateRange(timezone: string, days = 30) {
  const end = new Date();
  const start = new Date(end.getTime() - Math.min(366, Math.max(1, days)) * 24 * 60 * 60 * 1000);
  return { range_start: start.toISOString(), range_end: end.toISOString(), timezone: timezone || "UTC" };
}

export async function fetchTenantAnalytics(audience: TenantAnalyticsAudience, includeDescendants: boolean) {
  const catalog = await tenantFetch<TenantAnalyticsCatalog>(pathFor(audience, "catalog"));
  const report = await tenantFetch<TenantAnalyticsReport>(pathFor(audience, "query"), {
    method: "POST",
    body: JSON.stringify({
      ...createTenantAnalyticsDateRange(catalog.timezone),
      interval: "day",
      metric_set: "overview.summary@1",
      include_descendants: includeDescendants,
    }),
  });
  return { catalog, report };
}
