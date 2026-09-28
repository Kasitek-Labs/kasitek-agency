"use client";

import Link from "next/link";
import {
  useGetClientConversation,
  useListClientConversationMessages,
} from "@workspace/api-client-react";
import { ArrowLeft, Clock3, MessageSquareText } from "lucide-react";
import { ProtectedRoute } from "@/lib/auth";
import { ClientLayout } from "@/components/client-layout";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";

export default function ClientConversationDetailScreen({
  conversationId,
}: {
  conversationId: string;
}) {
  const { data: conversation, isLoading: loadingConversation } =
    useGetClientConversation(conversationId);
  const { data: messages, isLoading: loadingMessages } =
    useListClientConversationMessages(conversationId);

  return (
    <ProtectedRoute audience="client">
      <ClientLayout>
        <div className="space-y-6">
          <div className="space-y-3">
            <Link
              href="/client/conversations"
              className="inline-flex items-center gap-2 text-sm text-muted-foreground transition-colors hover:text-foreground"
            >
              <ArrowLeft className="h-4 w-4" />
              Back to conversations
            </Link>

            {loadingConversation ? (
              <Skeleton className="h-28 rounded-xl" />
            ) : conversation ? (
              <Card>
                <CardHeader>
                  <div className="flex flex-wrap items-center gap-2">
                    <CardTitle>
                      {conversation.customer_name || conversation.customer_identifier}
                    </CardTitle>
                    <Badge variant="outline">{conversation.channel}</Badge>
                    <Badge variant="secondary">{conversation.status}</Badge>
                  </div>
                  <CardDescription>
                    Conversation ID {conversation.id}
                  </CardDescription>
                </CardHeader>
                <CardContent className="flex flex-wrap gap-6 text-sm text-muted-foreground">
                  <div className="inline-flex items-center gap-2">
                    <Clock3 className="h-4 w-4" />
                    Last activity {new Date(conversation.last_message_at).toLocaleString()}
                  </div>
                  <div className="inline-flex items-center gap-2">
                    <MessageSquareText className="h-4 w-4" />
                    Started {new Date(conversation.created_at).toLocaleString()}
                  </div>
                </CardContent>
              </Card>
            ) : (
              <Card>
                <CardContent className="px-6 py-12 text-center text-muted-foreground">
                  Conversation not found.
                </CardContent>
              </Card>
            )}
          </div>

          <Card>
            <CardHeader>
              <CardTitle>Messages</CardTitle>
              <CardDescription>Client-scoped conversation history.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              {loadingMessages ? (
                Array.from({ length: 5 }).map((_, index) => (
                  <Skeleton key={index} className="h-24 rounded-xl" />
                ))
              ) : messages?.length ? (
                messages.map((message) => (
                  <div
                    key={message.id}
                    className="rounded-xl border border-border bg-card px-4 py-4"
                  >
                    <div className="flex flex-wrap items-center justify-between gap-3">
                      <Badge variant="outline">{message.sender_role}</Badge>
                      <span className="text-xs text-muted-foreground">
                        {new Date(message.created_at).toLocaleString()}
                      </span>
                    </div>
                    <p className="mt-3 whitespace-pre-wrap text-sm leading-6 text-foreground">
                      {message.content}
                    </p>
                  </div>
                ))
              ) : (
                <div className="rounded-xl border border-dashed border-border px-6 py-10 text-center text-muted-foreground">
                  No messages yet.
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      </ClientLayout>
    </ProtectedRoute>
  );
}
