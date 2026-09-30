import { useState } from "react";
import { useListVoiceAgents, useCreateVoiceAgent, useListVoiceCalls } from "@workspace/api-client-react";
import { ProtectedRoute } from "@/lib/auth";
import { Layout } from "@/components/layout";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Form, FormControl, FormField, FormItem, FormLabel, FormMessage } from "@/components/ui/form";
import { Textarea } from "@/components/ui/textarea";
import { Skeleton } from "@/components/ui/skeleton";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { zodResolver } from "@hookform/resolvers/zod";
import { Plus, Mic, PhoneCall, Clock, PlayCircle, Settings2, Activity } from "lucide-react";
import { useQueryClient } from "@tanstack/react-query";
import { getListVoiceAgentsQueryKey, getListVoiceCallsQueryKey } from "@workspace/api-client-react";
import { Badge } from "@/components/ui/badge";
import { TENANT_ROUTE_ACCESS } from "@/lib/tenant-access";

const createAgentSchema = z.object({
  name: z.string().min(1, "Name is required"),
  useCase: z.string().min(1, "Use case is required"),
  greetingScript: z.string().min(10, "Greeting script must be meaningful"),
  businessHours: z.string().optional(),
});

export default function Voice() {
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const queryClient = useQueryClient();

  const { data: agents, isLoading: loadingAgents } = useListVoiceAgents();
  const { data: calls, isLoading: loadingCalls } = useListVoiceCalls();
  const createAgent = useCreateVoiceAgent();

  const form = useForm<z.infer<typeof createAgentSchema>>({
    resolver: zodResolver(createAgentSchema),
    defaultValues: {
      name: "",
      useCase: "",
      greetingScript: "Hi, thanks for calling. How can I help you today?",
      businessHours: "9AM - 5PM EST",
    },
  });

  const onSubmit = (values: z.infer<typeof createAgentSchema>) => {
    createAgent.mutate(
      { data: values },
      {
        onSuccess: () => {
          queryClient.invalidateQueries({ queryKey: getListVoiceAgentsQueryKey() });
          setIsDialogOpen(false);
          form.reset();
        },
      }
    );
  };

  const getStatusBadge = (status: string) => {
    switch (status) {
      case "active": return <Badge className="bg-green-500/20 text-green-500 hover:bg-green-500/30">Active</Badge>;
      case "training": return <Badge className="bg-blue-500/20 text-blue-500 hover:bg-blue-500/30">Training</Badge>;
      case "inactive": return <Badge variant="secondary">Inactive</Badge>;
      default: return null;
    }
  };

  const formatDuration = (seconds: number) => {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins}m ${secs}s`;
  };

  return (
    <ProtectedRoute allowedRoles={TENANT_ROUTE_ACCESS.voice}>
      <Layout>
        <div className="p-8 space-y-8 animate-in fade-in duration-500">
          <div className="flex justify-between items-end">
            <div>
              <h1 className="text-3xl font-bold tracking-tight">Voice Agents</h1>
              <p className="text-muted-foreground mt-1">AI conversational agents for inbound and outbound calls.</p>
            </div>
            <Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
              <DialogTrigger asChild>
                <Button>
                  <Plus className="w-4 h-4 mr-2" />
                  Deploy Agent
                </Button>
              </DialogTrigger>
              <DialogContent className="sm:max-w-[500px]">
                <DialogHeader>
                  <DialogTitle>Deploy Voice Agent</DialogTitle>
                  <DialogDescription>
                    Configure a new conversational AI agent.
                  </DialogDescription>
                </DialogHeader>
                <Form {...form}>
                  <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
                    <FormField
                      control={form.control}
                      name="name"
                      render={({ field }) => (
                         <FormItem>
                          <FormLabel>Agent Name</FormLabel>
                          <FormControl>
                            <Input placeholder="Sales Assistant" {...field} />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                    <FormField
                      control={form.control}
                      name="useCase"
                      render={({ field }) => (
                         <FormItem>
                          <FormLabel>Primary Use Case</FormLabel>
                          <FormControl>
                            <Input placeholder="Inbound lead qualification" {...field} />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                    <FormField
                      control={form.control}
                      name="greetingScript"
                      render={({ field }) => (
                         <FormItem>
                          <FormLabel>Greeting Script</FormLabel>
                          <FormControl>
                            <Textarea {...field} />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                    <div className="flex justify-end pt-4">
                      <Button type="submit" loading={createAgent.isPending} loadingLabel="Deploying agent...">
                        Deploy Agent
                      </Button>
                    </div>
                  </form>
                </Form>
              </DialogContent>
            </Dialog>
          </div>

          <div className="grid grid-cols-1 lg:grid-cols-3 gap-8">
            <div className="lg:col-span-2 space-y-6">
              <h2 className="text-xl font-semibold">Active Agents</h2>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                {loadingAgents ? (
                  Array.from({ length: 2 }).map((_, i) => <Skeleton key={i} className="h-48 rounded-xl" />)
                ) : agents && agents.length > 0 ? (
                  agents.map(agent => (
                    <Card key={agent.id} className="overflow-hidden border-border/50 hover:border-primary/30 transition-colors">
                      <div className="bg-muted/30 p-4 border-b border-border flex justify-between items-start">
                        <div className="flex items-center space-x-3">
                          <div className="bg-primary/20 p-2 rounded-full">
                            <Mic className="w-5 h-5 text-primary" />
                          </div>
                          <div>
                            <h3 className="font-bold">{agent.name}</h3>
                            <div className="text-xs text-muted-foreground font-mono mt-1">{agent.phoneNumber || "Pending provisioning"}</div>
                          </div>
                        </div>
                        {getStatusBadge(agent.status)}
                      </div>
                      <CardContent className="p-4 space-y-4">
                        <div className="text-sm">{agent.useCase}</div>
                        <div className="grid grid-cols-3 gap-2 border-t border-border pt-4">
                          <div className="text-center">
                            <div className="text-xl font-bold">{agent.callsHandled}</div>
                            <div className="text-[10px] text-muted-foreground uppercase">Calls</div>
                          </div>
                          <div className="text-center border-x border-border">
                            <div className="text-xl font-bold">{formatDuration(agent.avgCallDuration)}</div>
                            <div className="text-[10px] text-muted-foreground uppercase">Avg Dur</div>
                          </div>
                          <div className="text-center">
                            <div className="text-xl font-bold text-green-500">{agent.satisfactionScore}%</div>
                            <div className="text-[10px] text-muted-foreground uppercase">CSAT</div>
                          </div>
                        </div>
                        <Button variant="outline" className="w-full mt-2" size="sm">
                          <Settings2 className="w-4 h-4 mr-2" /> Configure
                        </Button>
                      </CardContent>
                    </Card>
                  ))
                ) : (
                  <div className="col-span-2 text-center py-12 text-muted-foreground border border-dashed rounded-xl">
                    No agents deployed yet.
                  </div>
                )}
              </div>
            </div>

            <div className="space-y-6">
              <Card className="h-full">
                <CardHeader className="border-b border-border bg-muted/20">
                  <CardTitle className="flex items-center text-lg">
                    <Activity className="w-5 h-5 mr-2 text-primary" />
                    Call Log
                  </CardTitle>
                </CardHeader>
                <CardContent className="p-0">
                  <div className="divide-y divide-border">
                    {loadingCalls ? (
                      Array.from({ length: 5 }).map((_, i) => <div key={i} className="p-4"><Skeleton className="h-12 w-full" /></div>)
                    ) : calls && calls.length > 0 ? (
                      calls.map(call => (
                        <div key={call.id} className="p-4 hover:bg-muted/30 transition-colors">
                          <div className="flex items-center justify-between mb-2">
                            <div className="font-mono text-sm">{call.callerNumber}</div>
                            <div className="text-xs text-muted-foreground flex items-center">
                              <Clock className="w-3 h-3 mr-1" /> {formatDuration(call.duration)}
                            </div>
                          </div>
                          <div className="flex items-center justify-between">
                            <Badge variant="outline" className="text-[10px] capitalize border-primary/20 text-primary">
                              {call.outcome.replace('_', ' ')}
                            </Badge>
                            {call.recordingUrl && (
                              <Button variant="ghost" size="icon" className="h-6 w-6 rounded-full">
                                <PlayCircle className="w-4 h-4 text-muted-foreground" />
                              </Button>
                            )}
                          </div>
                          <div className="text-[10px] text-muted-foreground mt-2 text-right">
                            {new Date(call.timestamp).toLocaleString()}
                          </div>
                        </div>
                      ))
                    ) : (
                       <div className="p-8 text-center text-sm text-muted-foreground">No calls recorded.</div>
                    )}
                  </div>
                </CardContent>
              </Card>
            </div>
          </div>
        </div>
      </Layout>
    </ProtectedRoute>
  );
}
