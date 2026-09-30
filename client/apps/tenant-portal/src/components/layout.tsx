"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import {
  LayoutDashboard,
  Users,
  PenTool,
  BarChart3,
  Mail,
  SearchCheck,
  Mic,
  Star,
  Megaphone,
  LogOut,
  Building2,
  Settings,
} from "lucide-react";
import { useAuth } from "@/lib/auth";
import { Button } from "@/components/ui/button";
import { TENANT_ROUTE_ACCESS, hasAnyTenantRole } from "@/lib/tenant-access";
import { TenantBrandMark, TenantBrandName } from "@/components/tenant-brand";
import { useTenantContext } from "@/lib/tenant-context-provider";
import type { TenantFeatureKey } from "@/lib/tenant-context";

const navItems: Array<{
  href: string;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  allowedRoles: readonly string[];
  featureKey: TenantFeatureKey;
}> = [
  { href: "/", label: "Dashboard", icon: LayoutDashboard, allowedRoles: TENANT_ROUTE_ACCESS.dashboard, featureKey: "dashboard" },
  { href: "/clients", label: "Clients", icon: Building2, allowedRoles: TENANT_ROUTE_ACCESS.clients, featureKey: "clients" },
  { href: "/leads", label: "Leads", icon: Users, allowedRoles: TENANT_ROUTE_ACCESS.leads, featureKey: "leads" },
  { href: "/content", label: "Content Pipeline", icon: PenTool, allowedRoles: TENANT_ROUTE_ACCESS.content, featureKey: "content" },
  { href: "/reporting", label: "Client Reporting", icon: BarChart3, allowedRoles: TENANT_ROUTE_ACCESS.reporting, featureKey: "reporting" },
  { href: "/nurturing", label: "Nurturing", icon: Mail, allowedRoles: TENANT_ROUTE_ACCESS.nurturing, featureKey: "nurturing" },
  { href: "/aeo", label: "AEO", icon: SearchCheck, allowedRoles: TENANT_ROUTE_ACCESS.aeo, featureKey: "aeo" },
  { href: "/voice", label: "Voice Agents", icon: Mic, allowedRoles: TENANT_ROUTE_ACCESS.voice, featureKey: "voice" },
  { href: "/reviews", label: "Reputation", icon: Star, allowedRoles: TENANT_ROUTE_ACCESS.reviews, featureKey: "reviews" },
  { href: "/adcreative", label: "Ad Creatives", icon: Megaphone, allowedRoles: TENANT_ROUTE_ACCESS.adcreative, featureKey: "adcreative" },
  { href: "/settings", label: "Settings", icon: Settings, allowedRoles: TENANT_ROUTE_ACCESS.settings, featureKey: "settings" },
];

export function Layout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const { logout, user } = useAuth();
  const { features } = useTenantContext();
  const visibleNavItems = navItems.filter(
    (item) => features[item.featureKey] && hasAnyTenantRole(user?.roles, [...item.allowedRoles]),
  );

  return (
    <div className="flex h-screen bg-background text-foreground overflow-hidden">
      {/* Sidebar */}
      <aside className="w-64 border-r border-border bg-sidebar flex flex-col h-full shrink-0">
        <div className="h-16 flex items-center px-6 border-b border-border">
          <div className="mr-3">
            <TenantBrandMark compact />
          </div>
          <TenantBrandName subtitle={user?.workspaceSlug} />
        </div>

        <div className="flex-1 overflow-y-auto py-4">
          <nav className="space-y-1 px-3">
            {visibleNavItems.map((item) => {
              const isActive =
                pathname === item.href || (item.href !== "/" && pathname.startsWith(item.href));
              const Icon = item.icon;

              return (
                <Link key={item.href} href={item.href}>
                  <div className={`flex items-center px-3 py-2.5 rounded-md cursor-pointer transition-colors ${isActive ? 'bg-primary/10 text-primary font-medium' : 'text-sidebar-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground'}`}>
                    <Icon className={`w-5 h-5 mr-3 ${isActive ? 'text-primary' : 'text-muted-foreground'}`} />
                    {item.label}
                  </div>
                </Link>
              );
            })}
          </nav>
        </div>

        <div className="p-4 border-t border-border">
          <Button
            variant="ghost"
            className="w-full justify-start text-muted-foreground hover:text-foreground"
            onClick={() => void logout()}
          >
            <LogOut className="w-5 h-5 mr-3" />
            Logout
          </Button>
        </div>
      </aside>

      {/* Main Content */}
      <main className="flex-1 overflow-y-auto bg-background/50 relative">
        {children}
      </main>
    </div>
  );
}
