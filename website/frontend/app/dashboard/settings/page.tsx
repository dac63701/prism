"use client";

import { useEffect, useState } from "react";
import { Card, Input, SectionHeading, Button } from "@/components/ui";
import { getCurrentUser, changePassword } from "@/lib/api";
import type { User } from "@/lib/types";
import { ShieldCheck, Eye, EyeOff, KeyRound, AlertCircle } from "lucide-react";

export default function SettingsPage() {
  const [user, setUser] = useState<User | null>(null);
  const [loadingUser, setLoadingUser] = useState(true);
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [showCurrent, setShowCurrent] = useState(false);
  const [showNew, setShowNew] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  useEffect(() => {
    getCurrentUser()
      .then(setUser)
      .catch(() => setError("Failed to load user data"))
      .finally(() => setLoadingUser(false));
  }, []);

  async function handleChangePassword(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setSuccess(null);

    if (newPassword.length < 8) {
      setError("New password must be at least 8 characters");
      return;
    }

    setSaving(true);
    try {
      await changePassword(currentPassword, newPassword);
      setSuccess("Password changed successfully");
      setCurrentPassword("");
      setNewPassword("");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to change password");
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="mx-auto max-w-4xl space-y-8 px-5 py-8 lg:px-8 lg:py-10">
      <SectionHeading
        eyebrow="Settings"
        title="Account settings"
        description="Manage your password, API keys, and desktop integration here."
      />

      {loadingUser ? (
        <Card className="space-y-6 p-6">
          <div className="h-5 w-40 animate-pulse rounded bg-white/10" />
          <div className="h-4 w-64 animate-pulse rounded bg-white/5" />
        </Card>
      ) : user?.google_connected ? (
        <Card className="space-y-4 p-6">
          <div className="flex items-center gap-3">
            <KeyRound className="h-5 w-5 text-zinc-500" />
            <div>
              <h2 className="text-lg font-semibold text-white">Password</h2>
              <p className="mt-1 text-sm text-zinc-400">
                You signed up with Google. Password authentication is not available for your account.
              </p>
            </div>
          </div>
        </Card>
      ) : (
        <form onSubmit={handleChangePassword}>
          <Card className="space-y-6 p-6">
            <div>
              <h2 className="text-lg font-semibold text-white">Change password</h2>
              <p className="mt-1 text-sm text-zinc-400">
                Use this if you created an email/password account.
              </p>
            </div>
            <div className="grid gap-4 md:grid-cols-2">
              <div className="relative">
                <Input
                  type={showCurrent ? "text" : "password"}
                  placeholder="Current password"
                  value={currentPassword}
                  onChange={(e) => setCurrentPassword(e.target.value)}
                  required
                  className="pr-11"
                />
                <button
                  type="button"
                  onClick={() => setShowCurrent(!showCurrent)}
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-zinc-500 hover:text-zinc-300"
                  tabIndex={-1}
                >
                  {showCurrent ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                </button>
              </div>
              <div className="relative">
                <Input
                  type={showNew ? "text" : "password"}
                  placeholder="New password"
                  value={newPassword}
                  onChange={(e) => setNewPassword(e.target.value)}
                  required
                  className="pr-11"
                />
                <button
                  type="button"
                  onClick={() => setShowNew(!showNew)}
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-zinc-500 hover:text-zinc-300"
                  tabIndex={-1}
                >
                  {showNew ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                </button>
              </div>
            </div>
            <div className="flex items-center gap-2 rounded-xl border border-border bg-white/[0.03] px-4 py-3 text-xs text-zinc-400">
              <ShieldCheck className="h-4 w-4 shrink-0 text-blue-300" />
              Passwords are hashed with Argon2id. We never store plain text.
            </div>
            {error && (
              <div className="flex items-center gap-2 rounded-xl bg-red-500/10 px-4 py-3 text-sm text-red-300">
                <AlertCircle className="h-4 w-4 shrink-0" />
                {error}
              </div>
            )}
            {success && (
              <div className="rounded-xl bg-emerald-500/10 px-4 py-3 text-sm text-emerald-200">
                {success}
              </div>
            )}
            <Button type="submit" variant="secondary" disabled={saving}>
              {saving ? "Saving..." : "Update password"}
            </Button>
          </Card>
        </form>
      )}

      <Card className="space-y-4 p-6">
        <h2 className="text-lg font-semibold text-white">API keys</h2>
        <p className="text-sm text-zinc-400">Create a key for the desktop app to upload clips to the cloud.</p>
        <div className="rounded-2xl border border-dashed border-border bg-white/[0.03] p-5 text-sm text-zinc-500">
          API key management will appear here.
        </div>
      </Card>
    </div>
  );
}
