import ClientDetailScreen from "@/screens/client-detail";

export default async function Page({
  params,
  searchParams,
}: {
  params: Promise<{ id: string }>;
  searchParams?: Promise<{
    adminInviteEmail?: string;
    adminInviteSent?: string;
    adminInviteUrl?: string;
  }>;
}) {
  const { id } = await params;
  const resolvedSearchParams = searchParams ? await searchParams : {};

  return (
    <ClientDetailScreen
      clientId={id}
      initialAdminInviteEmail={resolvedSearchParams.adminInviteEmail ?? null}
      initialAdminInviteSent={resolvedSearchParams.adminInviteSent === "1"}
      initialAdminInviteUrl={resolvedSearchParams.adminInviteUrl ?? null}
    />
  );
}
