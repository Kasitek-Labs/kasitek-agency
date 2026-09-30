import { NextResponse } from "next/server";

const TENANT_BACKEND_URL = (
  process.env.TENANT_BACKEND_URL ||
  process.env.NEXT_PUBLIC_TENANT_API_BASE_URL ||
  "http://localhost:8090"
).replace(/\/$/, "");

function buildTargetUrl(req: Request, pathSegments: string[]) {
  const url = new URL(req.url);
  return `${TENANT_BACKEND_URL}/${pathSegments.map(encodeURIComponent).join("/")}${url.search}`;
}

function buildForwardHeaders(req: Request) {
  const headers = new Headers();

  const contentType = req.headers.get("content-type");
  if (contentType) {
    headers.set("content-type", contentType);
  }

  const accept = req.headers.get("accept");
  if (accept) {
    headers.set("accept", accept);
  }

  const cookie = req.headers.get("cookie");
  if (cookie) {
    headers.set("cookie", cookie);
  }

  const host = req.headers.get("host");
  if (host) {
    headers.set("x-kasitek-tenant-host", host);
    headers.set("x-forwarded-host", host);
    headers.set("x-forwarded-proto", new URL(req.url).protocol.replace(":", ""));
  }

  return headers;
}

async function proxy(req: Request, pathSegments: string[]) {
  const method = req.method.toUpperCase();
  const target = buildTargetUrl(req, pathSegments);

  const upstream = await fetch(target, {
    method,
    headers: buildForwardHeaders(req),
    body: method === "GET" || method === "HEAD" ? undefined : await req.text(),
    cache: "no-store",
  });

  const response = new NextResponse(upstream.body, {
    status: upstream.status,
    headers: upstream.headers,
  });

  const setCookie = upstream.headers.get("set-cookie");
  if (setCookie) {
    response.headers.set("set-cookie", setCookie);
  }

  return response;
}

export async function GET(
  req: Request,
  { params }: { params: Promise<{ path: string[] }> },
) {
  return proxy(req, (await params).path);
}

export async function POST(
  req: Request,
  { params }: { params: Promise<{ path: string[] }> },
) {
  return proxy(req, (await params).path);
}

export async function PUT(
  req: Request,
  { params }: { params: Promise<{ path: string[] }> },
) {
  return proxy(req, (await params).path);
}

export async function PATCH(
  req: Request,
  { params }: { params: Promise<{ path: string[] }> },
) {
  return proxy(req, (await params).path);
}

export async function DELETE(
  req: Request,
  { params }: { params: Promise<{ path: string[] }> },
) {
  return proxy(req, (await params).path);
}

export async function OPTIONS(
  req: Request,
  { params }: { params: Promise<{ path: string[] }> },
) {
  return proxy(req, (await params).path);
}
