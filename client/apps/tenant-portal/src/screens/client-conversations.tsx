"use client";

import Link from "next/link";
import { useState } from "react";
import { useListClientConversations } from "@workspace/api-client-react";
import { ProtectedRoute } from "@/lib/auth";
import { ClientLayout } from "@/components/client-layout";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Skeleton } from "@/components/ui/skeleton";
import { Badge } from "@/components/ui/badge";
import { Search } from "lucide-react";

export default function ClientConversationsScreen() {
  const [search, setSearch] = useState("");
  const { data: conversations, isLoading } = useListClientConversations();

  const filtered = conversations?.filter((conversation) => {
    const haystack = [
      conversation.customer_name,
      conversation.customer_identifier,
      conversation.channel,
      conversation.status,
    ]
      .filter(Boolean)
      .join(" ")
      .toLowerCase();

    return haystack.includes(search.toLowerCase());
  });

  return (
    <ProtectedRoute audience="client">
      <ClientLayout>
        <div className="space-y-6">
          <div>
            <h1 className="text-3xl font-semibold tracking-tight">Client Conversations</h1>
            <p className="mt-2 text-muted-foreground">
              Only conversations associated with this client account are listed here.
            </p>
          </div>

          <Card>
            <CardHeader>
              <CardTitle>Conversation Log</CardTitle>
              <CardDescription>Search and review client-scoped conversations.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="relative max-w-md">
                <Search className="absolute left-3 top-3.5 h-4 w-4 text-muted-foreground" />
                <Input
                  value={search}
                  onChange={(event) => setSearch(event.target.value)}
                  placeholder="Search conversations"
                  className="pl-10"
                />
              </div>

              {isLoading ? (
                <div className="space-y-3">
                  {Array.from({ length: 5 }).map((_, index) => (
                    <Skeleton key={index} className="h-24 w-full rounded-xl" />
                  ))}
                </div>
              ) : filtered?.length ? (
                <div className="space-y-3">
                  {filtered.map((conversation) => (
                    <Link
                      key={conversation.id}
                      href={`/client/conversations/${conversation.id}`}
                      className="block rounded-xl border border-border px-4 py-4 transition-colors hover:border-primary/40 hover:bg-muted/40"
                    >
                      <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
                        <div className="min-w-0">
                          <div className="flex items-center gap-2">
                            <span className="font-medium">
                              {conversation.customer_name || conversation.customer_identifier}
                            </span>
                            <Badge variant="outline">{conversation.channel}</Badge>
                            <Badge variant="secondary">{conversation.status}</Badge>
                          </div>
                          <p className="mt-2 text-sm text-muted-foreground">
                            Conversation ID: {conversation.id}
                          </p>
                        </div>
                        <p className="text-sm text-muted-foreground">
                          {new Date(conversation.last_message_at).toLocaleString()}
                        </p>
                      </div>
                    </Link>
                  ))}
                </div>
              ) : (
                <div className="rounded-xl border border-dashed border-border px-6 py-10 text-center text-muted-foreground">
                  No conversations matched your search.
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      </ClientLayout>
    </ProtectedRoute>
  );
}
