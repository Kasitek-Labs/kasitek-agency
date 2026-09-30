import { useState } from "react";
import { useListReviewCampaigns, useCreateReviewCampaign, useGetReviewStats } from "@workspace/api-client-react";
import { ProtectedRoute } from "@/lib/auth";
import { Layout } from "@/components/layout";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Form, FormControl, FormField, FormItem, FormLabel, FormMessage } from "@/components/ui/form";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { zodResolver } from "@hookform/resolvers/zod";
import { Plus, Star, ShieldAlert, TrendingUp } from "lucide-react";
import { useQueryClient } from "@tanstack/react-query";
import { getListReviewCampaignsQueryKey, getGetReviewStatsQueryKey } from "@workspace/api-client-react";
import { Badge } from "@/components/ui/badge";
import { Progress } from "@/components/ui/progress";
import { TENANT_ROUTE_ACCESS } from "@/lib/tenant-access";

const createCampaignSchema = z.object({
  name: z.string().min(1, "Name is required"),
  channel: z.enum(["whatsapp", "email", "sms"]),
  triggerEvent: z.string().min(1, "Trigger is required"),
  positiveRedirectUrl: z.string().url("Must be a valid URL"),
  negativeCapture: z.boolean().default(true),
});

export default function Reviews() {
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const queryClient = useQueryClient();

  const { data: campaigns, isLoading: loadingCampaigns } = useListReviewCampaigns();
  const { data: stats, isLoading: loadingStats } = useGetReviewStats();
  const createCampaign = useCreateReviewCampaign();

  const form = useForm<z.infer<typeof createCampaignSchema>>({
    resolver: zodResolver(createCampaignSchema),
    defaultValues: {
      name: "",
      channel: "email",
      triggerEvent: "purchase_completed",
      positiveRedirectUrl: "https://g.page/r/...",
      negativeCapture: true,
    },
  });

  const onSubmit = (values: z.infer<typeof createCampaignSchema>) => {
    createCampaign.mutate(
      { data: values },
      {
        onSuccess: () => {
          queryClient.invalidateQueries({ queryKey: getListReviewCampaignsQueryKey() });
          queryClient.invalidateQueries({ queryKey: getGetReviewStatsQueryKey() });
          setIsDialogOpen(false);
          form.reset();
        },
      }
    );
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case "active": return "bg-green-500/10 text-green-500 border-green-500/20";
      case "paused": return "bg-yellow-500/10 text-yellow-500 border-yellow-500/20";
      case "completed": return "bg-blue-500/10 text-blue-500 border-blue-500/20";
      default: return "";
    }
  };

  return (
    <ProtectedRoute allowedRoles={TENANT_ROUTE_ACCESS.reviews}>
      <Layout>
        <div className="p-8 space-y-8 animate-in fade-in duration-500">
          <div className="flex justify-between items-end">
            <div>
              <h1 className="text-3xl font-bold tracking-tight">Reputation Management</h1>
              <p className="text-muted-foreground mt-1">Automate review generation and intercept negative feedback.</p>
            </div>
            <Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
              <DialogTrigger asChild>
                <Button>
                  <Plus className="w-4 h-4 mr-2" />
                  New Campaign
                </Button>
              </DialogTrigger>
              <DialogContent className="sm:max-w-[425px]">
                <DialogHeader>
                  <DialogTitle>Create Review Campaign</DialogTitle>
                  <DialogDescription>
                    Set up automated review requests.
                  </DialogDescription>
                </DialogHeader>
                <Form {...form}>
                  <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
                    <FormField
                      control={form.control}
                      name="name"
                      render={({ field }) => (
                         <FormItem>
                          <FormLabel>Campaign Name</FormLabel>
                          <FormControl>
                            <Input placeholder="Post-Purchase Flow" {...field} />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                    <FormField
                      control={form.control}
                      name="channel"
                      render={({ field }) => (
                         <FormItem>
                          <FormLabel>Delivery Channel</FormLabel>
                          <Select onValueChange={field.onChange} defaultValue={field.value}>
                            <FormControl>
                              <SelectTrigger>
                                <SelectValue />
                              </SelectTrigger>
                            </FormControl>
                            <SelectContent>
                              <SelectItem value="email">Email</SelectItem>
                              <SelectItem value="whatsapp">WhatsApp</SelectItem>
                              <SelectItem value="sms">SMS</SelectItem>
                            </SelectContent>
                          </Select>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                    <FormField
                      control={form.control}
                      name="positiveRedirectUrl"
                      render={({ field }) => (
                         <FormItem>
                          <FormLabel>Google Review Link</FormLabel>
                          <FormControl>
                            <Input placeholder="https://g.page/r/..." {...field} />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                    <FormField
                      control={form.control}
                      name="negativeCapture"
                      render={({ field }) => (
                         <FormItem className="flex flex-row items-center justify-between rounded-lg border border-border p-4">
                          <div className="space-y-0.5">
                            <FormLabel className="text-base">Negative Intercept</FormLabel>
                            <div className="text-sm text-muted-foreground">
                              Route 1-3 star reviews to internal form
                            </div>
                          </div>
                          <FormControl>
                            <Switch checked={field.value} onCheckedChange={field.onChange} />
                          </FormControl>
                        </FormItem>
                      )}
                    />
                    <div className="flex justify-end pt-4">
                      <Button type="submit" loading={createCampaign.isPending} loadingLabel="Launching campaign...">
                        Launch Campaign
                      </Button>
                    </div>
                  </form>
                </Form>
              </DialogContent>
            </Dialog>
          </div>

          {/* Stats Overview */}
          {loadingStats ? (
            <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
              {Array.from({ length: 4 }).map((_, i) => <Skeleton key={i} className="h-32 w-full" />)}
            </div>
          ) : stats && (
            <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
              <Card className="bg-card">
                <CardContent className="p-6">
                  <div className="flex items-center justify-between mb-4">
                    <div className="text-sm font-medium text-muted-foreground">Average Rating</div>
                    <Star className="w-5 h-5 text-yellow-500 fill-yellow-500" />
                  </div>
                  <div className="text-4xl font-bold">{stats.avgRating.toFixed(1)}</div>
                  <div className="text-xs text-muted-foreground mt-2">Across {stats.totalReviews} total reviews</div>
                </CardContent>
              </Card>
              <Card>
                <CardContent className="p-6">
                  <div className="flex items-center justify-between mb-4">
                    <div className="text-sm font-medium text-muted-foreground">Positive Sentiment</div>
                    <TrendingUp className="w-5 h-5 text-green-500" />
                  </div>
                  <div className="text-4xl font-bold text-green-500">{stats.positiveRate}%</div>
                  <Progress value={stats.positiveRate} className="h-1 mt-3" />
                </CardContent>
              </Card>
              <Card>
                <CardContent className="p-6">
                  <div className="flex items-center justify-between mb-4">
                    <div className="text-sm font-medium text-muted-foreground">Google Reviews Generated</div>
                    <div className="font-bold text-lg text-primary">G</div>
                  </div>
                  <div className="text-4xl font-bold">{stats.googleReviews}</div>
                </CardContent>
              </Card>
              <Card className="border-red-500/20 bg-red-500/5">
                <CardContent className="p-6">
                  <div className="flex items-center justify-between mb-4">
                    <div className="text-sm font-medium text-red-500/80">Crises Averted</div>
                    <ShieldAlert className="w-5 h-5 text-red-500" />
                  </div>
                  <div className="text-4xl font-bold text-red-500">{stats.negativesCaptured}</div>
                  <div className="text-xs text-red-500/60 mt-2">Intercepted privately</div>
                </CardContent>
              </Card>
            </div>
          )}

          <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
            <div className="lg:col-span-2">
              <Card>
                <CardHeader>
                  <CardTitle>Active Campaigns</CardTitle>
                </CardHeader>
                <CardContent>
                  {loadingCampaigns ? (
                    <div className="space-y-4">
                      {Array.from({ length: 3 }).map((_, i) => <Skeleton key={i} className="h-20 w-full" />)}
                    </div>
                  ) : campaigns && campaigns.length > 0 ? (
                    <div className="space-y-4">
                      {campaigns.map((campaign) => (
                        <div key={campaign.id} className="flex items-center justify-between p-4 border border-border rounded-lg hover:bg-muted/30">
                          <div>
                            <div className="flex items-center gap-3 mb-1">
                              <h4 className="font-semibold">{campaign.name}</h4>
                              <Badge variant="outline" className={`capitalize ${getStatusColor(campaign.status)} text-[10px]`}>
                                {campaign.status}
                              </Badge>
                              <Badge variant="secondary" className="capitalize text-[10px]">
                                {campaign.channel}
                              </Badge>
                            </div>
                            <div className="text-sm text-muted-foreground flex gap-4">
                              <span>Sent: <strong className="text-foreground">{campaign.reviewsSent}</strong></span>
                              <span>Google: <strong className="text-primary">{campaign.googleReviewsGenerated}</strong></span>
                            </div>
                          </div>
                          <div className="text-right">
                            <div className="text-2xl font-bold text-green-500">{campaign.positiveRate}%</div>
                            <div className="text-[10px] text-muted-foreground uppercase">Conversion</div>
                          </div>
                        </div>
                      ))}
                    </div>
                  ) : (
                    <div className="text-center py-8 text-muted-foreground">No active campaigns</div>
                  )}
                </CardContent>
              </Card>
            </div>

            <div>
              <Card>
                <CardHeader>
                  <CardTitle>Rating Distribution</CardTitle>
                </CardHeader>
                <CardContent>
                  {loadingStats ? (
                    <Skeleton className="h-[200px] w-full" />
                  ) : stats && stats.ratingDistribution ? (
                    <div className="space-y-4">
                      {stats.ratingDistribution.sort((a,b) => b.stars - a.stars).map((dist) => {
                        const percentage = (dist.count / stats.totalReviews) * 100;
                        return (
                          <div key={dist.stars} className="flex items-center gap-3">
                            <div className="flex items-center gap-1 w-12 text-sm font-medium">
                              {dist.stars} <Star className="w-3 h-3 fill-yellow-500 text-yellow-500" />
                            </div>
                            <div className="flex-1 h-2 bg-muted rounded-full overflow-hidden">
                              <div
                                className={`h-full rounded-full ${dist.stars >= 4 ? 'bg-green-500' : dist.stars === 3 ? 'bg-yellow-500' : 'bg-red-500'}`}
                                style={{ width: `${percentage}%` }}
                              />
                            </div>
                            <div className="w-8 text-right text-xs text-muted-foreground">{dist.count}</div>
                          </div>
                        )
                      })}
                    </div>
                  ) : null}
                </CardContent>
              </Card>
            </div>
          </div>
        </div>
      </Layout>
    </ProtectedRoute>
  );
}
