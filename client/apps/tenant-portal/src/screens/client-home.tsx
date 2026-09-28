"use client";

import Link from "next/link";
import { MessageSquare, ShieldCheck, Sparkles, Users } from "lucide-react";
import {
  useGetClientLeadStats,
  useGetClientOverview,
  useListClientConversations,
} from "@workspace/api-client-react";
import { ProtectedRoute } from "@/lib/auth";
import { ClientLayout } from "@/components/client-layout";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { Badge } from "@/components/ui/badge";
import { useTenantContext } from "@/lib/tenant-context-provider";
import { TenantAnalyticsPanel } from "@/components/tenant-analytics-panel";

export default function ClientHomeScreen() {
  const { data: overview, isLoading: loadingOverview } = useGetClientOverview();
  const { data: conversations, isLoading: loadingConversations } = useListClientConversations();
  const { data: leadStats, isLoading: loadingLeadStats } = useGetClientLeadStats();
  const { brandName } = useTenantContext();

  return (
    <ProtectedRoute audience="client">
      <ClientLayout>
        <div className="space-y-8">
          <div>
            <p className="text-sm font-medium uppercase tracking-[0.18em] text-primary">
              Client Portal
            </p>
            <h1 className="mt-2 text-4xl font-semibold tracking-tight">
              {overview?.tenantClientName ?? brandName}
            </h1>
            <p className="mt-3 max-w-2xl text-muted-foreground">
              Your client-facing portal is now connected to tenant-scoped data. Only records linked
              to this client account are visible here.
            </p>
          </div>

          <div className="grid gap-6 md:grid-cols-2 xl:grid-cols-5">
            {loadingOverview || loadingLeadStats ? (
              Array.from({ length: 5 }).map((_, index) => (
                <Skeleton key={index} className="h-32 rounded-xl" />
              ))
            ) : (
              <>
                <Card>
                  <CardHeader className="pb-2">
                    <Users className="h-4 w-4 text-primary" />
                    <CardTitle className="text-sm">Users</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="text-3xl font-semibold">{overview?.totalUsers ?? 0}</div>
                  </CardContent>
                </Card>
                <Card>
                  <CardHeader className="pb-2">
                    <MessageSquare className="h-4 w-4 text-primary" />
                    <CardTitle className="text-sm">Conversations</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="text-3xl font-semibold">{overview?.totalConversations ?? 0}</div>
                  </CardContent>
                </Card>
                <Card>
                  <CardHeader className="pb-2">
                    <ShieldCheck className="h-4 w-4 text-primary" />
                    <CardTitle className="text-sm">Open Conversations</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="text-3xl font-semibold">{overview?.openConversations ?? 0}</div>
                  </CardContent>
                </Card>
                <Card>
                  <CardHeader className="pb-2">
                    <Sparkles className="h-4 w-4 text-primary" />
                    <CardTitle className="text-sm">Leads</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="text-3xl font-semibold">{leadStats?.total ?? 0}</div>
                  </CardContent>
                </Card>
                <Card>
                  <CardHeader className="pb-2">
                    <Sparkles className="h-4 w-4 text-primary" />
                    <CardTitle className="text-sm">Messages</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="text-3xl font-semibold">{overview?.totalMessages ?? 0}</div>
                  </CardContent>
                </Card>
              </>
            )}
          </div>

          <TenantAnalyticsPanel audience="client" />

          <Card>
            <CardHeader>
              <CardTitle>Quick Access</CardTitle>
              <CardDescription>Jump into the available client-facing areas.</CardDescription>
            </CardHeader>
            <CardContent className="grid gap-3 md:grid-cols-2">
              <Link
                href="/client/leads"
                className="rounded-xl border border-border px-4 py-4 transition-colors hover:border-primary/40 hover:bg-muted/40"
              >
                <div className="font-medium">Lead Register</div>
                <p className="mt-1 text-sm text-muted-foreground">
                  Review your client-scoped leads and their current statuses.
                </p>
              </Link>
              <Link
                href="/client/conversations"
                className="rounded-xl border border-border px-4 py-4 transition-colors hover:border-primary/40 hover:bg-muted/40"
              >
                <div className="font-medium">Conversation Hub</div>
                <p className="mt-1 text-sm text-muted-foreground">
                  Inspect messages and recent activity tied to this client account.
                </p>
              </Link>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>Recent Conversations</CardTitle>
              <CardDescription>Latest client-scoped communication activity.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              {loadingConversations ? (
                Array.from({ length: 4 }).map((_, index) => (
                  <Skeleton key={index} className="h-20 w-full rounded-xl" />
                ))
              ) : conversations?.length ? (
                conversations.slice(0, 5).map((conversation) => (
                  <Link
                    key={conversation.id}
                    href={`/client/conversations/${conversation.id}`}
                    className="flex flex-col gap-3 rounded-xl border border-border bg-card px-4 py-4 transition-colors hover:border-primary/40 hover:bg-muted/40 lg:flex-row lg:items-center lg:justify-between"
                  >
                    <div className="min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="font-medium">
                          {conversation.customer_name || conversation.customer_identifier}
                        </span>
                        <Badge variant="outline">{conversation.channel}</Badge>
                      </div>
                      <p className="mt-1 truncate text-sm text-muted-foreground">
                        {conversation.status}
                      </p>
                    </div>
                    <p className="text-sm text-muted-foreground">
                      {new Date(conversation.last_message_at).toLocaleString()}
                    </p>
                  </Link>
                ))
              ) : (
                <div className="rounded-xl border border-dashed border-border px-6 py-10 text-center text-muted-foreground">
                  No client-scoped conversations yet.
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      </ClientLayout>
    </ProtectedRoute>
  );
}
