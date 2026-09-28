"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useMemo, useState } from "react";
import { getListTenantClientsQueryKey, useCreateTenantClient, useListTenantClients } from "@workspace/api-client-react";
import { useQueryClient } from "@tanstack/react-query";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { zodResolver } from "@hookform/resolvers/zod";
import { Building2, Plus, Users, ArrowRight } from "lucide-react";
import { ProtectedRoute } from "@/lib/auth";
import { Layout } from "@/components/layout";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Form, FormControl, FormField, FormItem, FormLabel, FormMessage } from "@/components/ui/form";
import { Skeleton } from "@/components/ui/skeleton";
import { Badge } from "@/components/ui/badge";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { CLIENT_ADMIN_ROLES } from "@/lib/tenant-access";

const createClientSchema = z.object({
  name: z.string().min(2, "Client name is required"),
  admin_email: z.string().email("Valid admin email is required"),
  external_ref: z.string().optional(),
});

function formatDate(value: string) {
  return new Date(value).toLocaleDateString();
}

export default function ClientsScreen() {
  const router = useRouter();
  const queryClient = useQueryClient();
  const [isClientDialogOpen, setIsClientDialogOpen] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);
  const { data: clients, isLoading: isLoadingClients } = useListTenantClients();
  const createClient = useCreateTenantClient();

  const createClientForm = useForm<z.infer<typeof createClientSchema>>({
    resolver: zodResolver(createClientSchema),
    defaultValues: {
      name: "",
      admin_email: "",
      external_ref: "",
    },
  });

  const totalUsers = useMemo(
    () => clients?.reduce((sum, client) => sum + (client.user_count ?? 0), 0) ?? 0,
    [clients],
  );

  const onCreateClient = (values: z.infer<typeof createClientSchema>) => {
    setCreateError(null);

    createClient.mutate(
      {
        name: values.name,
        admin_email: values.admin_email,
        external_ref: values.external_ref,
      },
      {
        onSuccess: async (payload) => {
          await queryClient.invalidateQueries({ queryKey: getListTenantClientsQueryKey() });
          createClientForm.reset();
          setIsClientDialogOpen(false);
          const inviteQuery = new URLSearchParams({
            adminInviteEmail: payload.adminInvite.email,
            adminInviteSent: payload.adminInvite.emailSent ? "1" : "0",
          });
          if (!payload.adminInvite.emailSent && payload.adminInvite.activationUrl) {
            inviteQuery.set("adminInviteUrl", payload.adminInvite.activationUrl);
          }
          router.push(`/clients/${payload.client.id}?${inviteQuery.toString()}`);
        },
        onError: (error) => {
          createClientForm.setError("root", {
            message: error instanceof Error ? error.message : "Unable to create client",
          });
          setCreateError(error instanceof Error ? error.message : "Unable to create client");
        },
      },
    );
  };

  return (
    <ProtectedRoute allowedRoles={CLIENT_ADMIN_ROLES}>
      <Layout>
        <div className="space-y-8 p-8 animate-in fade-in duration-500">
          <div className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
            <div>
              <h1 className="text-3xl font-bold tracking-tight">Clients</h1>
              <p className="mt-1 text-muted-foreground">
                Manage the client accounts inside this portal.
              </p>
            </div>
            <div className="flex flex-wrap gap-3">
              <Dialog open={isClientDialogOpen} onOpenChange={setIsClientDialogOpen}>
                <DialogTrigger asChild>
                  <Button>
                    <Plus className="mr-2 h-4 w-4" />
                    Add Client
                  </Button>
                </DialogTrigger>
                <DialogContent>
                  <DialogHeader>
                    <DialogTitle>Create Client Account</DialogTitle>
                    <DialogDescription>
                      Add a new client account to this portal. The first admin invite is sent immediately.
                    </DialogDescription>
                  </DialogHeader>
                  <Form {...createClientForm}>
                    <form onSubmit={createClientForm.handleSubmit(onCreateClient)} className="space-y-4">
                      <FormField
                        control={createClientForm.control}
                        name="name"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Client Name</FormLabel>
                            <FormControl>
                              <Input placeholder="Harbor Dental" {...field} />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />
                      <FormField
                        control={createClientForm.control}
                        name="admin_email"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Client Admin Email</FormLabel>
                            <FormControl>
                              <Input placeholder="admin@harbordental.com" {...field} />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />
                      <FormField
                        control={createClientForm.control}
                        name="external_ref"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>External Ref</FormLabel>
                            <FormControl>
                              <Input placeholder="CRM-123" {...field} />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />
                      {createError || createClientForm.formState.errors.root ? (
                        <p className="text-sm text-destructive">
                          {createError || createClientForm.formState.errors.root?.message}
                        </p>
                      ) : (
                        <p className="text-xs text-muted-foreground">
                          We send the first admin invite to the email above.
                        </p>
                      )}
                      <div className="flex justify-end">
                        <Button type="submit" loading={createClient.isPending} loadingLabel="Creating client...">
                          Create Client
                        </Button>
                      </div>
                    </form>
                  </Form>
                </DialogContent>
              </Dialog>
            </div>
          </div>

          <div className="grid gap-4 md:grid-cols-3">
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm">Clients</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-3xl font-semibold">{clients?.length ?? 0}</div>
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm">Users</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-3xl font-semibold">{totalUsers}</div>
              </CardContent>
            </Card>
            <Card>
              <CardHeader className="pb-2">
                <CardTitle className="text-sm">Status</CardTitle>
              </CardHeader>
              <CardContent>
                <div className="text-3xl font-semibold">{clients?.length ? "Active" : "Empty"}</div>
              </CardContent>
            </Card>
          </div>

          <Card>
            <CardHeader>
              <CardTitle>Client Directory</CardTitle>
              <CardDescription>
                Each row opens a client details page where you can manage its users and invitations.
              </CardDescription>
            </CardHeader>
            <CardContent>
              {isLoadingClients ? (
                <div className="space-y-3">
                  {Array.from({ length: 4 }).map((_, index) => (
                    <Skeleton key={index} className="h-16 w-full rounded-xl" />
                  ))}
                </div>
              ) : clients?.length ? (
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Client</TableHead>
                      <TableHead>Slug</TableHead>
                      <TableHead>Users</TableHead>
                      <TableHead>Status</TableHead>
                      <TableHead>Created</TableHead>
                      <TableHead className="text-right">Open</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {clients.map((client) => (
                      <TableRow key={client.id}>
                        <TableCell>
                          <div className="flex items-center gap-3">
                            <Building2 className="h-4 w-4 text-primary" />
                            <div>
                              <div className="font-medium">{client.name}</div>
                              {client.external_ref ? (
                                <div className="text-xs text-muted-foreground">External ref: {client.external_ref}</div>
                              ) : null}
                            </div>
                          </div>
                        </TableCell>
                        <TableCell className="font-mono text-xs text-muted-foreground">{client.slug}</TableCell>
                        <TableCell>{client.user_count ?? 0}</TableCell>
                        <TableCell>
                          <Badge variant={client.status === "active" ? "default" : "outline"}>{client.status}</Badge>
                        </TableCell>
                        <TableCell className="text-sm text-muted-foreground">
                          {formatDate(client.created_at)}
                        </TableCell>
                        <TableCell className="text-right">
                          <Button asChild variant="ghost" size="sm">
                            <Link href={`/clients/${client.id}`}>
                              Open
                              <ArrowRight className="ml-2 h-4 w-4" />
                            </Link>
                          </Button>
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              ) : (
                <div className="rounded-xl border border-dashed border-border px-6 py-10 text-center text-muted-foreground">
                  No clients yet. Add your first client account to get started.
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      </Layout>
    </ProtectedRoute>
  );
}
