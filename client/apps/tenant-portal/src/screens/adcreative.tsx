import { useState } from "react";
import { useListAdCampaigns, useCreateAdCampaign, useListAdCreatives } from "@workspace/api-client-react";
import { ProtectedRoute } from "@/lib/auth";
import { Layout } from "@/components/layout";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Form, FormControl, FormField, FormItem, FormLabel, FormMessage } from "@/components/ui/form";
import { Textarea } from "@/components/ui/textarea";
import { Skeleton } from "@/components/ui/skeleton";
import { Checkbox } from "@/components/ui/checkbox";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { zodResolver } from "@hookform/resolvers/zod";
import { Plus, Megaphone, Image as ImageIcon, Layers, Video, ArrowRight } from "lucide-react";
import { useQueryClient } from "@tanstack/react-query";
import { getListAdCampaignsQueryKey, getListAdCreativesQueryKey } from "@workspace/api-client-react";
import { Badge } from "@/components/ui/badge";
import { TENANT_ROUTE_ACCESS } from "@/lib/tenant-access";

const createAdCampaignSchema = z.object({
  name: z.string().min(1, "Campaign name is required"),
  goal: z.string().min(1, "Goal is required"),
  brandAssets: z.string().min(10, "Describe brand assets/guidelines"),
  platforms: z.array(z.enum(["facebook", "instagram", "google", "tiktok", "linkedin"])).min(1),
  budget: z.string().transform(v => Number(v) || 0).optional(),
});

export default function AdCreative() {
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const [selectedCampaignId, setSelectedCampaignId] = useState<string | undefined>(undefined);
  const queryClient = useQueryClient();

  const { data: campaigns, isLoading: loadingCampaigns } = useListAdCampaigns();
  const { data: creatives, isLoading: loadingCreatives } = useListAdCreatives(
    { campaignId: selectedCampaignId },
    { query: { enabled: true } } // Always enabled, fetches all if undefined
  );

  const createCampaign = useCreateAdCampaign();

  const form = useForm<z.infer<typeof createAdCampaignSchema>>({
    resolver: zodResolver(createAdCampaignSchema),
    defaultValues: {
      name: "",
      goal: "",
      brandAssets: "",
      platforms: ["facebook", "instagram"],
    },
  });

  const onSubmit = (values: z.infer<typeof createAdCampaignSchema>) => {
    createCampaign.mutate(
      { data: values },
      {
        onSuccess: () => {
          queryClient.invalidateQueries({ queryKey: getListAdCampaignsQueryKey() });
          setIsDialogOpen(false);
          form.reset();
        },
      }
    );
  };

  const platformsList = [
    { id: "facebook", label: "Facebook" },
    { id: "instagram", label: "Instagram" },
    { id: "google", label: "Google Ads" },
    { id: "tiktok", label: "TikTok" },
    { id: "linkedin", label: "LinkedIn" },
  ] as const;

  const getFormatIcon = (format: string) => {
    switch (format) {
      case "image": return <ImageIcon className="w-4 h-4 text-blue-500" />;
      case "video": return <Video className="w-4 h-4 text-purple-500" />;
      case "carousel": return <Layers className="w-4 h-4 text-green-500" />;
      default: return <ImageIcon className="w-4 h-4" />;
    }
  };

  return (
    <ProtectedRoute allowedRoles={TENANT_ROUTE_ACCESS.adcreative}>
      <Layout>
        <div className="p-8 space-y-8 animate-in fade-in duration-500">
          <div className="flex justify-between items-end">
            <div>
              <h1 className="text-3xl font-bold tracking-tight">Ad Creatives</h1>
              <p className="text-muted-foreground mt-1">Generate high-converting multi-platform ad variations.</p>
            </div>
            <Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
              <DialogTrigger asChild>
                <Button>
                  <Plus className="w-4 h-4 mr-2" />
                  New Campaign Build
                </Button>
              </DialogTrigger>
              <DialogContent className="sm:max-w-[500px]">
                <DialogHeader>
                  <DialogTitle>Generate Campaign Creatives</DialogTitle>
                  <DialogDescription>
                    AI will generate copy and visual direction for multiple platforms.
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
                            <Input placeholder="Q4 Black Friday Sale" {...field} />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                    <FormField
                      control={form.control}
                      name="goal"
                      render={({ field }) => (
                         <FormItem>
                          <FormLabel>Primary Goal</FormLabel>
                          <FormControl>
                            <Input placeholder="Drive e-commerce conversions" {...field} />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                    <FormField
                      control={form.control}
                      name="brandAssets"
                      render={({ field }) => (
                         <FormItem>
                          <FormLabel>Brand Guidelines & Offer</FormLabel>
                          <FormControl>
                            <Textarea placeholder="We are offering 30% off. Brand voice is energetic and urgent..." {...field} />
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
                          <div className="mb-2">
                            <FormLabel>Target Platforms</FormLabel>
                          </div>
                          <div className="grid grid-cols-2 gap-2">
                            {platformsList.map((item) => (
                              <FormField
                                key={item.id}
                                control={form.control}
                                name="platforms"
                                render={({ field }) => {
                                  return (
                                    <FormItem
                                      key={item.id}
                                      className="flex flex-row items-center space-x-3 space-y-0"
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
                                      <FormLabel className="font-normal cursor-pointer text-sm">
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
                      <Button type="submit" loading={createCampaign.isPending} loadingLabel="Generating creatives...">
                        Generate Creatives
                      </Button>
                    </div>
                  </form>
                </Form>
              </DialogContent>
            </Dialog>
          </div>

          <div className="grid grid-cols-1 lg:grid-cols-4 gap-8">
            <div className="lg:col-span-1 space-y-4">
               <h3 className="font-medium text-sm text-muted-foreground px-1">Campaigns</h3>
               <div className="space-y-2">
                  <div
                    onClick={() => setSelectedCampaignId(undefined)}
                    className={`p-3 rounded-lg border cursor-pointer transition-all ${
                      selectedCampaignId === undefined
                        ? 'bg-primary/10 border-primary text-primary'
                        : 'bg-card border-border hover:border-primary/50'
                    }`}
                  >
                    <div className="font-medium">All Campaigns</div>
                  </div>
                 {loadingCampaigns ? (
                   Array.from({ length: 3 }).map((_, i) => <Skeleton key={i} className="h-16 w-full" />)
                 ) : campaigns && campaigns.map(c => (
                    <div
                      key={c.id}
                      onClick={() => setSelectedCampaignId(c.id)}
                      className={`p-3 rounded-lg border cursor-pointer transition-all ${
                        selectedCampaignId === c.id
                          ? 'bg-muted border-primary shadow-sm'
                          : 'bg-card border-border hover:border-primary/50'
                      }`}
                    >
                      <div className="flex justify-between items-start mb-1">
                        <div className="font-semibold text-sm line-clamp-1">{c.name}</div>
                        {c.status === 'generating' && <div className="w-2 h-2 rounded-full bg-yellow-500 animate-pulse" />}
                      </div>
                      <div className="text-xs text-muted-foreground flex items-center justify-between">
                         <span className="capitalize">{c.status}</span>
                         <span>{c.creativesCount} assets</span>
                      </div>
                    </div>
                 ))}
               </div>
            </div>

            <div className="lg:col-span-3">
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                {loadingCreatives ? (
                   Array.from({ length: 4 }).map((_, i) => <Skeleton key={i} className="h-[300px] w-full" />)
                ) : creatives && creatives.length > 0 ? (
                   creatives.map(creative => (
                     <Card key={creative.id} className="overflow-hidden flex flex-col group">
                       <div className="bg-muted p-3 border-b border-border flex items-center justify-between">
                         <div className="flex items-center gap-2">
                           <Badge variant="secondary" className="capitalize text-[10px] bg-background">
                             {creative.platform}
                           </Badge>
                           <div className="flex items-center gap-1 text-xs text-muted-foreground font-medium uppercase tracking-wider">
                             {getFormatIcon(creative.format)}
                             {creative.format}
                           </div>
                         </div>
                         <Badge variant="outline" className={`capitalize text-[10px] ${creative.status === 'approved' ? 'border-green-500 text-green-500' : ''}`}>
                           {creative.status}
                         </Badge>
                       </div>
                       <CardContent className="p-5 flex-1 flex flex-col">
                         <h4 className="font-bold text-lg mb-3 line-clamp-2 leading-tight">{creative.headline}</h4>
                         <p className="text-sm text-muted-foreground mb-6 flex-1 whitespace-pre-wrap">
                           {creative.bodyText}
                         </p>
                         <div className="mt-auto">
                            <Button variant="secondary" className="w-full justify-between" size="sm">
                              {creative.callToAction} <ArrowRight className="w-4 h-4 ml-2 opacity-50" />
                            </Button>
                         </div>
                       </CardContent>
                     </Card>
                   ))
                ) : (
                  <div className="col-span-2 text-center py-20 text-muted-foreground border border-dashed rounded-xl bg-card">
                    <Megaphone className="w-12 h-12 mx-auto mb-4 opacity-20" />
                    <p>No creatives found. Generate a campaign first.</p>
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>
      </Layout>
    </ProtectedRoute>
  );
}
