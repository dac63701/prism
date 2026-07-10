import { cookieHeader, requestOrigin } from "@/lib/server";
import type { AdminUserDetail, AdminUserRow, ClipDetail, ClipListItem, DashboardStats, User } from "@/lib/types";

async function readJson<T>(response: Response): Promise<T> {
  if (!response.ok) {
    const text = await response.text();
    throw new Error(text || response.statusText);
  }

  return (await response.json()) as T;
}

async function serverFetch<T>(path: string, init?: RequestInit) {
  return readJson<T>(
    await fetch(`${await requestOrigin()}${path}`, {
      ...init,
      headers: {
        ...(init?.headers ?? {}),
        cookie: await cookieHeader(),
      },
      cache: "no-store",
    })
  );
}

export function getDashboardStats() {
  return serverFetch<DashboardStats>("/api/admin/stats", { method: "GET" });
}

export function listClips() {
  return serverFetch<{ clips: ClipListItem[]; total: number; page: number; per_page: number; total_pages: number }>(
    "/api/clips",
    { method: "GET" }
  );
}

export function getShareMeta(shareId: string) {
  return serverFetch<{ clip: ClipDetail; user: User }>(`/api/s/${shareId}/meta`, { method: "GET" });
}

export function getProfile(username: string) {
  return serverFetch<{ user: User; clips: ClipListItem[] }>(`/api/u/${username}`, { method: "GET" });
}

export function getClip(id: string) {
  return serverFetch<ClipDetail>(`/api/clips/${id}`, { method: "GET" });
}

export function listAdminUsers(search = "") {
  const params = new URLSearchParams();
  if (search) params.set("search", search);
  return serverFetch<{ users: AdminUserRow[]; total: number; page: number; per_page: number; total_pages: number }>(
    `/api/admin/users?${params.toString()}`,
    { method: "GET" }
  );
}

export function getAdminUser(id: string) {
  return serverFetch<AdminUserDetail>(`/api/admin/users/${id}`, { method: "GET" });
}
