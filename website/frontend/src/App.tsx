import { useEffect } from "react";
import { Routes, Route, Navigate } from "react-router-dom";
import { useAuthStore } from "@/stores/auth";
import DashboardLayout from "@/components/DashboardLayout";
import AuthGuard from "@/components/AuthGuard";
import AdminGuard from "@/components/AdminGuard";
import HomePage from "@/pages/HomePage";
import LoginPage from "@/pages/LoginPage";
import RegisterPage from "@/pages/RegisterPage";
import LibraryPage from "@/pages/LibraryPage";
import ClipDetailPage from "@/pages/ClipDetailPage";
import SettingsPage from "@/pages/SettingsPage";
import PlayerPage from "@/pages/PlayerPage";
import AdminDashboard from "@/pages/admin/AdminDashboard";
import AdminUsersPage from "@/pages/admin/AdminUsersPage";
import AdminUserDetailPage from "@/pages/admin/AdminUserDetailPage";
import AdminClipsPage from "@/pages/admin/AdminClipsPage";
import AdminSettingsPage from "@/pages/admin/AdminSettingsPage";
import AdminLogsPage from "@/pages/admin/AdminLogsPage";

export default function App() {
  const loadUser = useAuthStore((s) => s.loadUser);
  const initialized = useAuthStore((s) => s.initialized);

  useEffect(() => {
    loadUser();
  }, [loadUser]);

  if (!initialized) {
    return (
      <div className="h-screen flex items-center justify-center text-zinc-500 text-sm">
        Loading...
      </div>
    );
  }

  return (
    <Routes>
      <Route path="/login" element={<LoginPage />} />
      <Route path="/register" element={<RegisterPage />} />
      <Route path="/s/:shareId" element={<PlayerPage />} />
      <Route element={<AuthGuard />}>
        <Route element={<DashboardLayout />}>
          <Route path="/" element={<HomePage />} />
          <Route path="/library" element={<LibraryPage />} />
          <Route path="/clip/:id" element={<ClipDetailPage />} />
          <Route path="/settings" element={<SettingsPage />} />
          <Route element={<AdminGuard />}>
            <Route path="/admin" element={<AdminDashboard />} />
            <Route path="/admin/users" element={<AdminUsersPage />} />
            <Route path="/admin/users/:id" element={<AdminUserDetailPage />} />
            <Route path="/admin/clips" element={<AdminClipsPage />} />
            <Route path="/admin/settings" element={<AdminSettingsPage />} />
            <Route path="/admin/logs" element={<AdminLogsPage />} />
          </Route>
        </Route>
      </Route>
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}
