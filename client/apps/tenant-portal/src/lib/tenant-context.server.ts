import { cache } from "react";
import { headers } from "next/headers";

import { normalizeTenantContext, type PublicTenantContext } from "@/lib/tenant-context";

const FALLBACK_CONTEXT: PublicTenantContext = {
  workspace: null,
  branding: null,
};

function getRequestOrigin(headerList: Headers) {
  const protocol = headerList.get("x-forwarded-proto") || "http";
  const host = headerList.get("x-forwarded-host") || headerList.get("host");

  if (!host) {
    return null;
  }

  return `${protocol}://${host}`;
}

export const getServerTenantContext = cache(async (): Promise<PublicTenantContext> => {
  try {
    const headerList = await headers();
    const origin = getRequestOrigin(headerList);

    if (!origin) {
      return FALLBACK_CONTEXT;
    }

    const response = await fetch(new URL("/api/public/tenant-context", origin), {
      method: "GET",
      cache: "no-store",
      headers: {
        Accept: "application/json",
      },
    });

    if (!response.ok) {
      return FALLBACK_CONTEXT;
    }

    const payload = await response.json().catch(() => null);
    return normalizeTenantContext(payload);
  } catch {
    return FALLBACK_CONTEXT;
  }
});
