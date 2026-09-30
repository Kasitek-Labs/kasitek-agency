"use client";

export const DEFAULT_TENANT_API_BASE_URL = "/api/tenant";

export function getTenantApiBaseUrl() {
  return DEFAULT_TENANT_API_BASE_URL;
}

export function tenantApiUrl(path: string) {
  return `${getTenantApiBaseUrl()}${path.startsWith("/") ? path : `/${path}`}`;
}

export async function tenantFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(tenantApiUrl(path), {
    credentials: "include",
    ...init,
    headers: {
      "Content-Type": "application/json",
      ...(init?.headers ?? {}),
    },
  });

  const payload = (await response.json().catch(() => ({}))) as T & { error?: string };
  if (!response.ok) {
    throw new Error(payload.error || `Request failed with status ${response.status}`);
  }

  return payload;
}
