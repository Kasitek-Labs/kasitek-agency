"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { AlertCircle, Database, RefreshCw } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { fetchTenantAnalytics, type TenantAnalyticsAudience, type TenantAnalyticsCatalog, type TenantAnalyticsMetric, type TenantAnalyticsReport } from "@/lib/tenant-analytics";

export function TenantAnalyticsPanel({ audience, includeDescendants = false }: { audience: TenantAnalyticsAudience; includeDescendants?: boolean }) {
  const [catalog, setCatalog] = useState<TenantAnalyticsCatalog | null>(null);
  const [report, setReport] = useState<TenantAnalyticsReport | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async (background = false) => {
    background ? setRefreshing(true) : setLoading(true);
    setError(null);
    try {
      const result = await fetchTenantAnalytics(audience, includeDescendants);
      setCatalog(result.catalog);
      setReport(result.report);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : "Analytics are unavailable");
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  }, [audience, includeDescendants]);

  useEffect(() => {
    void load();
  }, [load]);

  const definitions = useMemo(() => new Map((catalog?.metrics ?? []).map((metric) => [`${metric.key}@${metric.version}`, metric])), [catalog]);

  return (
    <Card>
      <CardHeader className="gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div>
          <CardTitle>{audience === "client" ? "Client analytics" : "Workspace analytics"}</CardTitle>
          <CardDescription>{audience === "client" ? "Metrics for this client only." : includeDescendants ? "Workspace metrics including permitted client scopes." : "Workspace metrics."}</CardDescription>
        </div>
        <Button type="button" variant="outline" size="sm" className="gap-1.5" disabled={loading || refreshing} onClick={() => void load(true)}><RefreshCw className={refreshing ? "h-3.5 w-3.5 animate-spin" : "h-3.5 w-3.5"} />Refresh</Button>
      </CardHeader>
      <CardContent>
        {loading ? <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">{Array.from({ length: 4 }).map((_, index) => <Skeleton key={index} className="h-28 rounded-xl" />)}</div> : error ? <div role="alert" className="flex items-start gap-3 rounded-lg border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive"><AlertCircle className="mt-0.5 h-4 w-4 shrink-0" />{error}<Button type="button" variant="outline" size="sm" className="ml-auto" onClick={() => void load(true)}>Try again</Button></div> : report ? <div className="space-y-5">
          <div className="flex flex-wrap gap-3 text-xs text-muted-foreground"><Badge variant="outline">Scope {report.scope.kind}</Badge><span>Timezone: {report.timezone}</span><span>Data as of: {formatDate(report.data_as_of)}</span><span>Generation: {report.generation_id ? `#${report.generation_id}` : "—"}</span></div>
          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">{report.metrics.slice(0, 4).map((metric) => <TenantMetricCard key={`${metric.key}-${metric.version}`} metric={metric} description={definitions.get(`${metric.key}@${metric.version}`)?.description} />)}</div>
          <div className="space-y-2">{report.metrics.map((metric) => <TenantMetricRow key={`${metric.key}-${metric.version}`} metric={metric} description={definitions.get(`${metric.key}@${metric.version}`)?.description} timezone={report.timezone} />)}</div>
        </div> : null}
      </CardContent>
    </Card>
  );
}

function TenantMetricCard({ metric, description }: { metric: TenantAnalyticsMetric; description?: string }) {
  const value = [...metric.series].reverse().find((point) => point.value !== null)?.value ?? null;
  return <div className="rounded-xl border border-border p-4"><div className="text-xs text-muted-foreground">{description || metric.key}</div><div className="mt-2 text-2xl font-semibold">{formatValue(value, metric.unit)}</div><div className="mt-3 flex flex-wrap gap-2"><StateBadge state={metric.availability.state} /><StateBadge state={metric.freshness.state} /><StateBadge state={metric.coverage.state} /></div></div>;
}

function TenantMetricRow({ metric, description, timezone }: { metric: TenantAnalyticsMetric; description?: string; timezone: string }) {
  return <details className="rounded-lg border border-border"><summary className="flex cursor-pointer list-none flex-col gap-2 p-4 sm:flex-row sm:items-center sm:justify-between [&::-webkit-details-marker]:hidden"><div><div className="text-sm font-medium">{description || metric.key}</div><div className="mt-1 text-xs text-muted-foreground">{metric.key} · {metric.exactness}</div></div><div className="flex flex-wrap gap-2"><StateBadge state={metric.availability.state} /><StateBadge state={metric.freshness.state} /><StateBadge state={metric.coverage.state} /></div></summary><div className="border-t border-border p-4"><div className="overflow-x-auto"><table className="w-full text-sm"><thead><tr className="border-b border-border text-left text-xs text-muted-foreground"><th className="px-2 py-2">Period</th><th className="px-2 py-2 text-right">Value</th></tr></thead><tbody>{metric.series.map((point) => <tr key={point.bucket_start} className="border-b border-border last:border-0"><td className="px-2 py-2">{new Date(point.bucket_start).toLocaleDateString(undefined, { timeZone: timezone })}</td><td className="px-2 py-2 text-right font-medium">{formatValue(point.value, metric.unit)}</td></tr>)}</tbody></table></div>{metric.warnings.length ? <p className="mt-3 text-xs text-amber-600">{metric.warnings.join(" · ")}</p> : null}</div></details>;
}

function StateBadge({ state }: { state: string }) {
  const tone = ["available", "complete", "fresh"].includes(state) ? "border-green-500/30 bg-green-500/10 text-green-700" : ["partial", "stale"].includes(state) ? "border-amber-500/30 bg-amber-500/10 text-amber-700" : "border-border bg-muted text-muted-foreground";
  return <Badge variant="outline" className={tone}>{state.replaceAll("_", " ")}</Badge>;
}

function formatValue(value: number | null, unit: string) {
  if (value === null) return "—";
  const formatted = value.toLocaleString(undefined, { maximumFractionDigits: unit === "count" ? 0 : 2 });
  return unit === "count" ? formatted : `${formatted} ${unit}`;
}

function formatDate(value: string | null) {
  if (!value) return "—";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? "—" : date.toLocaleString();
}
