import ClientConversationDetailScreen from "@/screens/client-conversation-detail";

export default async function ClientConversationDetailPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = await params;

  return <ClientConversationDetailScreen conversationId={id} />;
}
