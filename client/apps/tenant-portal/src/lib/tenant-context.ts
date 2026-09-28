import type { CSSProperties, ReactNode } from "react";

export type PublicWorkspaceContext = {
  id: string;
  slug: string;
  displayName: string;
};

export type PublicBrandingContext = {
  displayName?: string | null;
  primaryColor?: string | null;
  secondaryColor?: string | null;
  logoUrl?: string | null;
  widgetConfig?: Record<string, unknown> | null;
  customDomain?: string | null;
};

export type PublicTenantContext = {
  workspace: PublicWorkspaceContext | null;
  branding: PublicBrandingContext | null;
};

export type TenantThemeStyle = CSSProperties & Record<string, string>;

/**
 * Feature flags derived from the tenant's widgetConfig.
 * All features default to enabled when no config is present, so existing
 * tenants see no change. A tenant can restrict features by setting:
 *   widgetConfig.features = { voice: false, aeo: false }
 * or by listing only allowed modules in:
 *   widgetConfig.enabledModules = ["dashboard", "clients", "leads"]
 */
export type TenantFeatureConfig = {
  dashboard: boolean;
  clients: boolean;
  leads: boolean;
  content: boolean;
  reporting: boolean;
  nurturing: boolean;
  aeo: boolean;
  voice: boolean;
  reviews: boolean;
  adcreative: boolean;
  settings: boolean;
};

export type TenantFeatureKey = keyof TenantFeatureConfig;

const ALL_FEATURES_ENABLED: TenantFeatureConfig = {
  dashboard: true,
  clients: true,
  leads: true,
  content: true,
  reporting: true,
  nurturing: true,
  aeo: true,
  voice: true,
  reviews: true,
  adcreative: true,
  settings: true,
};

export function getTenantFeatureConfig(
  context: PublicTenantContext | null | undefined,
): TenantFeatureConfig {
  const widgetConfig = context?.branding?.widgetConfig;

  if (!widgetConfig || typeof widgetConfig !== "object") {
    return ALL_FEATURES_ENABLED;
  }

  const config = widgetConfig as Record<string, unknown>;

  // Support widgetConfig.features = { voice: false, aeo: false, ... }
  const featuresEntry = config["features"];
  if (featuresEntry && typeof featuresEntry === "object" && !Array.isArray(featuresEntry)) {
    const f = featuresEntry as Record<string, unknown>;
    return {
      dashboard: f["dashboard"] !== false,
      clients: f["clients"] !== false,
      leads: f["leads"] !== false,
      content: f["content"] !== false,
      reporting: f["reporting"] !== false,
      nurturing: f["nurturing"] !== false,
      aeo: f["aeo"] !== false,
      voice: f["voice"] !== false,
      reviews: f["reviews"] !== false,
      adcreative: f["adcreative"] !== false,
      settings: f["settings"] !== false,
    };
  }

  // Support widgetConfig.enabledModules = ["dashboard", "clients", "leads"]
  const enabledModules = config["enabledModules"];
  if (Array.isArray(enabledModules)) {
    const moduleSet = new Set(enabledModules as string[]);
    return {
      dashboard: moduleSet.has("dashboard"),
      clients: moduleSet.has("clients"),
      leads: moduleSet.has("leads"),
      content: moduleSet.has("content"),
      reporting: moduleSet.has("reporting"),
      nurturing: moduleSet.has("nurturing"),
      aeo: moduleSet.has("aeo"),
      voice: moduleSet.has("voice"),
      reviews: moduleSet.has("reviews"),
      adcreative: moduleSet.has("adcreative"),
      settings: moduleSet.has("settings"),
    };
  }

  return ALL_FEATURES_ENABLED;
}

export const DEFAULT_TENANT_NAME = "Portal";
export const DEFAULT_TENANT_TAGLINE = "Access your workspace";

export function normalizeTenantContext(payload: unknown): PublicTenantContext {
  if (!payload || typeof payload !== "object") {
    return { workspace: null, branding: null };
  }

  const data = payload as {
    workspace?: {
      id?: string;
      slug?: string;
      displayName?: string;
    } | null;
    branding?: {
      displayName?: string | null;
      primaryColor?: string | null;
      secondaryColor?: string | null;
      logoUrl?: string | null;
      widgetConfig?: Record<string, unknown> | null;
      customDomain?: string | null;
    } | null;
  };

  const workspace =
    data.workspace && data.workspace.id && data.workspace.slug && data.workspace.displayName
      ? {
          id: data.workspace.id,
          slug: data.workspace.slug,
          displayName: data.workspace.displayName,
        }
      : null;

  const branding = data.branding
    ? {
        displayName: data.branding.displayName ?? null,
        primaryColor: data.branding.primaryColor ?? null,
        secondaryColor: data.branding.secondaryColor ?? null,
        logoUrl: data.branding.logoUrl ?? null,
        widgetConfig: data.branding.widgetConfig ?? null,
        customDomain: data.branding.customDomain ?? null,
      }
    : null;

  return { workspace, branding };
}

export function getTenantDisplayName(context: PublicTenantContext | null | undefined) {
  return (
    context?.branding?.displayName ||
    context?.workspace?.displayName ||
    DEFAULT_TENANT_NAME
  );
}

export function getTenantTagline(context: PublicTenantContext | null | undefined) {
  const widgetConfig = context?.branding?.widgetConfig;
  const tagline =
    widgetConfig && typeof widgetConfig === "object"
      ? widgetConfig["tagline"]
      : null;

  if (typeof tagline === "string" && tagline.trim()) {
    return tagline.trim();
  }

  return DEFAULT_TENANT_TAGLINE;
}

export function getTenantLogoUrl(context: PublicTenantContext | null | undefined) {
  return context?.branding?.logoUrl?.trim() || null;
}

export function buildTenantThemeStyle(context: PublicTenantContext | null | undefined): TenantThemeStyle {
  const primaryColor = normalizeCssColorToHsl(context?.branding?.primaryColor);
  const secondaryColor = normalizeCssColorToHsl(context?.branding?.secondaryColor);

  const style: TenantThemeStyle = {};

  if (primaryColor) {
    style["--primary"] = primaryColor;
    style["--ring"] = primaryColor;
    style["--sidebar-primary"] = primaryColor;
    style["--sidebar-ring"] = primaryColor;
    style["--chart-1"] = primaryColor;
    style["--primary-foreground"] = getReadableForeground(primaryColor);
    style["--sidebar-primary-foreground"] = getReadableForeground(primaryColor);
  }

  if (secondaryColor) {
    style["--secondary"] = secondaryColor;
    style["--accent"] = secondaryColor;
    style["--sidebar-accent"] = secondaryColor;
    style["--chart-2"] = secondaryColor;
  }

  return style;
}

export function buildTenantMetadataTitle(context: PublicTenantContext | null | undefined) {
  const displayName = getTenantDisplayName(context);
  return {
    default: displayName,
    template: `%s | ${displayName}`,
  };
}

function normalizeCssColorToHsl(color: string | null | undefined) {
  if (!color) {
    return null;
  }

  const trimmed = color.trim();
  if (!trimmed) {
    return null;
  }

  if (trimmed.startsWith("hsl(")) {
    return extractHslFromFunction(trimmed);
  }

  if (trimmed.startsWith("#")) {
    return hexToHsl(trimmed);
  }

  if (trimmed.startsWith("rgb(") || trimmed.startsWith("rgba(")) {
    return rgbToHsl(trimmed);
  }

  return null;
}

function extractHslFromFunction(color: string) {
  const match = color.match(
    /hsl\(\s*([0-9.]+)(deg|rad|turn)?\s+([0-9.]+)%\s+([0-9.]+)%\s*(?:\/\s*([0-9.]+%?))?\s*\)/i,
  );
  if (!match) {
    return null;
  }

  const hue = normalizeHue(match[1], match[2]);
  const saturation = clamp(Number(match[3]), 0, 100);
  const lightness = clamp(Number(match[4]), 0, 100);
  return `${hue} ${saturation}% ${lightness}%`;
}

function hexToHsl(hex: string) {
  const normalized = hex.replace(/^#/, "").trim();
  const expanded =
    normalized.length === 3 || normalized.length === 4
      ? normalized
          .split("")
          .slice(0, 3)
          .map((char) => `${char}${char}`)
          .join("")
      : normalized.slice(0, 6);

  if (expanded.length !== 6) {
    return null;
  }

  const red = Number.parseInt(expanded.slice(0, 2), 16) / 255;
  const green = Number.parseInt(expanded.slice(2, 4), 16) / 255;
  const blue = Number.parseInt(expanded.slice(4, 6), 16) / 255;

  return rgbChannelsToHsl(red, green, blue);
}

function rgbToHsl(color: string) {
  const values = color.match(/rgba?\(([^)]+)\)/i)?.[1];
  if (!values) {
    return null;
  }

  const [red, green, blue] = values
    .split(",")
    .slice(0, 3)
    .map((value) => Number.parseFloat(value.trim()));

  if ([red, green, blue].some((value) => Number.isNaN(value))) {
    return null;
  }

  return rgbChannelsToHsl(red / 255, green / 255, blue / 255);
}

function rgbChannelsToHsl(red: number, green: number, blue: number) {
  const max = Math.max(red, green, blue);
  const min = Math.min(red, green, blue);
  const lightness = (max + min) / 2;

  if (max === min) {
    return `0 0% ${round(lightness * 100)}%`;
  }

  const delta = max - min;
  const saturation =
    lightness > 0.5
      ? delta / (2 - max - min)
      : delta / (max + min);

  let hue = 0;
  switch (max) {
    case red:
      hue = (green - blue) / delta + (green < blue ? 6 : 0);
      break;
    case green:
      hue = (blue - red) / delta + 2;
      break;
    case blue:
      hue = (red - green) / delta + 4;
      break;
  }

  return `${round((hue / 6) * 360)} ${round(saturation * 100)}% ${round(lightness * 100)}%`;
}

function normalizeHue(value: string, unit?: string | null) {
  const hue = Number.parseFloat(value);
  if (Number.isNaN(hue)) {
    return 0;
  }

  switch (unit) {
    case "rad":
      return round((hue * 180) / Math.PI) % 360;
    case "turn":
      return round(hue * 360) % 360;
    default:
      return round(hue) % 360;
  }
}

function getReadableForeground(hslColor: string) {
  const match = hslColor.match(/^\s*([0-9.]+)\s+([0-9.]+)%\s+([0-9.]+)%\s*$/);
  if (!match) {
    return "210 40% 98%";
  }

  const lightness = Number.parseFloat(match[3]);
  return lightness > 58 ? "222 47% 11%" : "210 40% 98%";
}

function clamp(value: number, min: number, max: number) {
  return Math.min(max, Math.max(min, value));
}

function round(value: number) {
  return Math.round(value);
}
