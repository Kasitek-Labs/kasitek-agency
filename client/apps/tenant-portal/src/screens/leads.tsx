"use client";

import { useMemo, useState } from "react";
import {
  getGetLeadsStatsQueryKey,
  getListLeadsQueryKey,
  useCreateLead,
  useGetLeadsStats,
  useListLeads,
  useListTenantClients,
} from "@workspace/api-client-react";
import { useQueryClient } from "@tanstack/react-query";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { zodResolver } from "@hookform/resolvers/zod";
import { BarChart, Bar, Line, LineChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts";
import { Filter, Plus, Search, Target, Trophy, Users } from "lucide-react";
import { Layout } from "@/components/layout";
import { ProtectedRoute, useAuth } from "@/lib/auth";
import { LEAD_OPERATOR_ROLES } from "@/lib/tenant-access";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";

const createLeadSchema = z.object({
  tenantClientId: z.string().min(1, "Client is required"),
  name: z.string().min(1, "Name is required"),
  email: z.string().email("Invalid email address"),
  phone: z.string().optional(),
  company: z.string().optional(),
  source: z.enum(["web_chat", "whatsapp", "instagram", "email", "voice", "manual"]),
  notes: z.string().optional(),
});

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

export default function LeadsScreen() {
  const queryClient = useQueryClient();
  const { user } = useAuth();
  const [searchTerm, setSearchTerm] = useState("");
  const [statusFilter, setStatusFilter] = useState("all");
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const canCreateLead = user?.roles?.some((role) => LEAD_OPERATOR_ROLES.includes(role)) ?? false;

  const { data: leadsData, isLoading: loadingLeads } = useListLeads();
  const { data: stats, isLoading: loadingStats } = useGetLeadsStats();
  const { data: clients } = useListTenantClients();
  const createLead = useCreateLead();

  const form = useForm<z.infer<typeof createLeadSchema>>({
    resolver: zodResolver(createLeadSchema),
    defaultValues: {
      tenantClientId: "",
      name: "",
      email: "",
      phone: "",
      company: "",
      source: "manual",
      notes: "",
    },
  });

  const filteredLeads = useMemo(() => {
    const leads = leadsData?.leads ?? [];
    return leads.filter((lead) => {
      const haystack = [
        lead.name,
        lead.email,
        lead.company,
        lead.tenant_client_name,
        lead.source,
        lead.status,
      ]
        .filter(Boolean)
        .join(" ")
        .toLowerCase();

      const matchesSearch = haystack.includes(searchTerm.toLowerCase());
      const matchesStatus = statusFilter === "all" || lead.status === statusFilter;
      return matchesSearch && matchesStatus;
    });
  }, [leadsData?.leads, searchTerm, statusFilter]);

  const onSubmit = (values: z.infer<typeof createLeadSchema>) => {
    createLead.mutate({
      tenantClientId: values.tenantClientId,
      name: values.name,
      email: values.email,
      phone: values.phone,
      company: values.company,
      source: values.source,
      notes: values.notes,
    }, {
      onSuccess: () => {
        void queryClient.invalidateQueries({ queryKey: getListLeadsQueryKey() });
        void queryClient.invalidateQueries({ queryKey: getGetLeadsStatsQueryKey() });
        setIsDialogOpen(false);
        form.reset({
          tenantClientId: "",
          name: "",
          email: "",
          phone: "",
          company: "",
          source: "manual",
          notes: "",
        });
      },
      onError: (error) => {
        form.setError("root", {
          message: error instanceof Error ? error.message : "Unable to create lead",
        });
      },
    });
  };

  return (
    <ProtectedRoute allowedRoles={LEAD_OPERATOR_ROLES}>
      <Layout>
        <div className="space-y-8 p-8 animate-in fade-in duration-500">
          <div className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
            <div>
              <h1 className="text-3xl font-bold tracking-tight">Lead Pipeline</h1>
              <p className="mt-1 text-muted-foreground">
                Track tenant-wide inbound leads and keep them segmented by client account.
              </p>
            </div>

            {canCreateLead && (
              <Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
                <DialogTrigger asChild>
                  <Button>
                    <Plus className="mr-2 h-4 w-4" />
                    Add Lead
                  </Button>
                </DialogTrigger>
                <DialogContent className="sm:max-w-[480px]">
                  <DialogHeader>
                    <DialogTitle>Create Lead</DialogTitle>
                    <DialogDescription>
                      Add a new lead and assign it to the correct tenant client.
                    </DialogDescription>
                  </DialogHeader>

                  <Form {...form}>
                    <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
                      <FormField
                        control={form.control}
                        name="tenantClientId"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Client</FormLabel>
                            <Select onValueChange={field.onChange} value={field.value}>
                              <FormControl>
                                <SelectTrigger>
                                  <SelectValue placeholder="Select client" />
                                </SelectTrigger>
                              </FormControl>
                              <SelectContent>
                                {clients?.map((client) => (
                                  <SelectItem key={client.id} value={client.id}>
                                    {client.name}
                                  </SelectItem>
                                ))}
                              </SelectContent>
                            </Select>
                            <FormMessage />
                          </FormItem>
                        )}
                      />

                      <div className="grid gap-4 sm:grid-cols-2">
                        <FormField
                          control={form.control}
                          name="name"
                          render={({ field }) => (
                            <FormItem>
                              <FormLabel>Name</FormLabel>
                              <FormControl>
                                <Input placeholder="John Doe" {...field} />
                              </FormControl>
                              <FormMessage />
                            </FormItem>
                          )}
                        />
                        <FormField
                          control={form.control}
                          name="email"
                          render={({ field }) => (
                            <FormItem>
                              <FormLabel>Email</FormLabel>
                              <FormControl>
                                <Input placeholder="john@example.com" {...field} />
                              </FormControl>
                              <FormMessage />
                            </FormItem>
                          )}
                        />
                      </div>

                      <div className="grid gap-4 sm:grid-cols-2">
                        <FormField
                          control={form.control}
                          name="phone"
                          render={({ field }) => (
                            <FormItem>
                              <FormLabel>Phone</FormLabel>
                              <FormControl>
                                <Input placeholder="+27 82 555 0100" {...field} />
                              </FormControl>
                              <FormMessage />
                            </FormItem>
                          )}
                        />
                        <FormField
                          control={form.control}
                          name="company"
                          render={({ field }) => (
                            <FormItem>
                              <FormLabel>Company</FormLabel>
                              <FormControl>
                                <Input placeholder="Northstar Holdings" {...field} />
                              </FormControl>
                              <FormMessage />
                            </FormItem>
                          )}
                        />
                      </div>

                      <FormField
                        control={form.control}
                        name="source"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Source</FormLabel>
                            <Select onValueChange={field.onChange} value={field.value}>
                              <FormControl>
                                <SelectTrigger>
                                  <SelectValue placeholder="Select source" />
                                </SelectTrigger>
                              </FormControl>
                              <SelectContent>
                                <SelectItem value="manual">Manual</SelectItem>
                                <SelectItem value="web_chat">Web Chat</SelectItem>
                                <SelectItem value="whatsapp">WhatsApp</SelectItem>
                                <SelectItem value="instagram">Instagram</SelectItem>
                                <SelectItem value="email">Email</SelectItem>
                                <SelectItem value="voice">Voice</SelectItem>
                              </SelectContent>
                            </Select>
                            <FormMessage />
                          </FormItem>
                        )}
                      />

                      <FormField
                        control={form.control}
                        name="notes"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Notes</FormLabel>
                            <FormControl>
                              <Input placeholder="Initial context or intent" {...field} />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />

                      {form.formState.errors.root && (
                        <p className="text-sm text-destructive">{form.formState.errors.root.message}</p>
                      )}

                      <div className="flex justify-end">
                        <Button type="submit" loading={createLead.isPending} loadingLabel="Creating lead...">
                          Create Lead
                        </Button>
                      </div>
                    </form>
                  </Form>
                </DialogContent>
              </Dialog>
            )}
          </div>

          <div className="grid gap-6 md:grid-cols-2 xl:grid-cols-3">
            {loadingStats ? (
              Array.from({ length: 3 }).map((_, index) => (
                <Skeleton key={index} className="h-32 rounded-xl" />
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

          <div className="grid gap-6 xl:grid-cols-[1.1fr,0.9fr]">
            <Card>
              <CardHeader>
                <CardTitle>Weekly Trend</CardTitle>
                <CardDescription>New leads captured in the last seven days.</CardDescription>
              </CardHeader>
              <CardContent className="h-[260px]">
                {loadingStats ? (
                  <Skeleton className="h-full w-full rounded-xl" />
                ) : (
                  <ResponsiveContainer width="100%" height="100%">
                    <LineChart data={stats?.weeklyTrend ?? []}>
                      <XAxis dataKey="date" tickLine={false} axisLine={false} fontSize={12} />
                      <YAxis tickLine={false} axisLine={false} fontSize={12} allowDecimals={false} />
                      <Tooltip />
                      <Line
                        type="monotone"
                        dataKey="count"
                        stroke="hsl(var(--primary))"
                        strokeWidth={2}
                        dot={false}
                      />
                    </LineChart>
                  </ResponsiveContainer>
                )}
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>Status Mix</CardTitle>
                <CardDescription>Current lead distribution by pipeline state.</CardDescription>
              </CardHeader>
              <CardContent className="h-[260px]">
                {loadingStats ? (
                  <Skeleton className="h-full w-full rounded-xl" />
                ) : (
                  <ResponsiveContainer width="100%" height="100%">
                    <BarChart data={stats?.byStatus ?? []} layout="vertical">
                      <XAxis type="number" hide />
                      <YAxis
                        dataKey="status"
                        type="category"
                        tickLine={false}
                        axisLine={false}
                        fontSize={12}
                        width={90}
                      />
                      <Tooltip />
                      <Bar dataKey="count" fill="hsl(var(--primary))" radius={[0, 6, 6, 0]} />
                    </BarChart>
                  </ResponsiveContainer>
                )}
              </CardContent>
            </Card>
          </div>

          <Card>
            <CardHeader className="pb-4">
              <div className="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
                <div>
                  <CardTitle>All Leads</CardTitle>
                  <CardDescription>Tenant-wide lead records across all client accounts.</CardDescription>
                </div>
                <div className="flex w-full flex-col gap-2 sm:flex-row lg:w-auto">
                  <div className="relative w-full sm:w-72">
                    <Search className="absolute left-3 top-3 h-4 w-4 text-muted-foreground" />
                    <Input
                      className="pl-9"
                      placeholder="Search leads"
                      value={searchTerm}
                      onChange={(event) => setSearchTerm(event.target.value)}
                    />
                  </div>
                  <div className="relative">
                    <Filter className="pointer-events-none absolute left-3 top-3 h-4 w-4 text-muted-foreground" />
                    <Select value={statusFilter} onValueChange={setStatusFilter}>
                      <SelectTrigger className="w-full pl-9 sm:w-44">
                        <SelectValue placeholder="Filter status" />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="all">All statuses</SelectItem>
                        <SelectItem value="new">New</SelectItem>
                        <SelectItem value="qualified">Qualified</SelectItem>
                        <SelectItem value="converted">Converted</SelectItem>
                        <SelectItem value="contacted">Contacted</SelectItem>
                        <SelectItem value="lost">Lost</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                </div>
              </div>
            </CardHeader>
            <CardContent>
              {loadingLeads ? (
                <div className="space-y-3">
                  {Array.from({ length: 6 }).map((_, index) => (
                    <Skeleton key={index} className="h-12 w-full rounded-xl" />
                  ))}
                </div>
              ) : (
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Lead</TableHead>
                      <TableHead>Client</TableHead>
                      <TableHead>Source</TableHead>
                      <TableHead>Status</TableHead>
                      <TableHead>Score</TableHead>
                      <TableHead className="text-right">Created</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {filteredLeads.length ? (
                      filteredLeads.map((lead) => (
                        <TableRow key={lead.id}>
                          <TableCell>
                            <div className="font-medium">{lead.name}</div>
                            <div className="text-xs text-muted-foreground">{lead.email}</div>
                            {lead.company && (
                              <div className="text-xs text-muted-foreground">{lead.company}</div>
                            )}
                          </TableCell>
                          <TableCell>{lead.tenant_client_name}</TableCell>
                          <TableCell className="capitalize text-muted-foreground">
                            {lead.source.replaceAll("_", " ")}
                          </TableCell>
                          <TableCell>
                            <Badge variant="outline" className={statusTone(lead.status)}>
                              {lead.status}
                            </Badge>
                          </TableCell>
                          <TableCell>{lead.score}</TableCell>
                          <TableCell className="text-right text-sm text-muted-foreground">
                            {new Date(lead.created_at).toLocaleDateString()}
                          </TableCell>
                        </TableRow>
                      ))
                    ) : (
                      <TableRow>
                        <TableCell colSpan={6} className="h-24 text-center text-muted-foreground">
                          No leads matched the current filters.
                        </TableCell>
                      </TableRow>
                    )}
                  </TableBody>
                </Table>
              )}
            </CardContent>
          </Card>
        </div>
      </Layout>
    </ProtectedRoute>
  );
}
