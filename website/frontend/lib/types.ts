export type Role = "user" | "admin";

export interface User {
  id: string;
  email: string;
  display_name: string;
  real_name?: string;
  avatar_url?: string | null;
  google_connected?: boolean;
  role: Role;
  storage_used_bytes: number;
  max_storage_bytes: number;
  email_verified: boolean;
  totp_enabled?: boolean;
  two_factor_method?: string | null;
  created_at: string;
}

export interface ClipListItem {
  id: string;
  user_id: string;
  title: string;
  game: string;
  duration_secs: number;
  size_bytes: number;
  width: number;
  height: number;
  visibility: "public" | "private" | "unlisted";
  thumbnail_path?: string | null;
  share_id: string;
  created_at: string;
  user_email?: string | null;
  user_display_name?: string | null;
}

export interface ClipDetail extends ClipListItem {
  original_filename: string;
  download_count: number;
  updated_at: string;
  share_url: string;
  video_url?: string;
  thumbnail_url?: string | null;
}

export interface AuthResponse {
  user: User;
  access_token: string;
  refresh_token: string;
}

export interface DashboardStats {
  total_users: number;
  total_clips: number;
  total_storage_bytes: number;
  total_storage_gb: number;
  uploads_today: number;
  uploads_this_week: number;
}

export interface AdminUserRow {
  id: string;
  email: string;
  display_name: string;
  avatar_url?: string | null;
  role: Role;
  clip_count: number;
  storage_used_bytes: number;
  created_at: string;
  is_banned: boolean;
}

export interface AdminUserDetail {
  id: string;
  email: string;
  display_name: string;
  role: Role;
  storage_used_bytes: number;
  max_storage_bytes: number;
  is_banned: boolean;
  clip_count: number;
  created_at: string;
  updated_at: string;
}
