"use client";

import { useEffect, useState, useCallback } from "react";
import { Card, Input, SectionHeading, Button } from "@/components/ui";
import { getCurrentUser, changePassword, tfaSetup, tfaEnable, tfaDisable } from "@/lib/api";
import type { User } from "@/lib/types";
import { ShieldCheck, Eye, EyeOff, KeyRound, AlertCircle, Smartphone, CheckCircle2 } from "lucide-react";
import QRCode from "qrcode";

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

  // 2FA state
  const [tfaSecret, setTfaSecret] = useState("");
  const [tfaUri, setTfaUri] = useState("");
  const [qrDataUrl, setQrDataUrl] = useState("");
  const [tfaSetupCode, setTfaSetupCode] = useState("");
  const [tfaDisableCode, setTfaDisableCode] = useState("");
  const [settingUpTfa, setSettingUpTfa] = useState(false);
  const [enablingTfa, setEnablingTfa] = useState(false);
  const [disablingTfa, setDisablingTfa] = useState(false);

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

  const handleTfaSetup = useCallback(async () => {
    setError(null);
    setSuccess(null);
    setSettingUpTfa(true);
    try {
      const result = await tfaSetup();
      setTfaSecret(result.secret);
      setTfaUri(result.uri);
      const url = await QRCode.toDataURL(result.uri, { width: 200, margin: 2 });
      setQrDataUrl(url);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to set up 2FA");
    } finally {
      setSettingUpTfa(false);
    }
  }, []);

  async function handleTfaEnable() {
    if (tfaSetupCode.length !== 6) return;
    setError(null);
    setSuccess(null);
    setEnablingTfa(true);
    try {
      await tfaEnable(tfaSetupCode);
      setSuccess("Two-factor authentication enabled");
      setTfaSecret("");
      setTfaUri("");
      setQrDataUrl("");
      setTfaSetupCode("");
      const updated = await getCurrentUser();
      setUser(updated);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to enable 2FA");
    } finally {
      setEnablingTfa(false);
    }
  }

  async function handleTfaDisable() {
    if (tfaDisableCode.length !== 6) return;
    setError(null);
    setSuccess(null);
    setDisablingTfa(true);
    try {
      await tfaDisable(tfaDisableCode);
      setSuccess("Two-factor authentication disabled");
      setTfaDisableCode("");
      const updated = await getCurrentUser();
      setUser(updated);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to disable 2FA");
    } finally {
      setDisablingTfa(false);
    }
  }

  return (
    <div className="mx-auto max-w-4xl space-y-8 px-5 py-8 lg:px-8 lg:py-10">
      <SectionHeading
        eyebrow="Settings"
        title="Account settings"
        description="Manage your password, security, and account preferences."
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

      {/* Two-Factor Authentication */}
      <Card className="space-y-4 p-6">
        <div className="flex items-center gap-3">
          <Smartphone className="h-5 w-5 text-zinc-500" />
          <div>
            <h2 className="text-lg font-semibold text-white">Two-factor authentication</h2>
            <p className="mt-1 text-sm text-zinc-400">
              Add an extra layer of security to your account using an authenticator app.
            </p>
          </div>
        </div>

        {user?.totp_enabled ? (
          <div className="space-y-4">
            <div className="flex items-center gap-2 rounded-xl border border-emerald-400/15 bg-emerald-500/10 px-4 py-3 text-sm text-emerald-200">
              <CheckCircle2 className="h-4 w-4 shrink-0" />
              2FA is currently enabled
            </div>
            <div className="flex gap-2">
              <Input
                value={tfaDisableCode}
                onChange={(e) => setTfaDisableCode(e.target.value.replace(/\D/g, "").slice(0, 6))}
                placeholder="Enter code to disable"
                className="max-w-40 font-mono text-center tracking-widest"
                maxLength={6}
                disabled={disablingTfa}
              />
              <Button
                type="button"
                variant="secondary"
                disabled={tfaDisableCode.length !== 6 || disablingTfa}
                onClick={handleTfaDisable}
                className="text-red-400 hover:border-red-500/40 hover:bg-red-500/10 hover:text-red-300"
              >
                {disablingTfa ? "Disabling..." : "Disable"}
              </Button>
            </div>
          </div>
        ) : tfaUri ? (
          <div className="space-y-4">
            {qrDataUrl && (
              <div className="flex justify-center">
                <img src={qrDataUrl} alt="QR code" className="rounded-xl" />
              </div>
            )}
            <div className="space-y-1">
              <p className="text-xs text-zinc-500">Secret key (manual entry):</p>
              <p className="select-all font-mono text-sm text-zinc-300">{tfaSecret}</p>
            </div>
            <p className="text-xs text-zinc-500">
              Scan the QR code with your authenticator app, then enter the 6-digit code below to confirm.
            </p>
            <div className="flex gap-2">
              <Input
                value={tfaSetupCode}
                onChange={(e) => setTfaSetupCode(e.target.value.replace(/\D/g, "").slice(0, 6))}
                placeholder="000000"
                className="max-w-40 font-mono text-center tracking-widest"
                maxLength={6}
                disabled={enablingTfa}
              />
              <Button
                type="button"
                variant="secondary"
                disabled={tfaSetupCode.length !== 6 || enablingTfa}
                onClick={handleTfaEnable}
              >
                {enablingTfa ? "Enabling..." : "Confirm & Enable"}
              </Button>
            </div>
            <button
              type="button"
              onClick={() => { setTfaSecret(""); setTfaUri(""); setQrDataUrl(""); }}
              className="text-xs text-zinc-500 hover:text-zinc-400"
            >
              Cancel setup
            </button>
          </div>
        ) : (
          <Button type="button" variant="secondary" disabled={settingUpTfa} onClick={handleTfaSetup}>
            {settingUpTfa ? "Setting up..." : "Set up two-factor authentication"}
          </Button>
        )}
      </Card>

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
