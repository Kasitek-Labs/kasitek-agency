"use client";

import { useMemo, useState } from "react";
import { useGetClientLeadStats, useListClientLeads } from "@workspace/api-client-react";
import { ClientLayout } from "@/components/client-layout";
import { ProtectedRoute } from "@/lib/auth";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Search, Target, Trophy, Users } from "lucide-react";

function statusTone(status: string) {
  switch (status) {
    case "new":
      return "bg-blue-500/10 text-blue-600 border-blue-500/20";
    case "qualified":
      return "bg-purple-500/10 text-purple-600 border-purple-500/20";
    case "converted":
      return "bg-green-500/10 text-green-600 border-green-500/20";
    case "lost":
      return "bg-red-500/10 text-red-600 border-red-500/20";
    default:
      return "bg-amber-500/10 text-amber-600 border-amber-500/20";
  }
}

export default function ClientLeadsScreen() {
  const [search, setSearch] = useState("");
  const { data: leads, isLoading: loadingLeads } = useListClientLeads();
  const { data: stats, isLoading: loadingStats } = useGetClientLeadStats();

  const filteredLeads = useMemo(() => {
    return (leads ?? []).filter((lead) => {
      const haystack = [lead.name, lead.email, lead.company, lead.source, lead.status]
        .filter(Boolean)
        .join(" ")
        .toLowerCase();

      return haystack.includes(search.toLowerCase());
    });
  }, [leads, search]);

  return (
    <ProtectedRoute audience="client">
      <ClientLayout>
        <div className="space-y-6">
          <div>
            <h1 className="text-3xl font-semibold tracking-tight">Leads</h1>
            <p className="mt-2 text-muted-foreground">
              View the leads captured for your client account inside this tenant portal.
            </p>
          </div>

          <div className="grid gap-6 md:grid-cols-3">
            {loadingStats ? (
              Array.from({ length: 3 }).map((_, index) => (
                <Skeleton key={index} className="h-28 rounded-xl" />
              ))
            ) : (
              <>
                <Card>
                  <CardHeader className="pb-2">
                    <Users className="h-4 w-4 text-primary" />
                    <CardTitle className="text-sm">Total Leads</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="text-3xl font-semibold">{stats?.total ?? 0}</div>
                  </CardContent>
                </Card>
                <Card>
                  <CardHeader className="pb-2">
                    <Target className="h-4 w-4 text-primary" />
                    <CardTitle className="text-sm">Qualified</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="text-3xl font-semibold">{stats?.qualified ?? 0}</div>
                  </CardContent>
                </Card>
                <Card>
                  <CardHeader className="pb-2">
                    <Trophy className="h-4 w-4 text-primary" />
                    <CardTitle className="text-sm">Converted</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="text-3xl font-semibold">{stats?.converted ?? 0}</div>
                  </CardContent>
                </Card>
              </>
            )}
          </div>

          <Card>
            <CardHeader>
              <CardTitle>Lead Register</CardTitle>
              <CardDescription>Only leads assigned to your client account are visible here.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="relative max-w-md">
                <Search className="absolute left-3 top-3 h-4 w-4 text-muted-foreground" />
                <Input
                  className="pl-9"
                  placeholder="Search leads"
                  value={search}
                  onChange={(event) => setSearch(event.target.value)}
                />
              </div>

              {loadingLeads ? (
                Array.from({ length: 5 }).map((_, index) => (
                  <Skeleton key={index} className="h-24 rounded-xl" />
                ))
              ) : filteredLeads.length ? (
                <div className="space-y-3">
                  {filteredLeads.map((lead) => (
                    <div key={lead.id} className="rounded-xl border border-border px-4 py-4">
                      <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
                        <div className="min-w-0">
                          <div className="flex flex-wrap items-center gap-2">
                            <span className="font-medium">{lead.name}</span>
                            <Badge variant="outline" className={statusTone(lead.status)}>
                              {lead.status}
                            </Badge>
                            <Badge variant="secondary">{lead.score}</Badge>
                          </div>
                          <div className="mt-2 text-sm text-muted-foreground">
                            <div>{lead.email}</div>
                            {lead.company && <div>{lead.company}</div>}
                            <div className="capitalize">{lead.source.replaceAll("_", " ")}</div>
                          </div>
                        </div>
                        <div className="text-sm text-muted-foreground">
                          {new Date(lead.created_at).toLocaleDateString()}
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              ) : (
                <div className="rounded-xl border border-dashed border-border px-6 py-10 text-center text-muted-foreground">
                  No leads matched your search.
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      </ClientLayout>
    </ProtectedRoute>
  );
}
