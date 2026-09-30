import { useState } from "react";
import { useListNurturingSequences, useCreateNurturingSequence, useGetNurturingStats } from "@workspace/api-client-react";
import { ProtectedRoute } from "@/lib/auth";
import { Layout } from "@/components/layout";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogTrigger } from "@/components/ui/dialog";
import { Form, FormControl, FormField, FormItem, FormLabel, FormMessage } from "@/components/ui/form";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { zodResolver } from "@hookform/resolvers/zod";
import { Plus, Mail, MessageCircle, GitBranch, ArrowRight, Play, Pause } from "lucide-react";
import { useQueryClient } from "@tanstack/react-query";
import { getListNurturingSequencesQueryKey, getGetNurturingStatsQueryKey } from "@workspace/api-client-react";
import { Badge } from "@/components/ui/badge";
import { TENANT_ROUTE_ACCESS } from "@/lib/tenant-access";

const createSequenceSchema = z.object({
  name: z.string().min(1, "Name is required"),
  channel: z.enum(["whatsapp", "email", "both"]),
  triggerEvent: z.string().min(1, "Trigger event is required"),
});

export default function Nurturing() {
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const queryClient = useQueryClient();

  const { data: sequences, isLoading: loadingSequences } = useListNurturingSequences();
  const { data: stats, isLoading: loadingStats } = useGetNurturingStats();
  const createSequence = useCreateNurturingSequence();

  const form = useForm<z.infer<typeof createSequenceSchema>>({
    resolver: zodResolver(createSequenceSchema),
    defaultValues: {
      name: "",
      channel: "email",
      triggerEvent: "lead_created",
    },
  });

  const onSubmit = (values: z.infer<typeof createSequenceSchema>) => {
    createSequence.mutate(
      { data: values },
      {
        onSuccess: () => {
          queryClient.invalidateQueries({ queryKey: getListNurturingSequencesQueryKey() });
          queryClient.invalidateQueries({ queryKey: getGetNurturingStatsQueryKey() });
          setIsDialogOpen(false);
          form.reset();
        },
      }
    );
  };

  const getChannelIcon = (channel: string) => {
    if (channel === "whatsapp") return <MessageCircle className="w-4 h-4" />;
    if (channel === "email") return <Mail className="w-4 h-4" />;
    return <GitBranch className="w-4 h-4" />;
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case "active": return "bg-green-500/10 text-green-500 border-green-500/20";
      case "paused": return "bg-yellow-500/10 text-yellow-500 border-yellow-500/20";
      case "draft": return "bg-gray-500/10 text-gray-500 border-gray-500/20";
      default: return "";
    }
  };

  return (
    <ProtectedRoute allowedRoles={TENANT_ROUTE_ACCESS.nurturing}>
      <Layout>
        <div className="p-8 space-y-8 animate-in fade-in duration-500">
          <div className="flex justify-between items-end">
            <div>
              <h1 className="text-3xl font-bold tracking-tight">Lead Nurturing</h1>
              <p className="text-muted-foreground mt-1">Automated sequences to convert leads into clients.</p>
            </div>
            <Dialog open={isDialogOpen} onOpenChange={setIsDialogOpen}>
              <DialogTrigger asChild>
                <Button>
                  <Plus className="w-4 h-4 mr-2" />
                  New Sequence
                </Button>
              </DialogTrigger>
              <DialogContent className="sm:max-w-[425px]">
                <DialogHeader>
                  <DialogTitle>Create Nurturing Sequence</DialogTitle>
                  <DialogDescription>
                    Set up an automated communication flow.
                  </DialogDescription>
                </DialogHeader>
                <Form {...form}>
                  <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-4">
                    <FormField
                      control={form.control}
                      name="name"
                      render={({ field }) => (
                         <FormItem>
                          <FormLabel>Sequence Name</FormLabel>
                          <FormControl>
                            <Input placeholder="Welcome Series" {...field} />
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
                          <FormLabel>Channel</FormLabel>
                          <Select onValueChange={field.onChange} defaultValue={field.value}>
                            <FormControl>
                              <SelectTrigger>
                                <SelectValue placeholder="Select channel" />
                              </SelectTrigger>
                            </FormControl>
                            <SelectContent>
                              <SelectItem value="email">Email</SelectItem>
                              <SelectItem value="whatsapp">WhatsApp</SelectItem>
                              <SelectItem value="both">Omnichannel (Both)</SelectItem>
                            </SelectContent>
                          </Select>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                    <FormField
                      control={form.control}
                      name="triggerEvent"
                      render={({ field }) => (
                         <FormItem>
                          <FormLabel>Trigger Event</FormLabel>
                          <Select onValueChange={field.onChange} defaultValue={field.value}>
                            <FormControl>
                              <SelectTrigger>
                                <SelectValue placeholder="Select trigger" />
                              </SelectTrigger>
                            </FormControl>
                            <SelectContent>
                              <SelectItem value="lead_created">New Lead Captured</SelectItem>
                              <SelectItem value="tag_added">Tag Added</SelectItem>
                              <SelectItem value="form_submitted">Form Submitted</SelectItem>
                              <SelectItem value="manual">Manual Enrollment</SelectItem>
                            </SelectContent>
                          </Select>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                    <div className="flex justify-end pt-4">
                      <Button type="submit" loading={createSequence.isPending} loadingLabel="Creating sequence...">
                        Create Sequence
                      </Button>
                    </div>
                  </form>
                </Form>
              </DialogContent>
            </Dialog>
          </div>

          {/* Stats Overview */}
          {loadingStats ? (
            <div className="grid grid-cols-1 md:grid-cols-5 gap-4">
              {Array.from({ length: 5 }).map((_, i) => <Skeleton key={i} className="h-24 w-full" />)}
            </div>
          ) : stats && (
            <div className="grid grid-cols-1 md:grid-cols-5 gap-4">
              <Card>
                <CardContent className="p-6">
                  <div className="text-sm font-medium text-muted-foreground mb-2">Total Enrolled</div>
                  <div className="text-3xl font-bold">{stats.totalEnrolled.toLocaleString()}</div>
                </CardContent>
              </Card>
              <Card>
                <CardContent className="p-6">
                  <div className="text-sm font-medium text-muted-foreground mb-2">Active Sequences</div>
                  <div className="text-3xl font-bold text-primary">{stats.activeSequences}</div>
                </CardContent>
              </Card>
              <Card>
                <CardContent className="p-6">
                  <div className="text-sm font-medium text-muted-foreground mb-2">Avg Open Rate</div>
                  <div className="text-3xl font-bold">{stats.avgOpenRate}%</div>
                </CardContent>
              </Card>
              <Card>
                <CardContent className="p-6">
                  <div className="text-sm font-medium text-muted-foreground mb-2">Avg Response Rate</div>
                  <div className="text-3xl font-bold">{stats.avgResponseRate}%</div>
                </CardContent>
              </Card>
              <Card className="bg-primary text-primary-foreground border-primary">
                <CardContent className="p-6">
                  <div className="text-sm font-medium text-primary-foreground/80 mb-2">Conversions</div>
                  <div className="text-3xl font-bold">{stats.conversionsThisMonth}</div>
                </CardContent>
              </Card>
            </div>
          )}

          <Card>
            <CardHeader>
              <CardTitle>Sequences</CardTitle>
            </CardHeader>
            <CardContent>
              {loadingSequences ? (
                <div className="space-y-4">
                  {Array.from({ length: 4 }).map((_, i) => (
                    <Skeleton key={i} className="h-16 w-full" />
                  ))}
                </div>
              ) : (
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Sequence Name</TableHead>
                      <TableHead>Status</TableHead>
                      <TableHead>Steps</TableHead>
                      <TableHead>Enrolled</TableHead>
                      <TableHead>Performance</TableHead>
                      <TableHead className="text-right">Actions</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {sequences && sequences.length > 0 ? (
                      sequences.map((seq) => (
                        <TableRow key={seq.id}>
                          <TableCell>
                            <div className="flex items-center space-x-3">
                              <div className="p-2 bg-muted rounded-md text-muted-foreground">
                                {getChannelIcon(seq.channel)}
                              </div>
                              <span className="font-medium">{seq.name}</span>
                            </div>
                          </TableCell>
                          <TableCell>
                            <Badge variant="outline" className={`capitalize ${getStatusColor(seq.status)}`}>
                              {seq.status}
                            </Badge>
                          </TableCell>
                          <TableCell>
                            <div className="flex items-center text-sm text-muted-foreground">
                              {seq.steps} steps <ArrowRight className="w-3 h-3 ml-1" />
                            </div>
                          </TableCell>
                          <TableCell className="font-medium">{seq.leadsEnrolled.toLocaleString()}</TableCell>
                          <TableCell>
                            <div className="flex items-center space-x-4 text-sm">
                              <div>
                                <span className="text-muted-foreground">Open: </span>
                                <span className="font-medium">{seq.openRate}%</span>
                              </div>
                              <div>
                                <span className="text-muted-foreground">Reply: </span>
                                <span className="font-medium">{seq.responseRate}%</span>
                              </div>
                            </div>
                          </TableCell>
                          <TableCell className="text-right">
                            <div className="flex justify-end space-x-2">
                              {seq.status === 'active' ? (
                                <Button variant="ghost" size="icon" className="h-8 w-8 text-yellow-500 hover:text-yellow-600 hover:bg-yellow-500/10">
                                  <Pause className="h-4 w-4" />
                                </Button>
                              ) : (
                                <Button variant="ghost" size="icon" className="h-8 w-8 text-green-500 hover:text-green-600 hover:bg-green-500/10">
                                  <Play className="h-4 w-4" />
                                </Button>
                              )}
                              <Button variant="outline" size="sm">Edit</Button>
                            </div>
                          </TableCell>
                        </TableRow>
                      ))
                    ) : (
                      <TableRow>
                        <TableCell colSpan={6} className="h-24 text-center text-muted-foreground">
                          No sequences found. Create one to get started.
                        </TableCell>
                      </TableRow>
                    )}
                  </TableBody>
                </Table>
              )}
            </CardContent>
          </Card>
        </div>
      </Layout>
    </ProtectedRoute>
  );
}
