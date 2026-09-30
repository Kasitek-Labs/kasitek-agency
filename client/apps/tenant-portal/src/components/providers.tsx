"use client";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { TooltipProvider } from "@/components/ui/tooltip";
import { Toaster } from "@/components/ui/toaster";
import { AuthProvider } from "@/lib/auth";
import { TenantContextProvider } from "@/lib/tenant-context-provider";
import type { PublicTenantContext } from "@/lib/tenant-context";
import { useState } from "react";

export default function Providers({
  children,
  initialTenantContext,
}: {
  children: React.ReactNode;
  initialTenantContext?: PublicTenantContext | null;
}) {
  const [queryClient] = useState(() => new QueryClient());

  return (
    <QueryClientProvider client={queryClient}>
      <TenantContextProvider initialContext={initialTenantContext}>
        <AuthProvider>
          <TooltipProvider>
            {children}
            <Toaster />
          </TooltipProvider>
        </AuthProvider>
      </TenantContextProvider>
    </QueryClientProvider>
  );
}
