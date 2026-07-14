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


export async function login(email: string, password: string) {
  return jsonFetch<AuthResponse>("/api/auth/login", {
    method: "POST",
    body: JSON.stringify({ email, password }),
  });
}

export async function register(email: string, password: string, display_name?: string) {
  return jsonFetch<{ status: string; message: string; email: string }>("/api/auth/register", {
    method: "POST",
    body: JSON.stringify({ email, password, display_name }),
  });
}

export async function getCurrentUser() {
  return jsonFetch<User>("/api/auth/me", { method: "GET" });
}

export async function changePassword(currentPassword: string, newPassword: string) {
  return jsonFetch<{ status: string }>("/api/auth/change-password", {
    method: "POST",
    body: JSON.stringify({ current_password: currentPassword, new_password: newPassword }),
  });
}

export async function verifyEmail(token: string) {
  const response = await fetch(`/api/auth/verify-email?token=${encodeURIComponent(token)}`, {
    credentials: "include",
    redirect: "manual",
  });
  if (response.status >= 400) {
    const text = await response.text();
    throw new Error(text || "Verification failed");
  }
  return response;
}

export async function resendVerification(email: string) {
  return jsonFetch<{ status: string; message: string }>("/api/auth/resend-verification", {
    method: "POST",
    body: JSON.stringify({ email }),
  });
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

export async function deleteClip(id: string) {
  await fetch(`/api/clips/${id}`, {
    method: "DELETE",
    credentials: "include",
  });
}

export async function updateClipVisibility(id: string, visibility: "public" | "private" | "unlisted") {
  return jsonFetch<{ id: string; visibility: string }>(`/api/clips/${id}/visibility`, {
    method: "PATCH",
    body: JSON.stringify({ visibility }),
  });
}

export async function updateClipName(id: string, title: string) {
  return jsonFetch<{ id: string; title: string }>(`/api/clips/${id}/name`, {
    method: "PATCH",
    body: JSON.stringify({ title }),
  });
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

export async function deleteAdminUser(id: string) {
  await fetch(`/api/admin/users/${id}`, {
    method: "DELETE",
    credentials: "include",
  });
}
