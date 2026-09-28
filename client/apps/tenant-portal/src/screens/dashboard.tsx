import { ProtectedRoute } from "@/lib/auth";
import { Layout } from "@/components/layout";
import { useGetDashboardSummary, useGetDashboardActivity, useListServices } from "@workspace/api-client-react";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Activity, BarChart, Users, Zap } from "lucide-react";
import { TENANT_ROUTE_ACCESS } from "@/lib/tenant-access";

export default function Dashboard() {
  const { data: summary, isLoading: loadingSummary } = useGetDashboardSummary();
  const { data: activity, isLoading: loadingActivity } = useGetDashboardActivity();
  const { data: services, isLoading: loadingServices } = useListServices();

  return (
    <ProtectedRoute allowedRoles={TENANT_ROUTE_ACCESS.dashboard}>
      <Layout>
        <div className="p-8 space-y-8 animate-in fade-in duration-500">
          <div className="flex justify-between items-end">
            <div>
              <h1 className="text-3xl font-bold tracking-tight">Overview</h1>
              <p className="text-muted-foreground mt-1">Platform overview and active services.</p>
            </div>
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <span className="flex h-2 w-2 rounded-full bg-green-500"></span>
              All systems operational
            </div>
          </div>

          {/* Stats Grid */}
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
            {loadingSummary ? (
              Array.from({ length: 4 }).map((_, i) => (
                <Skeleton key={i} className="h-32 rounded-xl" />
              ))
            ) : (
              <>
                <Card className="bg-card border-border/50">
                  <CardHeader className="pb-2 flex flex-row items-center justify-between space-y-0">
                    <CardTitle className="text-sm font-medium text-muted-foreground">Total Clients</CardTitle>
                    <Users className="w-4 h-4 text-muted-foreground" />
                  </CardHeader>
                  <CardContent>
                    <div className="text-3xl font-bold">{summary?.totalClients || 0}</div>
                    <p className="text-xs text-muted-foreground mt-1">Active accounts in this tenant</p>
                  </CardContent>
                </Card>
                <Card className="bg-card border-border/50">
                  <CardHeader className="pb-2 flex flex-row items-center justify-between space-y-0">
                    <CardTitle className="text-sm font-medium text-muted-foreground">Active Agents</CardTitle>
                    <BarChart className="w-4 h-4 text-muted-foreground" />
                  </CardHeader>
                  <CardContent>
                    <div className="text-3xl font-bold">{summary?.activeAgents || 0}</div>
                  </CardContent>
                </Card>
                <Card className="bg-card border-border/50">
                  <CardHeader className="pb-2 flex flex-row items-center justify-between space-y-0">
                    <CardTitle className="text-sm font-medium text-muted-foreground">Conversations</CardTitle>
                    <Zap className="w-4 h-4 text-muted-foreground" />
                  </CardHeader>
                  <CardContent>
                    <div className="text-3xl font-bold">{summary?.totalConversations || 0}</div>
                  </CardContent>
                </Card>
                <Card className="bg-card border-border/50">
                  <CardHeader className="pb-2 flex flex-row items-center justify-between space-y-0">
                    <CardTitle className="text-sm font-medium text-muted-foreground">Requests This Month</CardTitle>
                    <Activity className="w-4 h-4 text-muted-foreground" />
                  </CardHeader>
                  <CardContent>
                    <div className="text-3xl font-bold text-primary">{summary?.requestsThisMonth || 0}</div>
                  </CardContent>
                </Card>
              </>
            )}
          </div>

          <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
            {/* Activity Feed */}
            <Card className="lg:col-span-2 border-border/50 bg-card">
              <CardHeader>
                <CardTitle>Recent Activity</CardTitle>
                <CardDescription>Latest events across your AI stack</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-4">
                  {loadingActivity ? (
                    Array.from({ length: 5 }).map((_, i) => (
                      <Skeleton key={i} className="h-16 w-full" />
                    ))
                  ) : activity && activity.length > 0 ? (
                    activity.map((item) => (
                      <div key={item.id} className="flex items-start space-x-4 p-3 rounded-lg hover:bg-muted/50 transition-colors">
                        <div className="mt-1 bg-primary/10 p-2 rounded-full">
                          <Activity className="w-4 h-4 text-primary" />
                        </div>
                        <div className="flex-1 space-y-1">
                          <p className="text-sm font-medium leading-none">{item.title}</p>
                          <p className="text-sm text-muted-foreground">{item.description}</p>
                        </div>
                        <div className="text-xs text-muted-foreground">
                          {new Date(item.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
                        </div>
                      </div>
                    ))
                  ) : (
                    <div className="text-center py-8 text-muted-foreground">No recent activity</div>
                  )}
                </div>
              </CardContent>
            </Card>

            {/* Services Status */}
            <Card className="border-border/50 bg-card">
              <CardHeader>
                <CardTitle>Services</CardTitle>
                <CardDescription>Active platform modules</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="space-y-4">
                  {loadingServices ? (
                    Array.from({ length: 4 }).map((_, i) => (
                      <Skeleton key={i} className="h-12 w-full" />
                    ))
                  ) : services && services.length > 0 ? (
                    services.map((service) => (
                      <div key={service.id} className="flex items-center justify-between border-b border-border pb-3 last:border-0 last:pb-0">
                        <div className="flex items-center space-x-3">
                          <div className={`w-2 h-2 rounded-full ${service.isActive ? 'bg-green-500' : 'bg-muted'}`} />
                          <span className="font-medium text-sm">{service.name}</span>
                        </div>
                        {service.isActive ? (
                          <Badge variant="outline" className="text-xs text-primary border-primary/20 bg-primary/10">Active</Badge>
                        ) : (
                          <Badge variant="outline" className="text-xs">Inactive</Badge>
                        )}
                      </div>
                    ))
                  ) : (
                    <div className="text-center py-8 text-muted-foreground">No services configured</div>
                  )}
                </div>
              </CardContent>
            </Card>
          </div>
        </div>
      </Layout>
    </ProtectedRoute>
  );
}
