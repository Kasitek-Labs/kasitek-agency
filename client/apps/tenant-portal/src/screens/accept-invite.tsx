"use client";

import { useEffect, useMemo, useState } from "react";
import { useSearchParams, useRouter } from "next/navigation";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { zodResolver } from "@hookform/resolvers/zod";
import { tenantFetch } from "@/lib/api";
import { useAuth } from "@/lib/auth";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Form, FormControl, FormField, FormItem, FormLabel, FormMessage } from "@/components/ui/form";
import { TenantBrandMark } from "@/components/tenant-brand";
import { useTenantContext } from "@/lib/tenant-context-provider";

const acceptInviteSchema = z
  .object({
    password: z.string().min(8, "Password must be at least 8 characters"),
    confirmPassword: z.string().min(8, "Confirm your password"),
    name: z.string().optional(),
  })
  .refine((values) => values.password === values.confirmPassword, {
    path: ["confirmPassword"],
    message: "Passwords do not match",
  });

export default function AcceptInviteScreen() {
  const router = useRouter();
  const { refresh } = useAuth();
  const { brandName, tagline } = useTenantContext();
  const searchParams = useSearchParams();
  const [success, setSuccess] = useState(false);
  const [inviteEmail, setInviteEmail] = useState<string | null>(null);
  const [tenantClientName, setTenantClientName] = useState<string | null>(null);
  const [workspaceDisplayName, setWorkspaceDisplayName] = useState<string | null>(null);
  const [isLoadingInvite, setIsLoadingInvite] = useState(true);
  const [inviteError, setInviteError] = useState<string | null>(null);
  const token = useMemo(() => searchParams.get("token") ?? "", [searchParams]);

  const form = useForm<z.infer<typeof acceptInviteSchema>>({
    resolver: zodResolver(acceptInviteSchema),
    defaultValues: {
      password: "",
      confirmPassword: "",
      name: "",
    },
  });

  useEffect(() => {
    let active = true;

    if (!token) {
      setInviteError("Invite token is missing");
      setIsLoadingInvite(false);
      return;
    }

    void tenantFetch<{
      invite: {
        email: string;
        name?: string | null;
        tenantClientName: string;
        workspaceDisplayName: string;
        isValid: boolean;
      };
    }>(`/api/client-auth/invite?token=${encodeURIComponent(token)}`, {
      method: "GET",
    })
      .then((payload) => {
        if (!active) {
          return;
        }

        if (!payload.invite.isValid) {
          setInviteError("This invite is no longer valid");
          return;
        }

        setInviteEmail(payload.invite.email);
        setTenantClientName(payload.invite.tenantClientName);
        setWorkspaceDisplayName(payload.invite.workspaceDisplayName);
        form.setValue("name", payload.invite.name ?? "");
      })
      .catch((error) => {
        if (active) {
          setInviteError(error instanceof Error ? error.message : "Unable to load invite");
        }
      })
      .finally(() => {
        if (active) {
          setIsLoadingInvite(false);
        }
      });

    return () => {
      active = false;
    };
  }, [form, token]);

  const onSubmit = async (values: z.infer<typeof acceptInviteSchema>) => {
    if (!token) {
      form.setError("root", { message: "Invite token is missing" });
      return;
    }

    try {
      await tenantFetch("/api/client-auth/accept-invite", {
        method: "POST",
        body: JSON.stringify({
          token,
          password: values.password,
          name: values.name,
        }),
      });
      await refresh();
      setSuccess(true);
      setTimeout(() => router.replace("/client"), 900);
    } catch (error) {
      form.setError("root", {
        message: error instanceof Error ? error.message : "Unable to accept invite",
      });
    }
  };

  return (
    <div className="min-h-screen w-full bg-background p-4">
      <div className="mx-auto flex min-h-screen max-w-md items-center">
        <Card className="w-full border-border/60 shadow-xl">
          <CardHeader className="text-center">
            <div className="mx-auto mb-4">
              <TenantBrandMark />
            </div>
            <CardTitle>{brandName}</CardTitle>
            <CardDescription>
              {workspaceDisplayName && tenantClientName
                ? `Set credentials for ${tenantClientName} inside ${workspaceDisplayName}.`
                : `Create credentials for your ${tagline.toLowerCase()}.`}
            </CardDescription>
          </CardHeader>
          <CardContent>
            {isLoadingInvite ? (
              <div className="rounded-xl border border-border px-4 py-5 text-center text-sm text-muted-foreground">
                Loading invite details.
              </div>
            ) : inviteError ? (
              <div className="rounded-xl border border-destructive/20 bg-destructive/5 px-4 py-5 text-center text-sm text-destructive">
                {inviteError}
              </div>
            ) : success ? (
              <div className="rounded-xl border border-primary/20 bg-primary/5 px-4 py-5 text-center text-sm text-muted-foreground">
                Account activated. Redirecting to your client portal.
              </div>
            ) : (
              <Form {...form}>
                <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
                  {inviteEmail && (
                    <div className="rounded-xl border border-border bg-muted/40 px-4 py-3 text-sm">
                      <div className="font-medium">{inviteEmail}</div>
                      <div className="text-muted-foreground">This invited email will be used for sign-in.</div>
                    </div>
                  )}
                  <FormField
                    control={form.control}
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
                    control={form.control}
                    name="password"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel>Password</FormLabel>
                        <FormControl>
                          <Input type="password" placeholder="Create a password" {...field} />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />
                  <FormField
                    control={form.control}
                    name="confirmPassword"
                    render={({ field }) => (
                      <FormItem>
                        <FormLabel>Confirm Password</FormLabel>
                        <FormControl>
                          <Input type="password" placeholder="Confirm your password" {...field} />
                        </FormControl>
                        <FormMessage />
                      </FormItem>
                    )}
                  />
                  {form.formState.errors.root && (
                    <p className="text-sm text-destructive">{form.formState.errors.root.message}</p>
                  )}
                  <Button
                    className="w-full"
                    type="submit"
                    loading={form.formState.isSubmitting}
                    loadingLabel="Saving..."
                  >
                    Activate Account
                  </Button>
                </form>
              </Form>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
