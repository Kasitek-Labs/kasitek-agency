"use client";

import React, { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";

import { tenantFetch } from "@/lib/api";
import {
  buildTenantThemeStyle,
  getTenantDisplayName,
  getTenantFeatureConfig,
  getTenantLogoUrl,
  getTenantTagline,
  normalizeTenantContext,
  type PublicTenantContext,
  type TenantFeatureConfig,
  type TenantThemeStyle,
} from "@/lib/tenant-context";

type TenantContextValue = {
  context: PublicTenantContext;
  isLoading: boolean;
  refresh: () => Promise<void>;
  brandName: string;
  tagline: string;
  logoUrl: string | null;
  themeStyle: TenantThemeStyle;
  features: TenantFeatureConfig;
};

const DEFAULT_CONTEXT: PublicTenantContext = {
  workspace: null,
  branding: null,
};

const TenantContext = createContext<TenantContextValue | null>(null);

async function fetchTenantContext() {
  const payload = await tenantFetch<unknown>("/api/public/tenant-context", {
    method: "GET",
  });

  return normalizeTenantContext(payload);
}

export function TenantContextProvider({
  children,
  initialContext,
}: {
  children: React.ReactNode;
  initialContext?: PublicTenantContext | null;
}) {
  const [context, setContext] = useState<PublicTenantContext>(initialContext ?? DEFAULT_CONTEXT);
  const [isLoading, setIsLoading] = useState(!initialContext?.workspace && !initialContext?.branding);
  const shouldRefreshOnMount = !initialContext?.workspace && !initialContext?.branding;

  const refresh = useCallback(async () => {
    setIsLoading(true);
    try {
      const nextContext = await fetchTenantContext();
      setContext(nextContext);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    if (shouldRefreshOnMount) {
      void refresh();
    }
  }, [refresh, shouldRefreshOnMount]);

  useEffect(() => {
    const style = buildTenantThemeStyle(context);
    const root = document.documentElement;

    Object.entries(style).forEach(([key, value]) => {
      root.style.setProperty(key, value);
    });
  }, [context]);

  useEffect(() => {
    document.title = getTenantDisplayName(context);
  }, [context]);

  const value = useMemo<TenantContextValue>(() => {
    return {
      context,
      isLoading,
      refresh,
      brandName: getTenantDisplayName(context),
      tagline: getTenantTagline(context),
      logoUrl: getTenantLogoUrl(context),
      themeStyle: buildTenantThemeStyle(context),
      features: getTenantFeatureConfig(context),
    };
  }, [context, isLoading, refresh]);

  return <TenantContext.Provider value={value}>{children}</TenantContext.Provider>;
}

export function useTenantContext() {
  const context = useContext(TenantContext);
  if (!context) {
    throw new Error("useTenantContext must be used within TenantContextProvider");
  }

  return context;
}
