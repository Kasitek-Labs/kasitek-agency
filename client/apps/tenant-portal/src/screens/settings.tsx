"use client";

import { useEffect, useMemo, useState } from "react";
import {
  CheckCircle2,
  Globe,
  Loader2,
  Mail,
  RefreshCw,
  Send,
  UserPlus,
  ShieldCheck,
  Trash2,
  Users,
} from "lucide-react";
import { ProtectedRoute } from "@/lib/auth";
import { useAuth } from "@/lib/auth";
import { Layout } from "@/components/layout";
import { tenantFetch } from "@/lib/api";
import { SETTINGS_ROLES } from "@/lib/tenant-access";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";

type DomainRecord = {
  id: string;
  domain: string;
  isPrimary: boolean;
  dnsStatus: string;
  verifiedAt: string | null;
  verificationMessage: string;
};

type WorkspaceDomainsResponse = {
  hostedDomain: string | null;
  domains: DomainRecord[];
  verification?: {
    expectedTargetHost?: string | null;
    hostedDomainBase?: string | null;
  };
};

type WorkspaceResponse = {
  workspace: {
    slug: string;
    hostedDomain?: string | null;
    domainVerification?: {
      expectedTargetHost?: string | null;
      hostedDomainBase?: string | null;
    };
  };
};

type EmailDnsRecord = {
  id: string;
  recordGroup: string;
  recordType: string;
  recordName: string;
  recordValue: string;
  recordStatus: string;
  ttl: string | null;
  priority: number | null;
};

type EmailDomain = {
  id: string;
  provider: string;
  providerDomainId: string;
  domain: string;
  status: string;
  verifiedAt: string | null;
  createdAt: string;
  updatedAt: string;
  records: EmailDnsRecord[];
};

type EmailSenderPreview = {
  sendingMode: string;
  senderName: string;
  fromEmail: string;
  replyToEmail: string | null;
  domainStatus: string;
  activeDomainId: string | null;
  activeDomain: string | null;
};

type EmailSettings = {
  workspaceId: string;
  sendingMode: string;
  senderName: string | null;
  replyToEmail: string | null;
  sharedSenderLocalPart: string;
  customDomainId: string | null;
};

type EmailSettingsResponse = {
  email: {
    resendConfigured: boolean;
    sharedDomain: string;
    platformFromEmail: string;
    sender: EmailSenderPreview;
    settings: EmailSettings;
    customDomains: EmailDomain[];
  };
};

type StaffMember = {
  id: string;
  email: string;
  name: string | null;
  status: string;
  roles: string[];
  createdAt: string;
  lastLoginAt: string | null;
};

type StaffResponse = {
  staff: StaffMember[];
};

type StaffInviteResponse = {
  staff: StaffMember;
  inviteSent: boolean;
};

function statusTone(status: string) {
  switch (status) {
    case "verified":
      return "bg-green-500/15 text-green-400";
    case "misconfigured":
      return "bg-amber-500/15 text-amber-400";
    default:
      return "bg-white/5 text-muted-foreground";
  }
}

function emailStatusTone(status: string) {
  switch (status) {
    case "verified":
      return "bg-green-500/15 text-green-400";
    case "pending":
    case "not_started":
      return "bg-amber-500/15 text-amber-400";
    case "temporary_failure":
    case "failed":
      return "bg-red-500/15 text-red-400";
    default:
      return "bg-white/5 text-muted-foreground";
  }
}

export default function SettingsScreen() {
  const { user } = useAuth();
  const [hostedDomain, setHostedDomain] = useState<string | null>(null);
  const [expectedTargetHost, setExpectedTargetHost] = useState<string | null>(null);
  const [workspaceSlug, setWorkspaceSlug] = useState<string>("");
  const [domains, setDomains] = useState<DomainRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [submitting, setSubmitting] = useState(false);
  const [verifyingId, setVerifyingId] = useState<string | null>(null);
  const [removingId, setRemovingId] = useState<string | null>(null);
  const [primaryId, setPrimaryId] = useState<string | null>(null);
  const [error, setError] = useState("");
  const [newDomain, setNewDomain] = useState("");
  const [makePrimary, setMakePrimary] = useState(false);

  const [emailLoading, setEmailLoading] = useState(true);
  const [emailError, setEmailError] = useState("");
  const [emailSaving, setEmailSaving] = useState(false);
  const [emailTesting, setEmailTesting] = useState(false);
  const [creatingEmailDomain, setCreatingEmailDomain] = useState(false);
  const [verifyingEmailDomainId, setVerifyingEmailDomainId] = useState<string | null>(null);
  const [removingEmailDomainId, setRemovingEmailDomainId] = useState<string | null>(null);
  const [resendConfigured, setResendConfigured] = useState(false);
  const [sharedEmailDomain, setSharedEmailDomain] = useState("send.kasitek.co.za");
  const [platformFromEmail, setPlatformFromEmail] = useState("");
  const [emailSender, setEmailSender] = useState<EmailSenderPreview | null>(null);
  const [emailDomains, setEmailDomains] = useState<EmailDomain[]>([]);
  const [staffMembers, setStaffMembers] = useState<StaffMember[]>([]);
  const [staffLoading, setStaffLoading] = useState(true);
  const [staffSaving, setStaffSaving] = useState(false);
  const [staffRemovingId, setStaffRemovingId] = useState<string | null>(null);
  const [staffError, setStaffError] = useState("");
  const [newStaffEmail, setNewStaffEmail] = useState("");
  const [newStaffName, setNewStaffName] = useState("");
  const [newStaffRole, setNewStaffRole] = useState("admin");
  const [sendingMode, setSendingMode] = useState<"shared" | "custom">("shared");
  const [senderName, setSenderName] = useState("");
  const [replyToEmail, setReplyToEmail] = useState("");
  const [sharedSenderLocalPart, setSharedSenderLocalPart] = useState("no-reply");
  const [selectedCustomDomainId, setSelectedCustomDomainId] = useState<string>("");
  const [newEmailDomain, setNewEmailDomain] = useState("");
  const [testRecipient, setTestRecipient] = useState("");

  const primaryCustomDomain = useMemo(
    () => domains.find((entry) => entry.isPrimary) ?? null,
    [domains],
  );
  const verifiedEmailDomains = useMemo(
    () => emailDomains.filter((entry) => entry.status === "verified"),
    [emailDomains],
  );
  const activeEmailDomain = useMemo(
    () => emailDomains.find((entry) => entry.id === selectedCustomDomainId) ?? null,
    [emailDomains, selectedCustomDomainId],
  );
  const currentUserId = user?.id ?? null;

  function applyEmailPayload(payload: EmailSettingsResponse["email"]) {
    setResendConfigured(payload.resendConfigured);
    setSharedEmailDomain(payload.sharedDomain);
    setPlatformFromEmail(payload.platformFromEmail);
    setEmailSender(payload.sender);
    setEmailDomains(payload.customDomains ?? []);
    setSendingMode(payload.settings.sendingMode === "custom" ? "custom" : "shared");
    setSenderName(payload.settings.senderName ?? payload.sender.senderName ?? "");
    setReplyToEmail(payload.settings.replyToEmail ?? "");
    setSharedSenderLocalPart(payload.settings.sharedSenderLocalPart ?? "no-reply");
    setSelectedCustomDomainId(payload.settings.customDomainId ?? "");
  }

  async function loadSettings() {
    setLoading(true);
    setEmailLoading(true);
    setStaffLoading(true);
    setError("");
    setEmailError("");
    setStaffError("");

    try {
      const [workspace, domainPayload, emailPayload, staffPayload] = await Promise.all([
        tenantFetch<WorkspaceResponse>("/api/workspace"),
        tenantFetch<WorkspaceDomainsResponse>("/api/workspace/domains"),
        tenantFetch<EmailSettingsResponse>("/api/workspace/email-sending"),
        tenantFetch<StaffResponse>("/api/workspace/staff"),
      ]);

      setWorkspaceSlug(workspace.workspace.slug);
      setHostedDomain(domainPayload.hostedDomain ?? workspace.workspace.hostedDomain ?? null);
      setExpectedTargetHost(
        domainPayload.verification?.expectedTargetHost ??
          workspace.workspace.domainVerification?.expectedTargetHost ??
          null,
      );
      setDomains(domainPayload.domains ?? []);
      applyEmailPayload(emailPayload.email);
      setStaffMembers(staffPayload.staff ?? []);
    } catch (err) {
      const message = err instanceof Error ? err.message : "Unable to load workspace settings";
      setError(message);
      setEmailError(message);
      setStaffError(message);
    } finally {
      setLoading(false);
      setEmailLoading(false);
      setStaffLoading(false);
    }
  }

  useEffect(() => {
    void loadSettings();
  }, []);

  async function addDomain(e: React.FormEvent) {
    e.preventDefault();
    setSubmitting(true);
    setError("");
    try {
      await tenantFetch("/api/workspace/domains", {
        method: "POST",
        body: JSON.stringify({ domain: newDomain, makePrimary }),
      });
      setNewDomain("");
      setMakePrimary(false);
      await loadSettings();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Unable to add domain");
    } finally {
      setSubmitting(false);
    }
  }

  async function verifyDomain(domainId: string) {
    setVerifyingId(domainId);
    setError("");
    try {
      await tenantFetch(`/api/workspace/domains/${domainId}/verify`, { method: "POST" });
      await loadSettings();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Unable to verify domain");
    } finally {
      setVerifyingId(null);
    }
  }

  async function makeDomainPrimary(domainId: string) {
    setPrimaryId(domainId);
    setError("");
    try {
      await tenantFetch(`/api/workspace/domains/${domainId}/primary`, { method: "POST" });
      await loadSettings();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Unable to set primary domain");
    } finally {
      setPrimaryId(null);
    }
  }

  async function removeDomain(domainId: string) {
    setRemovingId(domainId);
    setError("");
    try {
      await tenantFetch(`/api/workspace/domains/${domainId}`, { method: "DELETE" });
      await loadSettings();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Unable to remove domain");
    } finally {
      setRemovingId(null);
    }
  }

  async function inviteStaffMember(e: React.FormEvent) {
    e.preventDefault();
    setStaffSaving(true);
    setStaffError("");

    try {
      const response = await tenantFetch<StaffInviteResponse>("/api/workspace/staff", {
        method: "POST",
        body: JSON.stringify({
          email: newStaffEmail,
          name: newStaffName || null,
          role: newStaffRole,
        }),
      });

      setNewStaffEmail("");
      setNewStaffName("");
      setNewStaffRole("admin");
      setStaffMembers((current) => {
        const next = current.filter((member) => member.id !== response.staff.id);
        return [response.staff, ...next];
      });
    } catch (err) {
      setStaffError(err instanceof Error ? err.message : "Unable to invite staff member");
    } finally {
      setStaffSaving(false);
    }
  }

  async function removeStaffMember(staffId: string) {
    setStaffRemovingId(staffId);
    setStaffError("");

    try {
      await tenantFetch(`/api/workspace/staff/${staffId}`, { method: "DELETE" });
      setStaffMembers((current) => current.filter((member) => member.id !== staffId));
    } catch (err) {
      setStaffError(err instanceof Error ? err.message : "Unable to remove staff member");
    } finally {
      setStaffRemovingId(null);
    }
  }

  async function saveEmailSettings(e: React.FormEvent) {
    e.preventDefault();
    setEmailSaving(true);
    setEmailError("");

    try {
      const response = await tenantFetch<EmailSettingsResponse>("/api/workspace/email-sending", {
        method: "PUT",
        body: JSON.stringify({
          sendingMode,
          senderName,
          replyToEmail: replyToEmail || null,
          sharedSenderLocalPart,
          customDomainId: selectedCustomDomainId || null,
        }),
      });

      applyEmailPayload(response.email);
    } catch (err) {
      setEmailError(err instanceof Error ? err.message : "Unable to save email settings");
    } finally {
      setEmailSaving(false);
    }
  }

  async function addEmailDomain(e: React.FormEvent) {
    e.preventDefault();
    setCreatingEmailDomain(true);
    setEmailError("");

    try {
      const response = await tenantFetch<EmailSettingsResponse>("/api/workspace/email-sending/domains", {
        method: "POST",
        body: JSON.stringify({ domain: newEmailDomain }),
      });
      setNewEmailDomain("");
      applyEmailPayload(response.email);
    } catch (err) {
      setEmailError(err instanceof Error ? err.message : "Unable to add email domain");
    } finally {
      setCreatingEmailDomain(false);
    }
  }

  async function verifyEmailDomain(domainId: string) {
    setVerifyingEmailDomainId(domainId);
    setEmailError("");

    try {
      const response = await tenantFetch<EmailSettingsResponse>(
        `/api/workspace/email-sending/domains/${domainId}/verify`,
        { method: "POST" },
      );
      applyEmailPayload(response.email);
    } catch (err) {
      setEmailError(err instanceof Error ? err.message : "Unable to verify email domain");
    } finally {
      setVerifyingEmailDomainId(null);
    }
  }

  async function deleteEmailDomain(domainId: string) {
    setRemovingEmailDomainId(domainId);
    setEmailError("");

    try {
      const response = await tenantFetch<EmailSettingsResponse>(
        `/api/workspace/email-sending/domains/${domainId}`,
        { method: "DELETE" },
      );
      applyEmailPayload(response.email);
    } catch (err) {
      setEmailError(err instanceof Error ? err.message : "Unable to remove email domain");
    } finally {
      setRemovingEmailDomainId(null);
    }
  }

  async function sendTestEmail(e: React.FormEvent) {
    e.preventDefault();
    setEmailTesting(true);
    setEmailError("");

    try {
      await tenantFetch("/api/workspace/email-sending/test", {
        method: "POST",
        body: JSON.stringify({ to: testRecipient }),
      });
    } catch (err) {
      setEmailError(err instanceof Error ? err.message : "Unable to send test email");
    } finally {
      setEmailTesting(false);
    }
  }

  return (
    <ProtectedRoute allowedRoles={SETTINGS_ROLES}>
      <Layout>
        <div className="space-y-8 p-8 animate-in fade-in duration-500">
          <div className="flex flex-col gap-2 lg:flex-row lg:items-end lg:justify-between">
            <div>
              <h1 className="text-3xl font-bold tracking-tight">Workspace Settings</h1>
              <p className="text-muted-foreground">
                Manage your hosted portal domain, branded sending setup, and DNS verification.
              </p>
            </div>
            <Button variant="outline" onClick={() => void loadSettings()} disabled={loading || emailLoading}>
              <RefreshCw className={`mr-2 h-4 w-4 ${loading || emailLoading ? "animate-spin" : ""}`} />
              Refresh
            </Button>
          </div>

          {error ? (
            <div className="rounded-xl border border-destructive/20 bg-destructive/10 px-4 py-3 text-sm text-destructive">
              {error}
            </div>
          ) : null}

          <div className="grid gap-6 lg:grid-cols-[1.15fr_0.85fr]">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <ShieldCheck className="h-5 w-5" />
                  Hosted Domain
                </CardTitle>
                <CardDescription>
                  This is your system-managed fallback portal address. It works immediately and should remain available
                  even after you connect a branded domain.
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-3">
                <p className="font-mono text-sm text-foreground">
                  {loading ? "Loading..." : hostedDomain ?? `${workspaceSlug || "workspace"}.portal.kasitek.co.za`}
                </p>
                <p className="text-sm text-muted-foreground">
                  Share this URL internally while your custom domain is being configured.
                </p>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Globe className="h-5 w-5" />
                  DNS Target
                </CardTitle>
                <CardDescription>
                  Point your branded domain at the shared KasiTek portal host, then verify it here.
                </CardDescription>
              </CardHeader>
              <CardContent className="space-y-3 text-sm">
                <p className="text-muted-foreground">Recommended record:</p>
                <p className="font-mono text-foreground">
                  {primaryCustomDomain?.domain ?? "your branded domain"} CNAME {expectedTargetHost ?? "portal.kasitek.co.za"}
                </p>
                <p className="text-muted-foreground">
                  Once DNS propagates, hit Verify. A verified custom domain can then be made primary.
                </p>
              </CardContent>
            </Card>
          </div>

          <Card>
            <CardHeader>
              <CardTitle>Custom Domain</CardTitle>
              <CardDescription>
                Add your branded domain when you’re ready. KasiTek will keep the hosted fallback domain active in case
                DNS or SSL need troubleshooting.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              <form onSubmit={addDomain} className="grid gap-4 lg:grid-cols-[1fr_auto] lg:items-end">
                <div className="space-y-2">
                  <Label htmlFor="custom-domain">Custom domain</Label>
                  <Input
                    id="custom-domain"
                    value={newDomain}
                    onChange={(event) => setNewDomain(event.target.value)}
                    placeholder="your branded domain"
                    required
                  />
                  <div className="flex items-center gap-2 text-sm text-muted-foreground">
                    <Checkbox
                      id="make-primary"
                      checked={makePrimary}
                      onCheckedChange={(checked) => setMakePrimary(Boolean(checked))}
                    />
                    <Label htmlFor="make-primary" className="cursor-pointer font-normal">
                      Make this primary after it is added
                    </Label>
                  </div>
                </div>
                <Button type="submit" loading={submitting} loadingLabel="Adding domain...">
                  Add Domain
                </Button>
              </form>

              <div className="space-y-4">
                {loading ? (
                  <p className="text-sm text-muted-foreground">Loading domains...</p>
                ) : domains.length === 0 ? (
                  <p className="text-sm text-muted-foreground">
                    No custom domains configured yet. Your hosted domain is already active.
                  </p>
                ) : (
                  domains.map((entry) => (
                    <div key={entry.id} className="rounded-xl border border-border bg-background/60 p-4">
                      <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
                        <div className="space-y-2">
                          <div className="flex flex-wrap items-center gap-2">
                            <span className="font-mono text-sm text-foreground">{entry.domain}</span>
                            {entry.isPrimary ? (
                              <span className="rounded-full bg-primary/10 px-2 py-0.5 text-xs font-medium text-primary">
                                Primary
                              </span>
                            ) : null}
                            <span className={`rounded-full px-2 py-0.5 text-xs font-medium capitalize ${statusTone(entry.dnsStatus)}`}>
                              {entry.dnsStatus}
                            </span>
                            {entry.dnsStatus === "verified" ? <CheckCircle2 className="h-4 w-4 text-green-400" /> : null}
                          </div>
                          <p className="text-sm text-muted-foreground">{entry.verificationMessage}</p>
                          <p className="text-xs text-muted-foreground">
                            {entry.verifiedAt ? `Verified ${new Date(entry.verifiedAt).toLocaleString()}` : "Not verified yet"}
                          </p>
                        </div>
                        <div className="flex flex-wrap gap-2">
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => void verifyDomain(entry.id)}
                            disabled={verifyingId === entry.id}
                          >
                            {verifyingId === entry.id ? (
                              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                            ) : (
                              <RefreshCw className="mr-2 h-4 w-4" />
                            )}
                            Verify
                          </Button>
                          {!entry.isPrimary ? (
                            <Button
                              variant="outline"
                              size="sm"
                              onClick={() => void makeDomainPrimary(entry.id)}
                              disabled={primaryId === entry.id}
                            >
                              {primaryId === entry.id ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : null}
                              Make Primary
                            </Button>
                          ) : null}
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => void removeDomain(entry.id)}
                            disabled={removingId === entry.id}
                            className="text-muted-foreground hover:text-destructive"
                          >
                            {removingId === entry.id ? (
                              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                            ) : (
                              <Trash2 className="mr-2 h-4 w-4" />
                            )}
                            Remove
                          </Button>
                        </div>
                      </div>
                    </div>
                  ))
                )}
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Users className="h-5 w-5" />
                Staff Members
              </CardTitle>
              <CardDescription>
                Invite internal team members to manage the workspace with their own login and role.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              {staffError ? (
                <div className="rounded-xl border border-destructive/20 bg-destructive/10 px-4 py-3 text-sm text-destructive">
                  {staffError}
                </div>
              ) : null}

              <form onSubmit={inviteStaffMember} className="grid gap-4 xl:grid-cols-[1fr_1fr_180px_auto] xl:items-end">
                <div className="space-y-2">
                    <Label htmlFor="staff-email">Email</Label>
                    <Input
                      id="staff-email"
                      value={newStaffEmail}
                      onChange={(event) => setNewStaffEmail(event.target.value)}
                      placeholder="admin@yourdomain.co.za"
                      required
                    />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="staff-name">Name</Label>
                  <Input
                    id="staff-name"
                    value={newStaffName}
                    onChange={(event) => setNewStaffName(event.target.value)}
                    placeholder="Alex"
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="staff-role">Role</Label>
                  <Select value={newStaffRole} onValueChange={setNewStaffRole}>
                    <SelectTrigger id="staff-role">
                      <SelectValue placeholder="Select role" />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="admin">Admin</SelectItem>
                      <SelectItem value="owner">Owner</SelectItem>
                      <SelectItem value="account_manager">Account Manager</SelectItem>
                      <SelectItem value="support">Support</SelectItem>
                      <SelectItem value="viewer">Viewer</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
                <Button type="submit" loading={staffSaving} loadingLabel="Inviting staff...">
                  <UserPlus className="mr-2 h-4 w-4" />
                  Invite Staff
                </Button>
              </form>

              <div className="space-y-3">
                {staffLoading ? (
                  <p className="text-sm text-muted-foreground">Loading staff members...</p>
                ) : staffMembers.length === 0 ? (
                  <p className="text-sm text-muted-foreground">
                    No staff members yet. Invite the first internal user to manage the workspace.
                  </p>
                ) : (
                  staffMembers.map((member) => (
                    <div key={member.id} className="rounded-xl border border-border bg-background/60 p-4">
                      <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
                        <div className="space-y-2">
                          <div className="flex flex-wrap items-center gap-2">
                            <span className="font-medium text-foreground">{member.name || member.email}</span>
                            <span className="text-sm text-muted-foreground">{member.email}</span>
                            <span
                              className={`rounded-full px-2 py-0.5 text-xs font-medium capitalize ${
                                member.status === "active"
                                  ? "bg-green-500/15 text-green-400"
                                  : "bg-amber-500/15 text-amber-400"
                              }`}
                            >
                              {member.status}
                            </span>
                          </div>
                          <div className="flex flex-wrap gap-2">
                            {member.roles.map((role) => (
                              <span
                                key={role}
                                className="rounded-full bg-primary/10 px-2 py-0.5 text-xs font-medium text-primary capitalize"
                              >
                                {role}
                              </span>
                            ))}
                          </div>
                          <p className="text-xs text-muted-foreground">
                            Added {new Date(member.createdAt).toLocaleString()}
                            {member.lastLoginAt ? ` · Last login ${new Date(member.lastLoginAt).toLocaleString()}` : ""}
                          </p>
                        </div>

                        <div className="flex flex-wrap gap-2">
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => void removeStaffMember(member.id)}
                            disabled={staffRemovingId === member.id || member.id === currentUserId}
                            className="text-muted-foreground hover:text-destructive"
                          >
                            {staffRemovingId === member.id ? (
                              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                            ) : (
                              <Trash2 className="mr-2 h-4 w-4" />
                            )}
                            Remove
                          </Button>
                        </div>
                      </div>
                    </div>
                  ))
                )}
              </div>
            </CardContent>
          </Card>

          <div className="space-y-6">
            <div>
              <h2 className="text-2xl font-semibold tracking-tight">Email Sending</h2>
              <p className="text-sm text-muted-foreground">
                Start on KasiTek-managed sending, then switch to your own verified domain when ready.
              </p>
            </div>

            {emailError ? (
              <div className="rounded-xl border border-destructive/20 bg-destructive/10 px-4 py-3 text-sm text-destructive">
                {emailError}
              </div>
            ) : null}

            {!resendConfigured ? (
              <div className="rounded-xl border border-amber-500/20 bg-amber-500/10 px-4 py-3 text-sm text-amber-300">
                Resend is not configured on the backend yet, so email sending cannot be activated from this workspace.
              </div>
            ) : null}

            <div className="grid gap-6 xl:grid-cols-[1.1fr_0.9fr]">
              <Card>
                <CardHeader>
                  <CardTitle className="flex items-center gap-2">
                    <Mail className="h-5 w-5" />
                    Sender Configuration
                  </CardTitle>
                  <CardDescription>
                    Configure the branded sender tenants use from the portal before they connect a custom domain.
                  </CardDescription>
                </CardHeader>
                <CardContent className="space-y-6">
                  <div className="grid gap-4 md:grid-cols-2">
                    <div className="rounded-xl border border-border bg-background/60 p-4">
                      <div className="text-xs uppercase tracking-[0.2em] text-muted-foreground">Shared sending</div>
                      <div className="mt-2 font-mono text-sm text-foreground">{sharedEmailDomain}</div>
                      <div className="mt-2 text-sm text-muted-foreground">
                        Shared fallback remains on KasiTek infrastructure until a verified custom domain is selected.
                      </div>
                    </div>
                    <div className="rounded-xl border border-border bg-background/60 p-4">
                      <div className="text-xs uppercase tracking-[0.2em] text-muted-foreground">Current sender</div>
                      <div className="mt-2 text-sm font-medium text-foreground">
                        {emailSender ? `${emailSender.senderName} <${emailSender.fromEmail}>` : "Loading..."}
                      </div>
                      <div className="mt-2 text-sm text-muted-foreground">
                        {emailSender?.replyToEmail ? `Reply-to ${emailSender.replyToEmail}` : `Platform sender ${platformFromEmail}`}
                      </div>
                    </div>
                  </div>

                  <form onSubmit={saveEmailSettings} className="space-y-5">
                    <div className="grid gap-4 md:grid-cols-2">
                      <div className="space-y-2">
                        <Label htmlFor="sender-name">Sender name</Label>
                        <Input
                          id="sender-name"
                          value={senderName}
                          onChange={(event) => setSenderName(event.target.value)}
                          placeholder="Your Brand"
                        />
                      </div>
                      <div className="space-y-2">
                        <Label htmlFor="reply-to-email">Reply-to email</Label>
                        <Input
                          id="reply-to-email"
                          value={replyToEmail}
                          onChange={(event) => setReplyToEmail(event.target.value)}
                          placeholder="hello@yourdomain.co.za"
                        />
                      </div>
                    </div>

                    <div className="grid gap-4 md:grid-cols-2">
                      <div className="space-y-2">
                        <Label htmlFor="shared-local-part">Shared sender local part</Label>
                        <Input
                          id="shared-local-part"
                          value={sharedSenderLocalPart}
                          onChange={(event) => setSharedSenderLocalPart(event.target.value)}
                          placeholder="no-reply"
                        />
                        <p className="text-xs text-muted-foreground">
                          Shared preview: {workspaceSlug || "workspace"}-{sharedSenderLocalPart || "no-reply"}@{sharedEmailDomain}
                        </p>
                      </div>
                      <div className="space-y-2">
                        <Label>Sending mode</Label>
                        <div className="grid grid-cols-2 gap-2">
                          <Button
                            type="button"
                            variant={sendingMode === "shared" ? "default" : "outline"}
                            onClick={() => setSendingMode("shared")}
                          >
                            Use KasiTek sending
                          </Button>
                          <Button
                            type="button"
                            variant={sendingMode === "custom" ? "default" : "outline"}
                            onClick={() => setSendingMode("custom")}
                            disabled={verifiedEmailDomains.length === 0}
                          >
                            Use your domain
                          </Button>
                        </div>
                      </div>
                    </div>

                    <div className="space-y-3">
                      <Label>Verified custom domains</Label>
                      {verifiedEmailDomains.length === 0 ? (
                        <div className="rounded-xl border border-dashed border-border px-4 py-3 text-sm text-muted-foreground">
                          No verified custom email domains yet. Start on KasiTek sending, then connect your own domain below.
                        </div>
                      ) : (
                        <div className="grid gap-3">
                          {verifiedEmailDomains.map((domain) => (
                            <button
                              key={domain.id}
                              type="button"
                              onClick={() => setSelectedCustomDomainId(domain.id)}
                              className={`rounded-xl border px-4 py-3 text-left transition-colors ${
                                selectedCustomDomainId === domain.id
                                  ? "border-primary bg-primary/10"
                                  : "border-border bg-background/60 hover:border-primary/40"
                              }`}
                            >
                              <div className="flex items-center justify-between gap-4">
                                <div>
                                  <div className="font-mono text-sm text-foreground">{domain.domain}</div>
                                  <div className="text-xs text-muted-foreground">
                                    Verified {domain.verifiedAt ? new Date(domain.verifiedAt).toLocaleString() : "recently"}
                                  </div>
                                </div>
                                <span className={`rounded-full px-2 py-0.5 text-xs font-medium capitalize ${emailStatusTone(domain.status)}`}>
                                  {domain.status}
                                </span>
                              </div>
                            </button>
                          ))}
                        </div>
                      )}
                    </div>

                    <div className="flex flex-wrap gap-3">
                      <Button
                        type="submit"
                        loading={emailSaving}
                        loadingLabel="Saving email settings..."
                        disabled={!resendConfigured}
                      >
                        Save Email Settings
                      </Button>
                      {sendingMode === "custom" && !activeEmailDomain ? (
                        <p className="text-sm text-amber-300">Select a verified domain before saving custom sending.</p>
                      ) : null}
                    </div>
                  </form>
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle className="flex items-center gap-2">
                    <Send className="h-5 w-5" />
                    Test Delivery
                  </CardTitle>
                  <CardDescription>
                    Send a live email using the active sender configuration before rolling it into client flows.
                  </CardDescription>
                </CardHeader>
                <CardContent className="space-y-6">
                  <div className="rounded-xl border border-border bg-background/60 p-4 text-sm">
                    <div className="text-xs uppercase tracking-[0.2em] text-muted-foreground">Live preview</div>
                    <div className="mt-2 font-medium text-foreground">
                      {emailSender ? `${emailSender.senderName} <${emailSender.fromEmail}>` : "Loading..."}
                    </div>
                    <div className="mt-2 text-muted-foreground">
                      {emailSender?.sendingMode === "custom"
                        ? `Custom domain ${emailSender.activeDomain ?? ""}`
                        : `Shared KasiTek domain ${sharedEmailDomain}`}
                    </div>
                    {emailSender?.replyToEmail ? (
                      <div className="mt-1 text-muted-foreground">Reply-to {emailSender.replyToEmail}</div>
                    ) : null}
                  </div>

                  <form onSubmit={sendTestEmail} className="space-y-4">
                    <div className="space-y-2">
                      <Label htmlFor="test-recipient">Recipient email</Label>
                      <Input
                        id="test-recipient"
                        value={testRecipient}
                        onChange={(event) => setTestRecipient(event.target.value)}
                        placeholder="you@company.com"
                        required
                      />
                    </div>
                    <Button
                      type="submit"
                      loading={emailTesting}
                      loadingLabel="Sending test email..."
                      disabled={!resendConfigured}
                    >
                      Send Test Email
                    </Button>
                  </form>

                  <div className="rounded-xl border border-border bg-background/60 p-4 text-sm text-muted-foreground">
                    Client invites already use this sender configuration. Once the domain is verified and selected, invite mails switch over automatically.
                  </div>
                </CardContent>
              </Card>
            </div>

            <Card>
                <CardHeader>
                  <CardTitle>Custom Sending Domain</CardTitle>
                  <CardDescription>
                    Connect a sending subdomain like <span className="font-mono">mail.yourdomain.co.za</span>. Resend handles verification;
                    KasiTek manages the onboarding and sender switching inside the portal.
                  </CardDescription>
                </CardHeader>
                <CardContent className="space-y-6">
                <form onSubmit={addEmailDomain} className="grid gap-4 lg:grid-cols-[1fr_auto] lg:items-end">
                  <div className="space-y-2">
                    <Label htmlFor="email-domain">Sending domain</Label>
                    <Input
                      id="email-domain"
                      value={newEmailDomain}
                      onChange={(event) => setNewEmailDomain(event.target.value)}
                      placeholder="mail.yourdomain.co.za"
                      required
                    />
                    <p className="text-xs text-muted-foreground">
                      Use a dedicated sending subdomain, not the main website root domain.
                    </p>
                  </div>
                  <Button
                    type="submit"
                    loading={creatingEmailDomain}
                    loadingLabel="Adding email domain..."
                    disabled={!resendConfigured}
                  >
                    Add Email Domain
                  </Button>
                </form>

                {emailLoading ? (
                  <p className="text-sm text-muted-foreground">Loading email domains...</p>
                ) : emailDomains.length === 0 ? (
                  <p className="text-sm text-muted-foreground">
                    No custom sending domains configured yet. KasiTek shared sending is already available.
                  </p>
                ) : (
                  <div className="space-y-4">
                    {emailDomains.map((domain) => (
                      <div key={domain.id} className="rounded-xl border border-border bg-background/60 p-4">
                        <div className="flex flex-col gap-4 xl:flex-row xl:items-start xl:justify-between">
                          <div className="space-y-3">
                            <div className="flex flex-wrap items-center gap-2">
                              <span className="font-mono text-sm text-foreground">{domain.domain}</span>
                              <span className={`rounded-full px-2 py-0.5 text-xs font-medium capitalize ${emailStatusTone(domain.status)}`}>
                                {domain.status}
                              </span>
                              {selectedCustomDomainId === domain.id ? (
                                <span className="rounded-full bg-primary/10 px-2 py-0.5 text-xs font-medium text-primary">
                                  Selected
                                </span>
                              ) : null}
                            </div>
                            <p className="text-xs text-muted-foreground">
                              {domain.verifiedAt
                                ? `Verified ${new Date(domain.verifiedAt).toLocaleString()}`
                                : `Added ${new Date(domain.createdAt).toLocaleString()}`}
                            </p>
                            <div className="grid gap-2">
                              {domain.records.map((record) => (
                                <div
                                  key={record.id}
                                  className="rounded-lg border border-border/70 bg-black/20 px-3 py-2 text-xs"
                                >
                                  <div className="flex flex-wrap items-center gap-2">
                                    <span className="font-medium text-foreground">{record.recordType}</span>
                                    <span className="text-muted-foreground">{record.recordName}</span>
                                    <span className={`rounded-full px-2 py-0.5 text-[10px] font-medium capitalize ${emailStatusTone(record.recordStatus)}`}>
                                      {record.recordStatus}
                                    </span>
                                  </div>
                                  <div className="mt-1 break-all font-mono text-muted-foreground">{record.recordValue}</div>
                                  {record.priority ? (
                                    <div className="mt-1 text-muted-foreground">Priority {record.priority}</div>
                                  ) : null}
                                </div>
                              ))}
                            </div>
                          </div>

                          <div className="flex flex-wrap gap-2 xl:w-44 xl:flex-col">
                            <Button
                              variant="outline"
                              size="sm"
                              onClick={() => void verifyEmailDomain(domain.id)}
                              disabled={verifyingEmailDomainId === domain.id}
                            >
                              {verifyingEmailDomainId === domain.id ? (
                                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                              ) : (
                                <RefreshCw className="mr-2 h-4 w-4" />
                              )}
                              Verify
                            </Button>
                            <Button
                              variant="outline"
                              size="sm"
                              onClick={() => setSelectedCustomDomainId(domain.id)}
                              disabled={domain.status !== "verified"}
                            >
                              Select Domain
                            </Button>
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={() => void deleteEmailDomain(domain.id)}
                              disabled={removingEmailDomainId === domain.id}
                              className="text-muted-foreground hover:text-destructive"
                            >
                              {removingEmailDomainId === domain.id ? (
                                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                              ) : (
                                <Trash2 className="mr-2 h-4 w-4" />
                              )}
                              Remove
                            </Button>
                          </div>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </CardContent>
            </Card>
          </div>
        </div>
      </Layout>
    </ProtectedRoute>
  );
}
