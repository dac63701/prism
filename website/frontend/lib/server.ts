import { cookies, headers } from "next/headers";
import { redirect } from "next/navigation";
import type { User } from "@/lib/types";

export function apiOrigin() {
  return process.env.API_ORIGIN ?? "";
}

export async function requestOrigin() {
  if (apiOrigin()) {
    return apiOrigin();
  }
  const h = await headers();
  const proto = h.get("x-forwarded-proto") ?? "http";
  const host = h.get("x-forwarded-host") ?? h.get("host") ?? "localhost:3000";
  return `${proto}://${host}`;
}

export async function cookieHeader() {
  return (await cookies()).toString();
}

export async function currentUser() {
  const response = await fetch(`${await requestOrigin()}/api/auth/me`, {
    headers: {
      cookie: await cookieHeader(),
    },
    cache: "no-store",
  });

  if (!response.ok) {
    return null;
  }

  return (await response.json()) as User;
}

export async function requireUser() {
  const user = await currentUser();
  if (!user) {
    redirect("/login");
  }
  return user;
}

export async function requireAdmin() {
  const user = await requireUser();
  if (user.role !== "admin") {
    redirect("/dashboard");
  }
  return user;
}
