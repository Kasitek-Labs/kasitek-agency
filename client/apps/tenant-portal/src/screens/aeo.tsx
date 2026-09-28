import { useState } from "react";
import { useListAeoContent, useCreateAeoContent, useGetAeoCitations } from "@workspace/api-client-react";
import { ProtectedRoute } from "@/lib/auth";
import { Layout } from "@/components/layout";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Form, FormControl, FormField, FormItem, FormLabel, FormMessage } from "@/components/ui/form";
import { Skeleton } from "@/components/ui/skeleton";
import { Checkbox } from "@/components/ui/checkbox";
import { Textarea } from "@/components/ui/textarea";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { zodResolver } from "@hookform/resolvers/zod";
import { Plus, SearchCheck, ExternalLink, Hash, TrendingUp, Cpu } from "lucide-react";
import { useQueryClient } from "@tanstack/react-query";
import { getListAeoContentQueryKey, getGetAeoCitationsQueryKey } from "@workspace/api-client-react";
import { Badge } from "@/components/ui/badge";
import { TENANT_ROUTE_ACCESS } from "@/lib/tenant-access";

const createAeoSchema = z.object({
  title: z.string().min(1, "Title is required"),
  targetQuery: z.string().min(1, "Target query is required"),
  businessContext: z.string().min(10, "Provide business context"),
  platforms: z.array(z.enum(["chatgpt", "perplexity", "google_ai", "bing_chat"])).min(1, "Select at least one platform"),
});

export default function AEO() {
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const queryClient = useQueryClient();

  const { data: contents, isLoading: loadingContent } = useListAeoContent();
  const { data: stats, isLoading: loadingStats } = useGetAeoCitations();
  const createAeo = useCreateAeoContent();

  const form = useForm<z.infer<typeof createAeoSchema>>({
    resolver: zodResolver(createAeoSchema),
    defaultValues: {
      title: "",
      targetQuery: "",
      businessContext: "",
      platforms: ["chatgpt", "perplexity"],
    },
  });

  const onSubmit = (values: z.infer<typeof createAeoSchema>) => {
    createAeo.mutate(
      { data: values },
      {
        onSuccess: () => {
          queryClient.invalidateQueries({ queryKey: getListAeoContentQueryKey() });
          queryClient.invalidateQueries({ queryKey: getGetAeoCitationsQueryKey() });
          setIsDialogOpen(false);
          form.reset();
        },
      }
    );
  };

  const platformOptions = [
    { id: "chatgpt", label: "ChatGPT" },
    { id: "perplexity", label: "Perplexity" },
    { id: "google_ai", label: "Google AI" },
    { id: "bing_chat", label: "Bing Chat" },
  ] as const;

  return (
    <ProtectedRoute allowedRoles={TENANT_ROUTE_ACCESS.aeo}>
      <Layout>
        <div className="p-8 space-y-8 animate-in fade-in duration-500">
          <div className="flex justify-between items-end">
            <div>
              <h1 className="text-3xl font-bold tracking-tight">Answer Engine Optimization</h1>
              <p className="text-muted-foreground mt-1">Optimize your brand presence in AI models and search chats.</p>
            </div>
            <Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
              <DialogTrigger asChild>
                <Button>
                  <Plus className="w-4 h-4 mr-2" />
                  Optimize Target
                </Button>
              </DialogTrigger>
              <DialogContent className="sm:max-w-[500px]">
                <DialogHeader>
                  <DialogTitle>New AEO Target</DialogTitle>
                  <DialogDescription>
                    Create content structured specifically for LLM ingestion and retrieval.
                  </DialogDescription>
                </DialogHeader>
                <Form {...form}>
                  <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
                    <FormField
                      control={form.control}
                      name="targetQuery"
                      render={({ field }) => (
                         <FormItem>
                          <FormLabel>Target Query</FormLabel>
                          <FormControl>
                            <Input placeholder="e.g. Best CRM for real estate agents" {...field} />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                    <FormField
                      control={form.control}
                      name="title"
                      render={({ field }) => (
                         <FormItem>
                          <FormLabel>Content Title</FormLabel>
                          <FormControl>
                            <Input placeholder="Definitive Guide to Real Estate CRMs" {...field} />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                    <FormField
                      control={form.control}
                      name="businessContext"
                      render={({ field }) => (
                         <FormItem>
                          <FormLabel>Business Context / Key Facts</FormLabel>
                          <FormControl>
                            <Textarea
                              placeholder="Factual information about your product that the AI should know..."
                              className="min-h-[100px]"
                              {...field}
                            />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                    <FormField
                      control={form.control}
                      name="platforms"
                      render={() => (
                        <FormItem>
                          <div className="mb-4">
                            <FormLabel>Target Platforms</FormLabel>
                            <CardDescription>Select the AI engines to format content for</CardDescription>
                          </div>
                          <div className="grid grid-cols-2 gap-2">
                            {platformOptions.map((item) => (
                              <FormField
                                key={item.id}
                                control={form.control}
                                name="platforms"
                                render={({ field }) => {
                                  return (
                                    <FormItem
                                      key={item.id}
                                      className="flex flex-row items-start space-x-3 space-y-0 rounded-md border p-4"
                                    >
                                      <FormControl>
                                        <Checkbox
                                          checked={field.value?.includes(item.id)}
                                          onCheckedChange={(checked) => {
                                            return checked
                                              ? field.onChange([...field.value, item.id])
                                              : field.onChange(
                                                  field.value?.filter(
                                                    (value) => value !== item.id
                                                  )
                                                )
                                          }}
                                        />
                                      </FormControl>
                                      <FormLabel className="font-normal cursor-pointer">
                                        {item.label}
                                      </FormLabel>
                                    </FormItem>
                                  )
                                }}
                              />
                            ))}
                          </div>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                    <div className="flex justify-end pt-4">
                      <Button type="submit" loading={createAeo.isPending} loadingLabel="Generating structure...">
                        Generate Structure
                      </Button>
                    </div>
                  </form>
                </Form>
              </DialogContent>
            </Dialog>
          </div>

          <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
            <div className="lg:col-span-2 space-y-6">
              <Card>
                <CardHeader>
                  <CardTitle>Optimization Pipeline</CardTitle>
                </CardHeader>
                <CardContent>
                  {loadingContent ? (
                     <div className="space-y-4">
                     {Array.from({ length: 3 }).map((_, i) => (
                       <Skeleton key={i} className="h-24 w-full" />
                     ))}
                   </div>
                  ) : contents && contents.length > 0 ? (
                    <div className="space-y-4">
                      {contents.map((content) => (
                        <div key={content.id} className="flex flex-col sm:flex-row gap-4 p-4 border border-border rounded-lg bg-card/50 hover:bg-muted/30 transition-colors">
                          <div className="flex-1 space-y-2">
                            <div className="flex items-center justify-between">
                              <h3 className="font-bold text-lg">{content.title}</h3>
                              <Badge variant={content.status === 'indexed' ? 'default' : 'secondary'} className="capitalize">
                                {content.status}
                              </Badge>
                            </div>
                            <div className="flex items-center text-sm text-primary">
                              <SearchCheck className="w-4 h-4 mr-2" />
                              "{content.targetQuery}"
                            </div>
                            <div className="flex items-center gap-2 pt-2">
                              {content.platforms.map((platform) => (
                                <Badge key={platform} variant="outline" className="text-[10px] capitalize bg-background">
                                  {platform.replace('_', ' ')}
                                </Badge>
                              ))}
                            </div>
                          </div>
                          <div className="flex sm:flex-col items-center sm:items-end justify-between sm:justify-center border-t sm:border-t-0 sm:border-l border-border pt-4 sm:pt-0 sm:pl-4 min-w-[120px]">
                            <div className="text-3xl font-bold text-primary">{content.citationCount}</div>
                            <div className="text-xs text-muted-foreground font-medium uppercase tracking-wider">Citations</div>
                          </div>
                        </div>
                      ))}
                    </div>
                  ) : (
                    <div className="text-center py-12 text-muted-foreground border border-dashed rounded-lg">
                      <Cpu className="w-12 h-12 mx-auto mb-4 opacity-20" />
                      <p>No AEO targets configured.</p>
                    </div>
                  )}
                </CardContent>
              </Card>
            </div>

            <div className="space-y-6">
              {loadingStats ? (
                <Skeleton className="h-[400px] w-full rounded-xl" />
              ) : stats && (
                <>
                  <Card>
                    <CardHeader className="pb-4">
                      <CardTitle className="text-lg">Citation Impact</CardTitle>
                    </CardHeader>
                    <CardContent>
                      <div className="flex items-center justify-between mb-6">
                        <div>
                          <div className="text-4xl font-bold text-primary">{stats.totalCitations}</div>
                          <div className="text-sm text-muted-foreground">Total LLM Mentions</div>
                        </div>
                        <div className="text-right">
                          <div className="flex items-center text-green-500 font-medium">
                            <TrendingUp className="w-4 h-4 mr-1" />
                            +{stats.citationsThisMonth}
                          </div>
                          <div className="text-xs text-muted-foreground">This month</div>
                        </div>
                      </div>

                      <div className="space-y-4">
                        <h4 className="text-sm font-semibold text-muted-foreground border-b border-border pb-2">Top Platforms</h4>
                        {stats.topPlatforms.map((platform, idx) => (
                          <div key={idx} className="flex items-center justify-between">
                            <div className="flex items-center text-sm capitalize">
                              <Hash className="w-4 h-4 mr-2 text-muted-foreground" />
                              {platform.platform.replace('_', ' ')}
                            </div>
                            <div className="font-mono text-sm">{platform.citations}</div>
                          </div>
                        ))}
                      </div>
                    </CardContent>
                  </Card>

                  <Card>
                    <CardHeader className="pb-4">
                      <CardTitle className="text-lg">Recent Detections</CardTitle>
                    </CardHeader>
                    <CardContent className="p-0">
                      <div className="divide-y divide-border">
                        {stats.recentCitations.map((citation, idx) => (
                          <div key={idx} className="p-4 text-sm hover:bg-muted/50 transition-colors">
                            <div className="font-medium text-primary flex items-center justify-between mb-1">
                              <span className="capitalize">{citation.platform.replace('_', ' ')}</span>
                              <span className="text-xs text-muted-foreground">{new Date(citation.date).toLocaleDateString()}</span>
                            </div>
                            <div className="text-muted-foreground italic">"...{citation.query}..."</div>
                          </div>
                        ))}
                      </div>
                    </CardContent>
                  </Card>
                </>
              )}
            </div>
          </div>
        </div>
      </Layout>
    </ProtectedRoute>
  );
}
