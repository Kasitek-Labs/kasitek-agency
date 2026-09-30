"use client";

import type { ReactNode } from "react";
import { BrainCircuit } from "lucide-react";

import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { cn } from "@/lib/utils";
import { useTenantContext } from "@/lib/tenant-context-provider";

export function TenantBrandMark({
  className,
  compact = false,
}: {
  className?: string;
  compact?: boolean;
}) {
  const { brandName, logoUrl } = useTenantContext();
  const initials = brandName
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase())
    .join("");

  return (
    <Avatar className={cn(compact ? "h-10 w-10" : "h-12 w-12", className)}>
      {logoUrl ? (
        <AvatarImage src={logoUrl} alt={brandName} referrerPolicy="no-referrer" />
      ) : null}
      <AvatarFallback className="bg-primary/10 text-primary">
        {initials || <BrainCircuit className="h-5 w-5" />}
      </AvatarFallback>
    </Avatar>
  );
}

export function TenantBrandName({
  subtitle,
  className,
}: {
  subtitle?: ReactNode;
  className?: string;
}) {
  const { brandName } = useTenantContext();

  return (
    <div className={cn("min-w-0", className)}>
      <p className="truncate text-base font-semibold">{brandName}</p>
      {subtitle ? <p className="truncate text-xs text-muted-foreground">{subtitle}</p> : null}
    </div>
  );
}
