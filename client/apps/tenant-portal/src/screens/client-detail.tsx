"use client";

import Link from "next/link";
import { useMemo, useState } from "react";
import {
  getListTenantClientsQueryKey,
  getListTenantClientUsersQueryKey,
  useInviteTenantClientUser,
  useListTenantClients,
  useListTenantClientUsers,
} from "@workspace/api-client-react";
import { useQueryClient } from "@tanstack/react-query";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { zodResolver } from "@hookform/resolvers/zod";
import { ArrowLeft, Building2, Copy, MailPlus, Users } from "lucide-react";
import { ProtectedRoute } from "@/lib/auth";
import { Layout } from "@/components/layout";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Input } from "@/components/ui/input";
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
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { CLIENT_ADMIN_ROLES } from "@/lib/tenant-access";

const inviteUserSchema = z.object({
  email: z.string().email("Valid email is required"),
  name: z.string().optional(),
  role: z.string().min(1, "Role is required"),
});

function formatDate(value: string) {
  return new Date(value).toLocaleString();
}

export default function ClientDetailScreen({
  clientId,
  initialAdminInviteEmail,
  initialAdminInviteSent,
  initialAdminInviteUrl,
}: {
  clientId: string;
  initialAdminInviteEmail: string | null;
  initialAdminInviteSent: boolean;
  initialAdminInviteUrl: string | null;
}) {
  const queryClient = useQueryClient();
  const [isInviteDialogOpen, setIsInviteDialogOpen] = useState(false);
  const [latestInviteLink, setLatestInviteLink] = useState<string | null>(initialAdminInviteUrl);
  const [latestInviteEmailSent, setLatestInviteEmailSent] = useState(initialAdminInviteSent);
  const [latestInviteEmail, setLatestInviteEmail] = useState<string | null>(initialAdminInviteEmail);

  const { data: clients, isLoading: isLoadingClients } = useListTenantClients();
  const { data: clientUsers, isLoading: isLoadingUsers } = useListTenantClientUsers(clientId, {
    enabled: !!clientId,
  });
  const inviteClientUser = useInviteTenantClientUser();

  const inviteUserForm = useForm<z.infer<typeof inviteUserSchema>>({
    resolver: zodResolver(inviteUserSchema),
    defaultValues: {
      email: "",
      name: "",
      role: "member",
    },
  });

  const selectedClient = useMemo(
    () => clients?.find((client) => client.id === clientId) ?? null,
    [clientId, clients],
  );

  const copyInviteToken = async () => {
    if (!latestInviteLink || typeof navigator === "undefined") {
      return;
    }

    await navigator.clipboard.writeText(latestInviteLink);
  };

  const onInviteUser = (values: z.infer<typeof inviteUserSchema>) => {
    inviteClientUser.mutate(
      {
        tenantClientId: clientId,
        email: values.email,
        name: values.name,
        role: values.role,
      },
      {
        onSuccess: (invite) => {
          void queryClient.invalidateQueries({
            queryKey: getListTenantClientUsersQueryKey(clientId),
          });
          const fallbackLink =
            typeof window !== "undefined"
              ? `${window.location.origin}/activate?token=${invite.token}`
              : null;
          setLatestInviteEmail(invite.email);
          setLatestInviteEmailSent(invite.emailSent);
          setLatestInviteLink(invite.emailSent ? null : invite.activationUrl ?? fallbackLink);
          setIsInviteDialogOpen(false);
          inviteUserForm.reset({ email: "", name: "", role: "member" });
        },
        onError: (error) => {
          inviteUserForm.setError("root", {
            message: error instanceof Error ? error.message : "Unable to invite user",
          });
        },
      },
    );
  };

  return (
    <ProtectedRoute allowedRoles={CLIENT_ADMIN_ROLES}>
      <Layout>
        <div className="space-y-8 p-8 animate-in fade-in duration-500">
          <div className="space-y-2">
            <Link href="/clients" className="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground">
              <ArrowLeft className="h-4 w-4" />
              Back to Clients
            </Link>
            {isLoadingClients ? (
              <Skeleton className="h-10 w-72 rounded-xl" />
            ) : selectedClient ? (
              <div className="space-y-2">
                <div className="flex flex-wrap items-center gap-3">
                  <h1 className="text-3xl font-bold tracking-tight">{selectedClient.name}</h1>
                  <Badge variant={selectedClient.status === "active" ? "default" : "outline"}>
                    {selectedClient.status}
                  </Badge>
                </div>
                <p className="text-muted-foreground">
                  Client account details and users for <span className="font-mono">{selectedClient.slug}</span>.
                </p>
              </div>
            ) : (
              <div className="space-y-2">
                <h1 className="text-3xl font-bold tracking-tight">Client not found</h1>
                <p className="text-muted-foreground">The client account could not be resolved in this portal.</p>
              </div>
            )}
          </div>

          {selectedClient ? (
            <>
              {latestInviteEmail ? (
                <Card className="border-primary/20 bg-primary/5">
                  <CardHeader>
                    <CardTitle className="text-base">
                      {latestInviteEmailSent ? "Admin Invite Sent" : "Admin Invite Ready"}
                    </CardTitle>
                    <CardDescription>
                      {latestInviteEmailSent
                        ? `We sent a verification link to ${latestInviteEmail}. They should open it to finish setting up client access.`
                        : `The verification link for ${latestInviteEmail} is ready in this session.`}
                    </CardDescription>
                  </CardHeader>
                </Card>
              ) : null}

              <div className="grid gap-4 md:grid-cols-4">
                <Card>
                  <CardHeader className="pb-2">
                    <CardTitle className="text-sm">Users</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="text-3xl font-semibold">{clientUsers?.length ?? selectedClient.user_count ?? 0}</div>
                  </CardContent>
                </Card>
                <Card>
                  <CardHeader className="pb-2">
                    <CardTitle className="text-sm">Status</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="text-3xl font-semibold capitalize">{selectedClient.status}</div>
                  </CardContent>
                </Card>
                <Card>
                  <CardHeader className="pb-2">
                    <CardTitle className="text-sm">Slug</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="text-sm font-mono">{selectedClient.slug}</div>
                  </CardContent>
                </Card>
                <Card>
                  <CardHeader className="pb-2">
                    <CardTitle className="text-sm">Created</CardTitle>
                  </CardHeader>
                  <CardContent>
                    <div className="text-sm">{formatDate(selectedClient.created_at)}</div>
                  </CardContent>
                </Card>
              </div>

              {(latestInviteLink || latestInviteEmailSent) && (
                <Card className="border-primary/20 bg-primary/5">
                  <CardHeader>
                    <CardTitle className="text-base">
                      {latestInviteEmailSent ? "Invite Sent" : "Latest Invite Link"}
                    </CardTitle>
                    <CardDescription>
                      {latestInviteEmailSent
                        ? `An activation email was sent to ${latestInviteEmail ?? "the invited user"}.`
                        : "Email delivery is not configured for this environment, so use this activation link directly."}
                    </CardDescription>
                  </CardHeader>
                  {latestInviteLink ? (
                    <CardContent className="flex flex-col gap-3 lg:flex-row lg:items-center">
                      <code className="flex-1 rounded-md border border-border bg-background px-3 py-2 text-sm">
                        {latestInviteLink}
                      </code>
                      <Button variant="secondary" onClick={() => void copyInviteToken()}>
                        <Copy className="mr-2 h-4 w-4" />
                        Copy Link
                      </Button>
                    </CardContent>
                  ) : null}
                </Card>
              )}

              <div className="grid gap-8 xl:grid-cols-[0.95fr,1.05fr]">
                <Card>
                  <CardHeader>
                    <CardTitle>Client Details</CardTitle>
                    <CardDescription>Quick facts for this client account.</CardDescription>
                  </CardHeader>
                  <CardContent className="space-y-4 text-sm">
                    <div className="flex items-start gap-3 rounded-xl border border-border bg-background/60 p-4">
                      <Building2 className="mt-0.5 h-4 w-4 text-primary" />
                      <div>
                        <div className="text-muted-foreground text-xs uppercase tracking-[0.2em]">Name</div>
                        <div className="mt-1 font-medium">{selectedClient.name}</div>
                      </div>
                    </div>
                    <div className="flex items-start gap-3 rounded-xl border border-border bg-background/60 p-4">
                      <Users className="mt-0.5 h-4 w-4 text-primary" />
                      <div>
                        <div className="text-muted-foreground text-xs uppercase tracking-[0.2em]">Users</div>
                        <div className="mt-1 font-medium">{clientUsers?.length ?? selectedClient.user_count ?? 0}</div>
                      </div>
                    </div>
                    {selectedClient.external_ref ? (
                      <div className="flex items-start gap-3 rounded-xl border border-border bg-background/60 p-4">
                        <MailPlus className="mt-0.5 h-4 w-4 text-primary" />
                        <div>
                          <div className="text-muted-foreground text-xs uppercase tracking-[0.2em]">External Ref</div>
                          <div className="mt-1 font-mono text-sm">{selectedClient.external_ref}</div>
                        </div>
                      </div>
                    ) : null}
                    <div className="flex items-start gap-3 rounded-xl border border-border bg-background/60 p-4">
                      <MailPlus className="mt-0.5 h-4 w-4 text-primary" />
                      <div>
                        <div className="text-muted-foreground text-xs uppercase tracking-[0.2em]">Slug</div>
                        <div className="mt-1 font-mono text-sm">{selectedClient.slug}</div>
                      </div>
                    </div>
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader className="flex flex-row items-center justify-between gap-4">
                    <div>
                      <CardTitle>Users</CardTitle>
                      <CardDescription>
                        Accounts invited into {selectedClient.name}.
                      </CardDescription>
                    </div>
                    <Dialog open={isInviteDialogOpen} onOpenChange={setIsInviteDialogOpen}>
                      <DialogTrigger asChild>
                        <Button>
                          <MailPlus className="mr-2 h-4 w-4" />
                          Invite User
                        </Button>
                      </DialogTrigger>
                      <DialogContent>
                        <DialogHeader>
                          <DialogTitle>Invite Client User</DialogTitle>
                          <DialogDescription>
                            Invite a new user into {selectedClient.name}.
                          </DialogDescription>
                        </DialogHeader>
                        <Form {...inviteUserForm}>
                          <form onSubmit={inviteUserForm.handleSubmit(onInviteUser)} className="space-y-4">
                            <FormField
                              control={inviteUserForm.control}
                              name="email"
                              render={({ field }) => (
                                <FormItem>
                                  <FormLabel>Email</FormLabel>
                                  <FormControl>
                                    <Input placeholder="owner@client.com" {...field} />
                                  </FormControl>
                                  <FormMessage />
                                </FormItem>
                              )}
                            />
                            <FormField
                              control={inviteUserForm.control}
                              name="name"
                              render={({ field }) => (
                                <FormItem>
                                  <FormLabel>Name</FormLabel>
                                  <FormControl>
                                    <Input placeholder="Jane Doe" {...field} />
                                  </FormControl>
                                  <FormMessage />
                                </FormItem>
                              )}
                            />
                            <FormField
                              control={inviteUserForm.control}
                              name="role"
                              render={({ field }) => (
                                <FormItem>
                                  <FormLabel>Role</FormLabel>
                                  <Select onValueChange={field.onChange} defaultValue={field.value}>
                                    <FormControl>
                                      <SelectTrigger>
                                        <SelectValue placeholder="Select role" />
                                      </SelectTrigger>
                                    </FormControl>
                                    <SelectContent>
                                      <SelectItem value="owner">Owner</SelectItem>
                                      <SelectItem value="member">Member</SelectItem>
                                    </SelectContent>
                                  </Select>
                                  <FormMessage />
                                </FormItem>
                              )}
                            />
                            {inviteUserForm.formState.errors.root ? (
                              <p className="text-sm text-destructive">{inviteUserForm.formState.errors.root.message}</p>
                            ) : null}
                            <div className="flex justify-end">
                              <Button type="submit" loading={inviteClientUser.isPending} loadingLabel="Sending invite...">
                                Send Invite
                              </Button>
                            </div>
                          </form>
                        </Form>
                      </DialogContent>
                    </Dialog>
                  </CardHeader>
                  <CardContent>
                    {isLoadingUsers ? (
                      <div className="space-y-3">
                        {Array.from({ length: 4 }).map((_, index) => (
                          <Skeleton key={index} className="h-12 w-full" />
                        ))}
                      </div>
                    ) : (
                      <Table>
                        <TableHeader>
                          <TableRow>
                            <TableHead>User</TableHead>
                            <TableHead>Status</TableHead>
                            <TableHead className="text-right">Created</TableHead>
                          </TableRow>
                        </TableHeader>
                        <TableBody>
                          {clientUsers?.length ? (
                            clientUsers.map((user) => (
                              <TableRow key={user.id}>
                                <TableCell>
                                  <div>
                                    <div className="font-medium">{user.name || "Unnamed user"}</div>
                                    <div className="text-xs text-muted-foreground">{user.email}</div>
                                  </div>
                                </TableCell>
                                <TableCell>
                                  <Badge variant="outline">{user.status}</Badge>
                                </TableCell>
                                <TableCell className="text-right text-sm text-muted-foreground">
                                  {new Date(user.created_at).toLocaleDateString()}
                                </TableCell>
                              </TableRow>
                            ))
                          ) : (
                            <TableRow>
                              <TableCell colSpan={3} className="h-24 text-center text-muted-foreground">
                                No users invited yet.
                              </TableCell>
                            </TableRow>
                          )}
                        </TableBody>
                      </Table>
                    )}
                  </CardContent>
                </Card>
              </div>
            </>
          ) : null}
        </div>
      </Layout>
    </ProtectedRoute>
  );
}
