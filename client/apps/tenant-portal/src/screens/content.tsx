import { useState } from "react";
import { useGenerateContent, useListContentHistory } from "@workspace/api-client-react";
import { ProtectedRoute } from "@/lib/auth";
import { Layout } from "@/components/layout";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Form, FormControl, FormField, FormItem, FormLabel, FormMessage } from "@/components/ui/form";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { zodResolver } from "@hookform/resolvers/zod";
import { Sparkles, Copy, Check, Clock } from "lucide-react";
import { useQueryClient } from "@tanstack/react-query";
import { getListContentHistoryQueryKey } from "@workspace/api-client-react";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { TENANT_ROUTE_ACCESS } from "@/lib/tenant-access";

const generateSchema = z.object({
  topic: z.string().min(3, "Topic is required"),
  type: z.enum(["blog_post", "social_caption", "ad_copy", "email_sequence", "product_description"]),
  tone: z.enum(["professional", "casual", "persuasive", "informative"]),
});

export default function ContentPipeline() {
  const queryClient = useQueryClient();
  const [generatedContent, setGeneratedContent] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  const { data: history, isLoading: loadingHistory } = useListContentHistory();
  const generateMutation = useGenerateContent();

  const form = useForm<z.infer<typeof generateSchema>>({
    resolver: zodResolver(generateSchema),
    defaultValues: {
      topic: "",
      type: "social_caption",
      tone: "professional",
    },
  });

  const onSubmit = (values: z.infer<typeof generateSchema>) => {
    generateMutation.mutate(
      { data: values },
      {
        onSuccess: (res) => {
          setGeneratedContent(res.content);
          queryClient.invalidateQueries({ queryKey: getListContentHistoryQueryKey() });
        },
      }
    );
  };

  const handleCopy = () => {
    if (generatedContent) {
      navigator.clipboard.writeText(generatedContent);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  return (
    <ProtectedRoute allowedRoles={TENANT_ROUTE_ACCESS.content}>
      <Layout>
        <div className="p-8 space-y-8 animate-in fade-in duration-500">
          <div>
            <h1 className="text-3xl font-bold tracking-tight">Content Pipeline</h1>
            <p className="text-muted-foreground mt-1">Generate high-quality marketing copy using AI.</p>
          </div>

          <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
            <div className="lg:col-span-1 space-y-6">
              <Card>
                <CardHeader>
                  <CardTitle>Generator</CardTitle>
                  <CardDescription>Configure your content request</CardDescription>
                </CardHeader>
                <CardContent>
                  <Form {...form}>
                    <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
                      <FormField
                        control={form.control}
                        name="topic"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Topic or Prompt</FormLabel>
                            <FormControl>
                              <Input placeholder="e.g. 5 tips for remote work" {...field} />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />

                      <FormField
                        control={form.control}
                        name="type"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Content Type</FormLabel>
                            <Select onValueChange={field.onChange} defaultValue={field.value}>
                              <FormControl>
                                <SelectTrigger>
                                  <SelectValue placeholder="Select type" />
                                </SelectTrigger>
                              </FormControl>
                              <SelectContent>
                                <SelectItem value="blog_post">Blog Post</SelectItem>
                                <SelectItem value="social_caption">Social Media Caption</SelectItem>
                                <SelectItem value="ad_copy">Ad Copy</SelectItem>
                                <SelectItem value="email_sequence">Email Sequence</SelectItem>
                                <SelectItem value="product_description">Product Description</SelectItem>
                              </SelectContent>
                            </Select>
                            <FormMessage />
                          </FormItem>
                        )}
                      />

                      <FormField
                        control={form.control}
                        name="tone"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Tone of Voice</FormLabel>
                            <Select onValueChange={field.onChange} defaultValue={field.value}>
                              <FormControl>
                                <SelectTrigger>
                                  <SelectValue placeholder="Select tone" />
                                </SelectTrigger>
                              </FormControl>
                              <SelectContent>
                                <SelectItem value="professional">Professional</SelectItem>
                                <SelectItem value="casual">Casual</SelectItem>
                                <SelectItem value="persuasive">Persuasive</SelectItem>
                                <SelectItem value="informative">Informative</SelectItem>
                              </SelectContent>
                            </Select>
                            <FormMessage />
                          </FormItem>
                        )}
                      />

                      <Button
                        type="submit"
                        className="w-full"
                        loading={generateMutation.isPending}
                        loadingLabel="Generating content..."
                      >
                        <>
                          <Sparkles className="w-4 h-4 mr-2" />
                          Generate Content
                        </>
                      </Button>
                    </form>
                  </Form>
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle className="flex items-center text-sm">
                    <Clock className="w-4 h-4 mr-2 text-muted-foreground" />
                    Recent Generations
                  </CardTitle>
                </CardHeader>
                <CardContent className="p-0">
                  <ScrollArea className="h-[300px]">
                    <div className="flex flex-col">
                      {loadingHistory ? (
                        <div className="p-4 space-y-4">
                          {Array.from({ length: 4 }).map((_, i) => (
                            <Skeleton key={i} className="h-12 w-full" />
                          ))}
                        </div>
                      ) : history?.length ? (
                        history.map((item) => (
                          <div key={item.id} className="p-4 border-b border-border hover:bg-muted/50 cursor-pointer transition-colors last:border-0">
                            <div className="font-medium text-sm line-clamp-1">{item.topic}</div>
                            <div className="flex items-center justify-between mt-2">
                              <Badge variant="secondary" className="text-[10px] capitalize">
                                {item.type.replace('_', ' ')}
                              </Badge>
                              <span className="text-xs text-muted-foreground">
                                {new Date(item.createdAt).toLocaleDateString()}
                              </span>
                            </div>
                          </div>
                        ))
                      ) : (
                        <div className="p-8 text-center text-sm text-muted-foreground">
                          No history yet
                        </div>
                      )}
                    </div>
                  </ScrollArea>
                </CardContent>
              </Card>
            </div>

            <div className="lg:col-span-2">
              <Card className="h-full min-h-[600px] flex flex-col">
                <CardHeader className="flex flex-row items-center justify-between pb-2 border-b border-border/50">
                  <CardTitle>Result</CardTitle>
                  <Button variant="outline" size="sm" onClick={handleCopy} disabled={!generatedContent}>
                    {copied ? <Check className="w-4 h-4 mr-2 text-green-500" /> : <Copy className="w-4 h-4 mr-2" />}
                    {copied ? "Copied" : "Copy to clipboard"}
                  </Button>
                </CardHeader>
                <CardContent className="flex-1 p-6 relative">
                  {generateMutation.isPending ? (
                    <div className="absolute inset-0 flex flex-col items-center justify-center space-y-4 text-muted-foreground">
                      <Sparkles className="w-8 h-8 animate-pulse text-primary" />
                      <p>Crafting your content...</p>
                    </div>
                  ) : generatedContent ? (
                    <div className="whitespace-pre-wrap text-sm leading-relaxed prose dark:prose-invert max-w-none">
                      {generatedContent}
                    </div>
                  ) : (
                    <div className="absolute inset-0 flex flex-col items-center justify-center text-muted-foreground/50">
                      <Sparkles className="w-12 h-12 mb-4 opacity-20" />
                      <p>Your generated content will appear here.</p>
                    </div>
                  )}
                </CardContent>
              </Card>
            </div>
          </div>
        </div>
      </Layout>
    </ProtectedRoute>
  );
}
