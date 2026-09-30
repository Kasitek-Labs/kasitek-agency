"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import DashboardPage from "@/screens/dashboard";
import { useAuth } from "@/lib/auth";

export default function Page() {
  const router = useRouter();
  const { audience, isLoading } = useAuth();

  useEffect(() => {
    if (!isLoading && audience === "client") {
      router.replace("/client");
    }
  }, [audience, isLoading, router]);

  return <DashboardPage />;
}
