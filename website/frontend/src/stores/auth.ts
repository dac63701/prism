import { create } from "zustand";
import { api } from "@/lib/api";

interface User {
  id: string;
  email: string;
  display_name: string;
  role: string;
  storage_used_bytes: number;
  max_storage_bytes: number;
  created_at: string;
}

interface AuthState {
  user: User | null;
  loading: boolean;
  initialized: boolean;
  login: (email: string, password: string) => Promise<void>;
  register: (email: string, password: string, displayName?: string) => Promise<void>;
  logout: () => void;
  loadUser: () => Promise<void>;
}

export const useAuthStore = create<AuthState>((set) => ({
  user: null,
  loading: false,
  initialized: false,

  login: async (email: string, password: string) => {
    const data = await api.post<{
      user: User;
      access_token: string;
      refresh_token: string;
    }>("/api/auth/login", { email, password });

    localStorage.setItem("access_token", data.access_token);
    localStorage.setItem("refresh_token", data.refresh_token);
    set({ user: data.user });
  },

  register: async (email: string, password: string, displayName?: string) => {
    const data = await api.post<{
      user: User;
      access_token: string;
      refresh_token: string;
    }>("/api/auth/register", {
      email,
      password,
      display_name: displayName,
    });

    localStorage.setItem("access_token", data.access_token);
    localStorage.setItem("refresh_token", data.refresh_token);
    set({ user: data.user });
  },

  logout: () => {
    localStorage.removeItem("access_token");
    localStorage.removeItem("refresh_token");
    set({ user: null });
  },

  loadUser: async () => {
    if (!localStorage.getItem("access_token")) {
      set({ initialized: true });
      return;
    }

    set({ loading: true });
    try {
      const user = await api.get<User>("/api/auth/me");
      set({ user, loading: false, initialized: true });
    } catch {
      localStorage.removeItem("access_token");
      localStorage.removeItem("refresh_token");
      set({ loading: false, initialized: true });
    }
  },
}));
