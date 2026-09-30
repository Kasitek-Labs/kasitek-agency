import type { Metadata } from "next";
import type { ReactNode } from "react";

import Providers from "@/components/providers";
import "../index.css";
import {
  buildTenantMetadataTitle,
  buildTenantThemeStyle,
  getTenantDisplayName,
  getTenantTagline,
} from "@/lib/tenant-context";
import { getServerTenantContext } from "@/lib/tenant-context.server";

export const dynamic = "force-dynamic";

export async function generateMetadata(): Promise<Metadata> {
  const tenantContext = await getServerTenantContext();
  const displayName = getTenantDisplayName(tenantContext);
  const tagline = getTenantTagline(tenantContext);
  const title = buildTenantMetadataTitle(tenantContext);

  return {
    title,
    description: `${displayName} - ${tagline}`,
    themeColor: tenantContext.branding?.primaryColor || "#0ea5e9",
  };
}

export default async function RootLayout({
  children,
}: Readonly<{
  children: ReactNode;
}>) {
  const tenantContext = await getServerTenantContext();
  const themeStyle = buildTenantThemeStyle(tenantContext);

  return (
    <html lang="en" style={themeStyle}>
      <body>
        <Providers initialTenantContext={tenantContext}>{children}</Providers>
      </body>
    </html>
  );
}
