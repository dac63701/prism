import type {
  AuthResponse,
  AdminUserRow,
  ClipDetail,
  ClipListItem,
  DashboardStats,
  User,
} from "@/lib/types";

type JsonRecord = Record<string, unknown>;

async function readJson<T>(response: Response): Promise<T> {
  if (!response.ok) {
    const text = await response.text();
    throw new Error(text || response.statusText);
  }

  return (await response.json()) as T;
}

async function jsonFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    ...init,
    credentials: "include",
    headers: {
      ...(init?.headers ?? {}),
      "Content-Type": "application/json",
    },
  });
  return readJson<T>(response);
}

export function googleLoginUrl(next = "/dashboard", desktop = false) {
  const params = new URLSearchParams();
  params.set("next", next);
  if (desktop) {
    params.set("desktop", "true");
  }
  return `/api/auth/google?${params.toString()}`;
}

export function desktopLoginUrl() {
  return googleLoginUrl("/dashboard", true);
}

export async function login(email: string, password: string) {
  return jsonFetch<AuthResponse>("/api/auth/login", {
    method: "POST",
    body: JSON.stringify({ email, password }),
  });
}

export async function register(email: string, password: string, display_name?: string) {
  return jsonFetch<AuthResponse>("/api/auth/register", {
    method: "POST",
    body: JSON.stringify({ email, password, display_name }),
  });
}

export async function logout() {
  return jsonFetch<{ status: string }>("/api/auth/logout", {
    method: "POST",
  });
}

export async function refresh(refresh_token?: string) {
  return jsonFetch<AuthResponse>("/api/auth/refresh", {
    method: "POST",
    body: JSON.stringify({ refresh_token }),
  });
}

export async function getMe() {
  return jsonFetch<User>("/api/auth/me", { method: "GET" });
}

export async function getDashboardStats() {
  return jsonFetch<DashboardStats>("/api/admin/stats", { method: "GET" });
}

export async function listClips(params?: URLSearchParams) {
  const query = params?.toString() ? `?${params.toString()}` : "";
  return jsonFetch<{ clips: ClipListItem[]; total: number; page: number; per_page: number; total_pages: number }>(
    `/api/clips${query}`,
    { method: "GET" }
  );
}

export async function getClip(id: string) {
  return jsonFetch<ClipDetail>(`/api/clips/${id}`, { method: "GET" });
}

export async function updateClip(id: string, body: JsonRecord) {
  return jsonFetch<{ status: string }>(`/api/clips/${id}`, {
    method: "PATCH",
    body: JSON.stringify(body),
  });
}

export async function deleteClip(id: string) {
  const response = await fetch(`/api/clips/${id}`, {
    method: "DELETE",
    credentials: "include",
  });

  if (!response.ok) {
    const text = await response.text();
    throw new Error(text || response.statusText);
  }
}

export async function getShareMeta(shareId: string) {
  return jsonFetch<{ clip: ClipDetail; user: User }>(`/api/s/${shareId}/meta`, { method: "GET" });
}

export async function getProfile(username: string) {
  return jsonFetch<{ user: User; clips: ClipListItem[] }>(`/api/u/${username}`, { method: "GET" });
}

export async function listAdminUsers(search = "") {
  const params = new URLSearchParams();
  if (search) params.set("search", search);
  return jsonFetch<{ users: AdminUserRow[]; total: number; page: number; per_page: number; total_pages: number }>(
    `/api/admin/users?${params.toString()}`,
    { method: "GET" }
  );
}
