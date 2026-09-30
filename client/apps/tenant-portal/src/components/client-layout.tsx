"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { LayoutDashboard, LogOut, MessageSquare, Users } from "lucide-react";
import { useAuth } from "@/lib/auth";
import { Button } from "@/components/ui/button";
import { TenantBrandMark } from "@/components/tenant-brand";
import { useTenantContext } from "@/lib/tenant-context-provider";

const navItems = [
  { href: "/client", label: "Overview", icon: LayoutDashboard },
  { href: "/client/leads", label: "Leads", icon: Users },
  { href: "/client/conversations", label: "Conversations", icon: MessageSquare },
];

export function ClientLayout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const { clientUser, logout } = useAuth();
  const { brandName } = useTenantContext();

  return (
    <div className="min-h-screen bg-background text-foreground">
      <header className="border-b border-border bg-card/80 backdrop-blur">
        <div className="mx-auto flex max-w-6xl items-center justify-between gap-6 px-6 py-4">
          <div className="min-w-0">
            <div className="flex items-center gap-3">
              <TenantBrandMark compact />
              <div className="min-w-0">
                <p className="truncate text-base font-semibold">{brandName}</p>
                <p className="truncate text-xs text-muted-foreground">
                  {clientUser?.tenantClientName || "Client workspace"}
                </p>
              </div>
            </div>
            <p className="truncate text-xs text-muted-foreground">
              {clientUser?.name || clientUser?.email || "Authenticated client user"}
            </p>
          </div>
          <Button variant="outline" size="sm" onClick={() => void logout()}>
            <LogOut className="mr-2 h-4 w-4" />
            Sign Out
          </Button>
        </div>
      </header>

      <div className="mx-auto grid max-w-6xl gap-8 px-6 py-8 lg:grid-cols-[220px,1fr]">
        <aside className="space-y-2">
          {navItems.map((item) => {
            const isActive =
              pathname === item.href || (item.href !== "/client" && pathname.startsWith(item.href));
            const Icon = item.icon;

            return (
              <Link
                key={item.href}
                href={item.href}
                className={`flex items-center gap-3 rounded-xl px-3 py-2.5 text-sm transition-colors ${
                  isActive
                    ? "bg-primary/10 font-medium text-primary"
                    : "text-muted-foreground hover:bg-muted hover:text-foreground"
                }`}
              >
                <Icon className="h-4 w-4" />
                {item.label}
              </Link>
            );
          })}
        </aside>

        <main>{children}</main>
      </div>
    </div>
  );
}
