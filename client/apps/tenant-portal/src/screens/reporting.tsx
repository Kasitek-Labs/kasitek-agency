import { useState } from "react";
import { useListReportingDashboards, useCreateReportingDashboard, useGetDashboardMetrics } from "@workspace/api-client-react";
import { ProtectedRoute } from "@/lib/auth";
import { Layout } from "@/components/layout";
import { Card, CardContent, CardHeader, CardTitle, CardDescription, CardFooter } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Form, FormControl, FormField, FormItem, FormLabel, FormMessage } from "@/components/ui/form";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { zodResolver } from "@hookform/resolvers/zod";
import { Plus, BarChart3, ExternalLink, Activity, Users, MousePointerClick, DollarSign } from "lucide-react";
import { useQueryClient } from "@tanstack/react-query";
import { getListReportingDashboardsQueryKey } from "@workspace/api-client-react";
import { Badge } from "@/components/ui/badge";
import { AreaChart, Area, XAxis, YAxis, Tooltip as RechartsTooltip, ResponsiveContainer, CartesianGrid } from "recharts";
import { TENANT_ROUTE_ACCESS } from "@/lib/tenant-access";
import { TenantAnalyticsPanel } from "@/components/tenant-analytics-panel";
import { useAuth } from "@/lib/auth";

const createDashboardSchema = z.object({
  name: z.string().min(1, "Name is required"),
  clientName: z.string().min(1, "Client name is required"),
  brandColor: z.string().regex(/^#([0-9A-F]{3}){1,2}$/i, "Must be a valid hex color code"),
  isPublic: z.boolean().default(false),
  logoUrl: z.string().url().optional().or(z.literal("")),
});

export default function Reporting() {
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const [selectedDashboardId, setSelectedDashboardId] = useState<string | null>(null);
  const queryClient = useQueryClient();
  const { user } = useAuth();

  const { data: dashboards, isLoading: loadingDashboards } = useListReportingDashboards();
  const createDashboard = useCreateReportingDashboard();

  // Pick first dashboard id if not selected
  const activeDashboardId = selectedDashboardId || (dashboards && dashboards.length > 0 ? dashboards[0].id : null);
  const { data: metrics, isLoading: loadingMetrics } = useGetDashboardMetrics(activeDashboardId || "", {
    query: { enabled: !!activeDashboardId }
  });

  const form = useForm<z.infer<typeof createDashboardSchema>>({
    resolver: zodResolver(createDashboardSchema),
    defaultValues: {
      name: "",
      clientName: "",
      brandColor: "#0ea5e9",
      isPublic: false,
      logoUrl: "",
    },
  });

  const onSubmit = (values: z.infer<typeof createDashboardSchema>) => {
    createDashboard.mutate(
      { data: values },
      {
        onSuccess: () => {
          queryClient.invalidateQueries({ queryKey: getListReportingDashboardsQueryKey() });
          setIsDialogOpen(false);
          form.reset();
        },
      }
    );
  };

  const activeDashboard = dashboards?.find(d => d.id === activeDashboardId);

  return (
    <ProtectedRoute allowedRoles={TENANT_ROUTE_ACCESS.reporting}>
      <Layout>
        <div className="p-8 space-y-8 animate-in fade-in duration-500">
          <TenantAnalyticsPanel audience="workspace" includeDescendants={user?.roles.some((role) => ["owner", "admin", "account_manager"].includes(role))} />
          <div className="flex justify-between items-end">
            <div>
              <h1 className="text-3xl font-bold tracking-tight">Client Reporting</h1>
              <p className="text-muted-foreground mt-1">Manage branded performance dashboards for your clients.</p>
            </div>
            <Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
              <DialogTrigger asChild>
                <Button>
                  <Plus className="w-4 h-4 mr-2" />
                  New Dashboard
                </Button>
              </DialogTrigger>
              <DialogContent className="sm:max-w-[425px]">
                <DialogHeader>
                  <DialogTitle>Create Client Dashboard</DialogTitle>
                  <DialogDescription>
                    Configure a new branded dashboard.
                  </DialogDescription>
                </DialogHeader>
                <Form {...form}>
                  <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
                    <FormField
                      control={form.control}
                      name="clientName"
                      render={({ field }) => (
                         <FormItem>
                          <FormLabel>Client Name</FormLabel>
                          <FormControl>
                            <Input placeholder="Northstar Agency" {...field} />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                    <FormField
                      control={form.control}
                      name="name"
                      render={({ field }) => (
                         <FormItem>
                          <FormLabel>Dashboard Title</FormLabel>
                          <FormControl>
                            <Input placeholder="Q3 Performance" {...field} />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                    <FormField
                      control={form.control}
                      name="brandColor"
                      render={({ field }) => (
                         <FormItem>
                          <FormLabel>Brand Color (Hex)</FormLabel>
                          <div className="flex space-x-2">
                            <div className="w-10 h-10 rounded-md border border-border" style={{ backgroundColor: field.value }}></div>
                            <FormControl>
                              <Input placeholder="#000000" {...field} />
                            </FormControl>
                          </div>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                    <FormField
                      control={form.control}
                      name="logoUrl"
                      render={({ field }) => (
                         <FormItem>
                          <FormLabel>Logo URL (Optional)</FormLabel>
                          <FormControl>
                            <Input placeholder="https://..." {...field} value={field.value || ""} />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                    <FormField
                      control={form.control}
                      name="isPublic"
                      render={({ field }) => (
                         <FormItem className="flex flex-row items-center justify-between rounded-lg border border-border p-4">
                          <div className="space-y-0.5">
                            <FormLabel className="text-base">Public Access</FormLabel>
                            <div className="text-sm text-muted-foreground">
                              Allow access via secure link
                            </div>
                          </div>
                          <FormControl>
                            <Switch checked={field.value} onCheckedChange={field.onChange} />
                          </FormControl>
                        </FormItem>
                      )}
                    />
                    <div className="flex justify-end pt-4">
                      <Button type="submit" loading={createDashboard.isPending} loadingLabel="Creating dashboard...">
                        Create Dashboard
                      </Button>
                    </div>
                  </form>
                </Form>
              </DialogContent>
            </Dialog>
          </div>

          <div className="grid grid-cols-1 xl:grid-cols-4 gap-8">
            <div className="xl:col-span-1 space-y-4">
              <h3 className="font-medium text-sm text-muted-foreground px-1">Your Dashboards</h3>
              <div className="space-y-2">
                {loadingDashboards ? (
                  Array.from({ length: 4 }).map((_, i) => (
                    <Skeleton key={i} className="h-20 w-full" />
                  ))
                ) : dashboards && dashboards.length > 0 ? (
                  dashboards.map((dashboard) => (
                    <div
                      key={dashboard.id}
                      onClick={() => setSelectedDashboardId(dashboard.id)}
                      className={`p-4 rounded-lg border cursor-pointer transition-all ${
                        activeDashboardId === dashboard.id
                          ? 'bg-muted border-primary shadow-sm'
                          : 'bg-card border-border hover:border-primary/50 hover:bg-muted/50'
                      }`}
                    >
                      <div className="flex items-center justify-between mb-2">
                        <div className="font-semibold">{dashboard.clientName}</div>
                        {dashboard.isPublic && <Badge variant="secondary" className="text-[10px]">Public</Badge>}
                      </div>
                      <div className="text-sm text-muted-foreground">{dashboard.name}</div>
                      <div className="flex items-center mt-3 text-xs text-muted-foreground">
                        <div className="w-3 h-3 rounded-full mr-2" style={{ backgroundColor: dashboard.brandColor }}></div>
                        {dashboard.viewCount} views
                      </div>
                    </div>
                  ))
                ) : (
                  <div className="text-center py-8 text-muted-foreground bg-card border border-border border-dashed rounded-lg">
                    No dashboards found
                  </div>
                )}
              </div>
            </div>

            <div className="xl:col-span-3">
              {activeDashboard ? (
                <div className="space-y-6">
                  <div className="flex items-center justify-between p-6 bg-card border border-border rounded-xl">
                    <div className="flex items-center space-x-4">
                      {activeDashboard.logoUrl ? (
                         <div className="w-12 h-12 rounded bg-background flex items-center justify-center overflow-hidden border border-border">
                           <img src={activeDashboard.logoUrl} alt={activeDashboard.clientName} className="max-w-full max-h-full object-contain" />
                         </div>
                      ) : (
                        <div className="w-12 h-12 rounded bg-background flex items-center justify-center font-bold text-xl border border-border" style={{ color: activeDashboard.brandColor }}>
                          {activeDashboard.clientName.substring(0, 2).toUpperCase()}
                        </div>
                      )}
                      <div>
                        <h2 className="text-2xl font-bold">{activeDashboard.clientName}</h2>
                        <div className="text-muted-foreground">{activeDashboard.name}</div>
                      </div>
                    </div>
                    {activeDashboard.isPublic && (
                      <Button variant="outline" className="gap-2">
                        <ExternalLink className="w-4 h-4" />
                        View Public Link
                      </Button>
                    )}
                  </div>

                  {loadingMetrics ? (
                    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
                      {Array.from({ length: 4 }).map((_, i) => <Skeleton key={i} className="h-28 w-full" />)}
                    </div>
                  ) : metrics ? (
                    <>
                      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
                        <Card>
                          <CardHeader className="pb-2 flex flex-row items-center justify-between space-y-0">
                            <CardTitle className="text-sm font-medium text-muted-foreground">Total Sessions</CardTitle>
                            <Activity className="w-4 h-4 text-muted-foreground" />
                          </CardHeader>
                          <CardContent>
                            <div className="text-2xl font-bold">{metrics.sessions.toLocaleString()}</div>
                          </CardContent>
                        </Card>
                        <Card>
                          <CardHeader className="pb-2 flex flex-row items-center justify-between space-y-0">
                            <CardTitle className="text-sm font-medium text-muted-foreground">Leads Generated</CardTitle>
                            <Users className="w-4 h-4 text-muted-foreground" />
                          </CardHeader>
                          <CardContent>
                            <div className="text-2xl font-bold">{metrics.leads.toLocaleString()}</div>
                          </CardContent>
                        </Card>
                        <Card>
                          <CardHeader className="pb-2 flex flex-row items-center justify-between space-y-0">
                            <CardTitle className="text-sm font-medium text-muted-foreground">Conversions</CardTitle>
                            <MousePointerClick className="w-4 h-4 text-muted-foreground" />
                          </CardHeader>
                          <CardContent>
                            <div className="text-2xl font-bold">{metrics.conversions.toLocaleString()}</div>
                          </CardContent>
                        </Card>
                        <Card>
                          <CardHeader className="pb-2 flex flex-row items-center justify-between space-y-0">
                            <CardTitle className="text-sm font-medium text-muted-foreground">Est. Revenue</CardTitle>
                            <DollarSign className="w-4 h-4 text-muted-foreground" />
                          </CardHeader>
                          <CardContent>
                            <div className="text-2xl font-bold text-green-500">${metrics.revenue.toLocaleString()}</div>
                          </CardContent>
                        </Card>
                      </div>

                      <Card>
                        <CardHeader>
                          <CardTitle>Performance Trend</CardTitle>
                        </CardHeader>
                        <CardContent className="h-[400px]">
                          <ResponsiveContainer width="100%" height="100%">
                            <AreaChart data={metrics.monthlyTrend} margin={{ top: 10, right: 30, left: 0, bottom: 0 }}>
                              <defs>
                                <linearGradient id="colorSessions" x1="0" y1="0" x2="0" y2="1">
                                  <stop offset="5%" stopColor={activeDashboard.brandColor} stopOpacity={0.3}/>
                                  <stop offset="95%" stopColor={activeDashboard.brandColor} stopOpacity={0}/>
                                </linearGradient>
                              </defs>
                              <XAxis dataKey="month" stroke="#888888" fontSize={12} tickLine={false} axisLine={false} />
                              <YAxis stroke="#888888" fontSize={12} tickLine={false} axisLine={false} />
                              <CartesianGrid strokeDasharray="3 3" vertical={false} stroke="rgba(255,255,255,0.1)" />
                              <RechartsTooltip contentStyle={{ backgroundColor: 'hsl(var(--card))', borderColor: 'hsl(var(--border))' }} />
                              <Area type="monotone" dataKey="sessions" stroke={activeDashboard.brandColor} fillOpacity={1} fill="url(#colorSessions)" />
                            </AreaChart>
                          </ResponsiveContainer>
                        </CardContent>
                      </Card>
                    </>
                  ) : null}
                </div>
              ) : (
                <div className="h-full min-h-[400px] flex flex-col items-center justify-center bg-card border border-border border-dashed rounded-xl text-muted-foreground">
                  <BarChart3 className="w-12 h-12 mb-4 opacity-20" />
                  <p>Select or create a dashboard to view performance metrics.</p>
                </div>
              )}
            </div>
          </div>
        </div>
      </Layout>
    </ProtectedRoute>
  );
}
