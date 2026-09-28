"use client";

import React, { createContext, useContext, useEffect, useMemo, useState } from "react";
import { usePathname, useRouter } from "next/navigation";
import { tenantFetch } from "@/lib/api";

type TenantUser = {
  id: string;
  workspaceId: string;
  workspaceSlug: string;
  email: string;
  name?: string | null;
  roles: string[];
};

type ClientUser = {
  id: string;
  workspaceId: string;
  workspaceSlug: string;
  tenantClientId: string;
  tenantClientName: string;
  email: string;
  name?: string | null;
  roles: string[];
};

type AuthAudience = "tenant" | "client";

type PortalSessionResponse =
  | { audience: "tenant"; user: TenantUser }
  | { audience: "client"; clientUser: ClientUser };

type AuthContextValue = {
  isAuthenticated: boolean;
  isLoading: boolean;
  audience: AuthAudience | null;
  user: TenantUser | null;
  clientUser: ClientUser | null;
  login: (email: string, password: string) => Promise<AuthAudience>;
  logout: () => Promise<void>;
  refresh: () => Promise<void>;
  defaultRoute: string;
};

const AuthContext = createContext<AuthContextValue | null>(null);

async function fetchPortalSession() {
  try {
    return await tenantFetch<PortalSessionResponse>("/api/auth/session", {
      method: "GET",
    });
  } catch {
    return null;
  }
}

export function AuthProvider({ children }: { children: React.ReactNode }) {
  const router = useRouter();
  const [user, setUser] = useState<TenantUser | null>(null);
  const [clientUser, setClientUser] = useState<ClientUser | null>(null);
  const [isLoading, setIsLoading] = useState(true);

  const refresh = async () => {
    try {
      const session = await fetchPortalSession();
      setUser(session?.audience === "tenant" ? session.user : null);
      setClientUser(session?.audience === "client" ? session.clientUser : null);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    void refresh();
  }, []);

  const login = async (email: string, password: string) => {
    setIsLoading(true);

    try {
      const payload = await tenantFetch<PortalSessionResponse>("/api/auth/session/login", {
        method: "POST",
        body: JSON.stringify({ email, password }),
      });

      if (payload.audience === "tenant") {
        setUser(payload.user);
        setClientUser(null);
        return "tenant" as const;
      }

      setClientUser(payload.clientUser);
      setUser(null);
      return "client" as const;
    } finally {
      setIsLoading(false);
    }
  };

  const logout = async () => {
    setIsLoading(true);
    try {
      await tenantFetch<{ message: string }>("/api/auth/session/logout", {
        method: "POST",
      });
    } finally {
      setUser(null);
      setClientUser(null);
      setIsLoading(false);
      router.replace("/login");
    }
  };

  const audience: AuthAudience | null = user ? "tenant" : clientUser ? "client" : null;
  const defaultRoute = audience === "client" ? "/client" : "/";

  const value = useMemo<AuthContextValue>(
    () => ({
      isAuthenticated: !!audience,
      isLoading,
      audience,
      user,
      clientUser,
      login,
      logout,
      refresh,
      defaultRoute,
    }),
    [audience, clientUser, isLoading, user],
  );

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>;
}

export function useAuth() {
  const context = useContext(AuthContext);
  if (!context) {
    throw new Error("useAuth must be used within AuthProvider");
  }

  return context;
}

export function ProtectedRoute({
  children,
  audience = "tenant",
  allowedRoles,
}: {
  children: React.ReactNode;
  audience?: AuthAudience;
  allowedRoles?: string[];
}) {
  const auth = useAuth();
  const router = useRouter();
  const pathname = usePathname();
  const activeRoles = auth.audience === "tenant" ? auth.user?.roles : auth.clientUser?.roles;
  const hasAllowedRole =
    !allowedRoles?.length || allowedRoles.some((role) => activeRoles?.includes(role));

  useEffect(() => {
    if (auth.isLoading) {
      return;
    }

    if (!auth.isAuthenticated && pathname !== "/login") {
      router.replace("/login");
      return;
    }

    if (auth.isAuthenticated && auth.audience !== audience) {
      router.replace(auth.defaultRoute);
      return;
    }

    if (auth.isAuthenticated && auth.audience === audience && !hasAllowedRole) {
      router.replace(auth.defaultRoute);
    }
  }, [audience, auth, hasAllowedRole, pathname, router]);

  if (auth.isLoading || !auth.isAuthenticated || auth.audience !== audience || !hasAllowedRole) {
    return null;
  }

  return <>{children}</>;
}
