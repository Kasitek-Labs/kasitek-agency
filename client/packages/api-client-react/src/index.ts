import { useMutation, useQuery } from "@tanstack/react-query";

const placeholder = <T,>(data: T) => data;
const DEFAULT_TENANT_API_BASE_URL = "/api/tenant";

function getTenantApiBaseUrl() {
  return DEFAULT_TENANT_API_BASE_URL;
}

function tenantApiUrl(path: string) {
  return `${getTenantApiBaseUrl()}${path.startsWith("/") ? path : `/${path}`}`;
}

async function fetchTenantJson<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(tenantApiUrl(path), {
    credentials: "include",
    ...init,
    headers: {
      "Content-Type": "application/json",
      ...(init?.headers ?? {}),
    },
  });

  const payload = (await response.json().catch(() => ({}))) as T & { error?: string };
  if (!response.ok) {
    throw new Error(payload.error || `Request failed with status ${response.status}`);
  }

  return payload;
}

export const getListReportingDashboardsQueryKey = () => ["tenant", "reporting", "dashboards"];
export const getListTenantClientsQueryKey = () => ["tenant", "clients"];
export const getListTenantClientUsersQueryKey = (tenantClientId: string) => [
  "tenant",
  "clients",
  tenantClientId,
  "users",
];
export const getClientOverviewQueryKey = () => ["tenant", "client", "overview"];
export const getClientConversationsQueryKey = () => ["tenant", "client", "conversations"];
export const getClientLeadsQueryKey = () => ["tenant", "client", "leads"];
export const getClientLeadStatsQueryKey = () => ["tenant", "client", "leads", "stats"];
export const getClientConversationQueryKey = (conversationId: string) => [
  "tenant",
  "client",
  "conversations",
  conversationId,
];
export const getClientConversationMessagesQueryKey = (conversationId: string) => [
  "tenant",
  "client",
  "conversations",
  conversationId,
  "messages",
];
export const getListNurturingSequencesQueryKey = () => ["tenant", "nurturing", "sequences"];
export const getGetNurturingStatsQueryKey = () => ["tenant", "nurturing", "stats"];
export const getListAeoContentQueryKey = () => ["tenant", "aeo", "content"];
export const getGetAeoCitationsQueryKey = () => ["tenant", "aeo", "citations"];
export const getListContentHistoryQueryKey = () => ["tenant", "content", "history"];
export const getListLeadsQueryKey = () => ["tenant", "leads"];
export const getGetLeadsStatsQueryKey = () => ["tenant", "leads", "stats"];
export const getListReviewCampaignsQueryKey = () => ["tenant", "reviews", "campaigns"];
export const getGetReviewStatsQueryKey = () => ["tenant", "reviews", "stats"];
export const getListAdCampaignsQueryKey = () => ["tenant", "ads", "campaigns"];
export const getListAdCreativesQueryKey = () => ["tenant", "ads", "creatives"];
export const getListVoiceAgentsQueryKey = () => ["tenant", "voice", "agents"];
export const getListVoiceCallsQueryKey = () => ["tenant", "voice", "calls"];

export function useGetDashboardSummary(..._args: any[]) {
  return useQuery({
    queryKey: ["tenant", "dashboard", "summary"],
    queryFn: async () =>
      fetchTenantJson<{
        summary: {
          workspaceId: string;
          workspaceSlug: string;
          totalClients: number;
          activeAgents: number;
          totalConversations: number;
          totalLeads: number;
          requestsThisMonth: number;
        };
      }>("/api/dashboard/summary").then((payload) => payload.summary),
  });
}

export function useGetDashboardActivity(..._args: any[]) {
  return useQuery<any>({
    queryKey: ["tenant", "dashboard", "activity"],
    queryFn: async () =>
      fetchTenantJson<{
        activity: Array<{
          id: string;
          title: string;
          description: string;
          timestamp: string;
        }>;
      }>("/api/dashboard/activity").then((payload) => payload.activity),
  });
}

export function useListServices(..._args: any[]) {
  return useQuery<any>({
    queryKey: ["tenant", "services"],
    queryFn: async () =>
      fetchTenantJson<{
        services: Array<{ id: string; name: string; isActive: boolean }>;
      }>("/api/dashboard/services").then((payload) => payload.services),
  });
}

export function useListLeads(..._args: any[]) {
  return useQuery({
    queryKey: getListLeadsQueryKey(),
    queryFn: async () =>
      fetchTenantJson<{
        leads: Array<{
          id: string;
          tenant_client_id: string;
          tenant_client_name: string;
          name: string;
          email: string;
          phone?: string | null;
          company?: string | null;
          source: string;
          status: string;
          score: number;
          notes?: string | null;
          created_at: string;
          updated_at: string;
        }>;
      }>("/api/leads").then((payload) => payload),
  });
}

export function useListTenantClients(..._args: any[]) {
  return useQuery({
    queryKey: getListTenantClientsQueryKey(),
    queryFn: async () =>
      fetchTenantJson<{
        clients: Array<{
          id: string;
          name: string;
          slug: string;
          status: string;
          user_count: number;
          external_ref?: string | null;
          metadata?: Record<string, unknown>;
          created_at: string;
          updated_at: string;
        }>;
      }>("/api/clients").then((payload) => payload.clients),
  });
}

export function useCreateTenantClient(..._args: any[]) {
  return useMutation({
    mutationFn: async (input: {
      name: string;
      slug?: string;
      external_ref?: string;
      admin_email: string;
    }) =>
      fetchTenantJson<{
        client: {
          id: string;
          workspaceId: string;
          name: string;
          slug: string;
          externalRef?: string | null;
          status: string;
        };
        adminInvite: {
          email: string;
          emailSent: boolean;
          activationUrl?: string | null;
          portalUrl?: string | null;
          expiresAt: string;
        };
      }>("/api/clients", {
        method: "POST",
        body: JSON.stringify(input),
      }),
  });
}

export function useListTenantClientUsers(tenantClientId: string, options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: getListTenantClientUsersQueryKey(tenantClientId),
    enabled: options?.enabled ?? !!tenantClientId,
    queryFn: async () =>
      fetchTenantJson<{
        users: Array<{
          id: string;
          tenant_client_id: string;
          email: string;
          name?: string | null;
          status: string;
          created_at: string;
          updated_at: string;
        }>;
      }>(`/api/clients/${tenantClientId}/users`).then((payload) => payload.users),
  });
}

export function useInviteTenantClientUser(..._args: any[]) {
  return useMutation({
    mutationFn: async (input: {
      tenantClientId: string;
      email: string;
      name?: string;
      role?: string;
    }) =>
      fetchTenantJson<{
        invite: {
          id: string;
          tenantClientId: string;
        tenantClientUserId: string;
        email: string;
        role: string;
        expiresAt: string;
        token: string;
        activationUrl?: string | null;
        emailSent: boolean;
      };
      }>(`/api/clients/${input.tenantClientId}/invites`, {
        method: "POST",
        body: JSON.stringify({
          email: input.email,
          name: input.name,
          role: input.role,
        }),
      }).then((payload) => payload.invite),
  });
}

export function useGetClientOverview(..._args: any[]) {
  return useQuery({
    queryKey: getClientOverviewQueryKey(),
    queryFn: async () =>
      fetchTenantJson<{
        overview: {
          workspaceId: string;
          workspaceSlug: string;
          tenantClientId: string;
          tenantClientName: string;
          totalUsers: number;
          totalConversations: number;
          openConversations: number;
          totalMessages: number;
        };
      }>("/api/client/overview").then((payload) => payload.overview),
  });
}

export function useListClientConversations(..._args: any[]) {
  return useQuery({
    queryKey: getClientConversationsQueryKey(),
    queryFn: async () =>
      fetchTenantJson<{
        conversations: Array<{
          id: string;
          external_id?: string | null;
          channel: string;
          customer_identifier: string;
          customer_name?: string | null;
          status: string;
          metadata: Record<string, unknown>;
          last_message_at: string;
          created_at: string;
          updated_at: string;
        }>;
      }>("/api/client/conversations").then((payload) => payload.conversations),
  });
}

export function useGetClientConversation(conversationId: string, options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: getClientConversationQueryKey(conversationId),
    enabled: options?.enabled ?? !!conversationId,
    queryFn: async () =>
      fetchTenantJson<{
        conversation: {
          id: string;
          external_id?: string | null;
          channel: string;
          customer_identifier: string;
          customer_name?: string | null;
          status: string;
          metadata: Record<string, unknown>;
          last_message_at: string;
          created_at: string;
          updated_at: string;
        };
      }>(`/api/client/conversations/${conversationId}`).then((payload) => payload.conversation),
  });
}

export function useListClientConversationMessages(
  conversationId: string,
  options?: { enabled?: boolean },
) {
  return useQuery({
    queryKey: getClientConversationMessagesQueryKey(conversationId),
    enabled: options?.enabled ?? !!conversationId,
    queryFn: async () =>
      fetchTenantJson<{
        messages: Array<{
          id: string;
          sender_role: string;
          content: string;
          metadata: Record<string, unknown>;
          created_at: string;
        }>;
      }>(`/api/client/conversations/${conversationId}/messages`).then((payload) => payload.messages),
  });
}

export function useGetLeadsStats(..._args: any[]) {
  return useQuery({
    queryKey: getGetLeadsStatsQueryKey(),
    queryFn: async () =>
      fetchTenantJson<{
        stats: {
          total: number;
          qualified: number;
          converted: number;
          weeklyTrend: Array<{ date: string; count: number }>;
          byStatus: Array<{ status: string; count: number }>;
        };
      }>("/api/leads/stats").then((payload) => payload.stats),
  });
}

export function useCreateLead(..._args: any[]) {
  return useMutation({
    mutationFn: async (input: {
      tenantClientId: string;
      name: string;
      email: string;
      phone?: string;
      company?: string;
      source: string;
      notes?: string;
    }) =>
      fetchTenantJson<{
        lead: {
          id: string;
          tenantClientId: string;
          tenantClientName: string;
          name: string;
          email: string;
          phone?: string | null;
          company?: string | null;
          source: string;
          status: string;
          score: number;
          notes?: string | null;
        };
      }>("/api/leads", {
        method: "POST",
        body: JSON.stringify(input),
      }).then((payload) => payload.lead),
  });
}

export function useListClientLeads(..._args: any[]) {
  return useQuery({
    queryKey: getClientLeadsQueryKey(),
    queryFn: async () =>
      fetchTenantJson<{
        leads: Array<{
          id: string;
          tenant_client_id: string;
          tenant_client_name: string;
          name: string;
          email: string;
          phone?: string | null;
          company?: string | null;
          source: string;
          status: string;
          score: number;
          notes?: string | null;
          created_at: string;
          updated_at: string;
        }>;
      }>("/api/client/leads").then((payload) => payload.leads),
  });
}

export function useGetClientLeadStats(..._args: any[]) {
  return useQuery({
    queryKey: getClientLeadStatsQueryKey(),
    queryFn: async () =>
      fetchTenantJson<{
        stats: {
          total: number;
          qualified: number;
          converted: number;
          byStatus: Array<{ status: string; count: number }>;
        };
      }>("/api/client/leads/stats").then((payload) => payload.stats),
  });
}

export function useGenerateContent(..._args: any[]) {
  return useMutation<any, any, any>({
    mutationFn: async (input: unknown) =>
      placeholder({
        input,
        content: "Generated content placeholder",
      }),
  });
}

export function useListContentHistory(..._args: any[]) {
  return useQuery<any>({
    queryKey: getListContentHistoryQueryKey(),
    queryFn: async () => placeholder([]),
  });
}

export function useListReportingDashboards(..._args: any[]) {
  return useQuery<any>({
    queryKey: getListReportingDashboardsQueryKey(),
    queryFn: async () => placeholder([]),
  });
}

export function useCreateReportingDashboard(..._args: any[]) {
  return useMutation<any, any, any>({
    mutationFn: async (input: unknown) => placeholder(input),
  });
}

export function useGetDashboardMetrics(..._args: any[]) {
  return useQuery<any>({
    queryKey: ["tenant", "reporting", "metrics"],
    queryFn: async () =>
      placeholder({
        sessions: 0,
        leads: 0,
        conversions: 0,
        revenue: 0,
        monthlyTrend: [],
      }),
  });
}

export function useListNurturingSequences(..._args: any[]) {
  return useQuery<any>({
    queryKey: getListNurturingSequencesQueryKey(),
    queryFn: async () => placeholder([]),
  });
}

export function useCreateNurturingSequence(..._args: any[]) {
  return useMutation<any, any, any>({
    mutationFn: async (input: unknown) => placeholder(input),
  });
}

export function useGetNurturingStats(..._args: any[]) {
  return useQuery<any>({
    queryKey: getGetNurturingStatsQueryKey(),
    queryFn: async () =>
      placeholder({
        total: 0,
        active: 0,
        conversions: 0,
        totalEnrolled: 0,
        activeSequences: 0,
        avgOpenRate: 0,
        avgResponseRate: 0,
        conversionsThisMonth: 0,
      }),
  });
}

export function useListAeoContent(..._args: any[]) {
  return useQuery<any>({
    queryKey: getListAeoContentQueryKey(),
    queryFn: async () => placeholder([]),
  });
}

export function useCreateAeoContent(..._args: any[]) {
  return useMutation<any, any, any>({
    mutationFn: async (input: unknown) => placeholder(input),
  });
}

export function useGetAeoCitations(..._args: any[]) {
  return useQuery<any>({
    queryKey: getGetAeoCitationsQueryKey(),
    queryFn: async () =>
      placeholder({
        totalCitations: 0,
        citationsThisMonth: 0,
        topPlatforms: [],
        recentCitations: [],
      }),
  });
}

export function useListReviewCampaigns(..._args: any[]) {
  return useQuery<any>({
    queryKey: getListReviewCampaignsQueryKey(),
    queryFn: async () => placeholder([]),
  });
}

export function useCreateReviewCampaign(..._args: any[]) {
  return useMutation<any, any, any>({
    mutationFn: async (input: unknown) => placeholder(input),
  });
}

export function useGetReviewStats(..._args: any[]) {
  return useQuery<any>({
    queryKey: getGetReviewStatsQueryKey(),
    queryFn: async () =>
      placeholder({
        total: 0,
        sent: 0,
        reviews: 0,
        avgRating: 0,
        totalReviews: 0,
        positiveRate: 0,
        googleReviews: 0,
        negativesCaptured: 0,
        ratingDistribution: [],
      }),
  });
}

export function useListAdCampaigns(..._args: any[]) {
  return useQuery<any>({
    queryKey: getListAdCampaignsQueryKey(),
    queryFn: async () => placeholder([]),
  });
}

export function useCreateAdCampaign(..._args: any[]) {
  return useMutation<any, any, any>({
    mutationFn: async (input: unknown) => placeholder(input),
  });
}

export function useListAdCreatives(..._args: any[]) {
  return useQuery<any>({
    queryKey: getListAdCreativesQueryKey(),
    queryFn: async () => placeholder([]),
  });
}

export function useListVoiceAgents(..._args: any[]) {
  return useQuery<any>({
    queryKey: getListVoiceAgentsQueryKey(),
    queryFn: async () => placeholder([]),
  });
}

export function useCreateVoiceAgent(..._args: any[]) {
  return useMutation<any, any, any>({
    mutationFn: async (input: unknown) => placeholder(input),
  });
}

export function useListVoiceCalls(..._args: any[]) {
  return useQuery<any>({
    queryKey: getListVoiceCallsQueryKey(),
    queryFn: async () => placeholder([]),
  });
}
