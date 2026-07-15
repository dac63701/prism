"use client";

import { useEffect, useState, useRef, useCallback } from "react";
import { Card, Input, SectionHeading, Button } from "@/components/ui";
import { getCurrentUser, changePassword, tfaSetup, tfaEnable, tfaDisable, tfaSendCode, updateProfile } from "@/lib/api";
import type { User } from "@/lib/types";
import { ShieldCheck, Eye, EyeOff, KeyRound, AlertCircle, Smartphone, CheckCircle2, X, Mail, ArrowLeft, Loader2 } from "lucide-react";
import QRCode from "qrcode";

function useDebounceSubmit() {
  const lastSubmit = useRef(0);
  const minInterval = 1500;

  const canSubmit = useCallback(() => {
    const now = Date.now();
    if (now - lastSubmit.current < minInterval) {
      return false;
    }
    lastSubmit.current = now;
    return true;
  }, []);

  return canSubmit;
}

function parseAuthError(msg: string): string {
  const lower = msg.toLowerCase();
  if (lower.includes("locked") || lower.includes("try again in")) return msg;
  if (lower.includes("rate limit") || lower.includes("too many requests")) return "Too many attempts. Please wait a moment before trying again.";
  if (lower.includes("timed out")) return "Request timed out. Please check your connection and try again.";
  if (lower.includes("recently sent")) return "A code was recently sent. Please wait before requesting a new one.";
  return msg;
}

export default function SettingsPage() {
  const [user, setUser] = useState<User | null>(null);
  const [loadingUser, setLoadingUser] = useState(true);
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [showCurrent, setShowCurrent] = useState(false);
  const [showNew, setShowNew] = useState(false);
  const [saving, setSaving] = useState(false);
  const [savingProfile, setSavingProfile] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [displayName, setDisplayName] = useState("");
  const [realName, setRealName] = useState("");
  const debounce = useDebounceSubmit();

  // 2FA state
  const [showTfaModal, setShowTfaModal] = useState(false);
  const [tfaStep, setTfaStep] = useState<"method" | "totp" | "email">("method");
  const [tfaSecret, setTfaSecret] = useState("");
  const [tfaUri, setTfaUri] = useState("");
  const [qrDataUrl, setQrDataUrl] = useState("");
  const [tfaCode, setTfaCode] = useState("");
  const [tfaMethod, setTfaMethod] = useState<"totp" | "email">("totp");
  const [enablingTfa, setEnablingTfa] = useState(false);
  const [settingUpTfa, setSettingUpTfa] = useState(false);
  const [disableTfaCode, setDisableTfaCode] = useState("");
  const [disablingTfa, setDisablingTfa] = useState(false);
  const [codeSent, setCodeSent] = useState(false);

  useEffect(() => {
    getCurrentUser()
      .then((currentUser) => {
        setUser(currentUser);
        setDisplayName(currentUser.display_name);
        setRealName(currentUser.real_name ?? "");
      })
      .catch(() => setError("Failed to load user data"))
      .finally(() => setLoadingUser(false));
  }, []);

  async function handleProfileUpdate(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setSuccess(null);

    if (!displayName.trim()) {
      setError("Display name cannot be empty");
      return;
    }

    setSavingProfile(true);
    try {
      const updated = await updateProfile(displayName.trim(), realName.trim());
      setUser(updated);
      setDisplayName(updated.display_name);
      setRealName(updated.real_name ?? "");
      setSuccess("Profile updated successfully");
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to update profile");
    } finally {
      setSavingProfile(false);
    }
  }

  async function handleChangePassword(e: React.FormEvent) {
    e.preventDefault();
    setError(null);
    setSuccess(null);

    if (newPassword.length < 8) {
      setError("New password must be at least 8 characters");
      return;
    }

    if (!debounce()) return;

    setSaving(true);
    try {
      await changePassword(currentPassword, newPassword);
      setSuccess("Password changed successfully");
      setCurrentPassword("");
      setNewPassword("");
    } catch (err) {
      setError(parseAuthError(err instanceof Error ? err.message : "Failed to change password"));
    } finally {
      setSaving(false);
    }
  }

  function handleTfaMethodSelect(method: "totp" | "email") {
    if (!debounce()) return;
    setTfaMethod(method);
    setSettingUpTfa(true);
    setError(null);

    if (method === "totp") {
      tfaSetup("totp").then((result) => {
        setTfaSecret(result.secret!);
        setTfaUri(result.uri!);
        return QRCode.toDataURL(result.uri!, { width: 200, margin: 2 });
      }).then((url) => {
        setQrDataUrl(url);
        setTfaStep("totp");
      }).catch((err) => {
        setError(err instanceof Error ? err.message : "Failed to set up 2FA");
      }).finally(() => {
        setSettingUpTfa(false);
      });
    } else {
      tfaSetup("email").then(() => {
        setCodeSent(true);
        setTfaStep("email");
      }).catch((err) => {
        setError(err instanceof Error ? err.message : "Failed to set up 2FA");
      }).finally(() => {
        setSettingUpTfa(false);
      });
    }
  }

  async function handleTfaEnable() {
    if (tfaCode.length !== 6) return;
    if (!debounce()) return;
    setEnablingTfa(true);
    setError(null);
    try {
      await tfaEnable(tfaMethod, tfaCode);
      const updated = await getCurrentUser();
      setUser(updated);
      setShowTfaModal(false);
      setSuccess("Two-factor authentication enabled");
      setTfaStep("method");
      setTfaCode("");
      setTfaSecret("");
      setTfaUri("");
      setQrDataUrl("");
      setCodeSent(false);
    } catch (err) {
      setError(parseAuthError(err instanceof Error ? err.message : "Failed to enable 2FA"));
    } finally {
      setEnablingTfa(false);
    }
  }

  async function handleTfaDisable() {
    if (disableTfaCode.length !== 6) return;
    if (!debounce()) return;
    setDisablingTfa(true);
    setError(null);
    try {
      const method = user?.two_factor_method || "totp";
      await tfaDisable(method, disableTfaCode);
      setSuccess("Two-factor authentication disabled");
      setDisableTfaCode("");
      const updated = await getCurrentUser();
      setUser(updated);
    } catch (err) {
      setError(parseAuthError(err instanceof Error ? err.message : "Failed to disable 2FA"));
    } finally {
      setDisablingTfa(false);
    }
  }

  async function handleTfaResendCode() {
    if (!debounce()) return;
    setError(null);
    setCodeSent(false);
    try {
      await tfaSendCode();
      setCodeSent(true);
    } catch (err) {
      setError(parseAuthError(err instanceof Error ? err.message : "Failed to send code"));
    }
  }

  async function handleTfaSendDisableCode() {
    if (!debounce()) return;
    setError(null);
    setSuccess(null);
    try {
      await tfaSendCode();
      setSuccess("Code sent! Check your email.");
    } catch (err) {
      setError(parseAuthError(err instanceof Error ? err.message : "Failed to send code"));
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
      ) : (
        <form onSubmit={handleProfileUpdate}>
          <Card className="space-y-6 p-6">
            <div>
              <h2 className="text-lg font-semibold text-white">Profile</h2>
              <p className="mt-1 text-sm text-zinc-400">
                Your display name is public. Your real name is only visible to you.
              </p>
            </div>
            <div className="grid gap-4 md:grid-cols-2">
              <label className="space-y-2">
                <span className="text-sm text-zinc-300">Display name</span>
                <Input
                  value={displayName}
                  onChange={(e) => setDisplayName(e.target.value)}
                  required
                />
              </label>
              <label className="space-y-2">
                <span className="text-sm text-zinc-300">Real name</span>
                <Input
                  value={realName}
                  onChange={(e) => setRealName(e.target.value)}
                  placeholder="Optional"
                />
              </label>
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
            <Button type="submit" variant="secondary" disabled={savingProfile}>
              {savingProfile ? "Saving..." : "Save profile"}
            </Button>
          </Card>
        </form>
      )}

      {loadingUser ? null : user?.google_connected ? (
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
              Add an extra layer of security to your account.
            </p>
          </div>
        </div>

        {user?.totp_enabled || user?.two_factor_method ? (
          <div className="space-y-4">
            <div className="flex items-center gap-2 rounded-xl border border-emerald-400/15 bg-emerald-500/10 px-4 py-3 text-sm text-emerald-200">
              <CheckCircle2 className="h-4 w-4 shrink-0" />
              <span>
                2FA is currently enabled
                {user?.two_factor_method === "email" ? " via Email" : user?.two_factor_method === "totp" ? " via Authenticator App" : ""}
              </span>
            </div>
            {user?.two_factor_method === "email" && (
              <div className="flex gap-2">
                <Button
                  type="button"
                  variant="secondary"
                  onClick={handleTfaSendDisableCode}
                >
                  Send code to email
                </Button>
              </div>
            )}
            <div className="flex gap-2">
              <Input
                value={disableTfaCode}
                onChange={(e) => setDisableTfaCode(e.target.value.replace(/\D/g, "").slice(0, 6))}
                placeholder="Enter code to disable"
                className="max-w-40 font-mono text-center tracking-widest"
                maxLength={6}
                disabled={disablingTfa}
              />
              <Button
                type="button"
                variant="secondary"
                disabled={disableTfaCode.length !== 6 || disablingTfa}
                onClick={handleTfaDisable}
                className="text-red-400 hover:border-red-500/40 hover:bg-red-500/10 hover:text-red-300"
              >
                {disablingTfa ? "Disabling..." : "Disable"}
              </Button>
            </div>
          </div>
        ) : (
          <Button type="button" variant="secondary" onClick={() => setShowTfaModal(true)}>
            Set up two-factor authentication
          </Button>
        )}
      </Card>

      {showTfaModal ? (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4">
          <div className="relative w-full max-w-md rounded-3xl border border-border bg-[linear-gradient(180deg,rgba(16,25,46,0.98),rgba(8,13,26,0.98))] p-6 shadow-2xl">
            <button
              type="button"
              onClick={() => { setShowTfaModal(false); setTfaStep("method"); setTfaCode(""); setError(null); setCodeSent(false); }}
              className="absolute right-4 top-4 text-zinc-500 hover:text-zinc-300"
            >
              <X className="h-5 w-5" />
            </button>

            {tfaStep === "method" && (
              <div className="space-y-5">
                <div>
                  <h2 className="text-lg font-semibold text-white">Choose your 2FA method</h2>
                  <p className="mt-1 text-sm text-zinc-400">Select how you want to receive verification codes.</p>
                </div>
                <div className="space-y-3">
                  <button
                    type="button"
                    onClick={() => handleTfaMethodSelect("totp")}
                    disabled={settingUpTfa}
                    className="flex w-full items-center gap-4 rounded-2xl border border-border bg-white/[0.03] p-4 text-left transition hover:bg-white/[0.06] disabled:opacity-50"
                  >
                    <Smartphone className="h-8 w-8 shrink-0 text-blue-400" />
                    <div>
                      <div className="font-medium text-white">Authenticator App</div>
                      <div className="text-xs text-zinc-400">Use an authenticator app like Google Authenticator or Authy</div>
                    </div>
                  </button>
                  <button
                    type="button"
                    onClick={() => handleTfaMethodSelect("email")}
                    disabled={settingUpTfa}
                    className="flex w-full items-center gap-4 rounded-2xl border border-border bg-white/[0.03] p-4 text-left transition hover:bg-white/[0.06] disabled:opacity-50"
                  >
                    <Mail className="h-8 w-8 shrink-0 text-blue-400" />
                    <div>
                      <div className="font-medium text-white">Email</div>
                      <div className="text-xs text-zinc-400">Receive verification codes via email</div>
                    </div>
                  </button>
                </div>
                {settingUpTfa && (
                  <div className="flex items-center justify-center gap-2 text-sm text-zinc-400">
                    <Loader2 className="h-4 w-4 animate-spin" />
                    Setting up...
                  </div>
                )}
              </div>
            )}

            {tfaStep === "totp" && (
              <div className="space-y-4">
                <button
                  type="button"
                  onClick={() => { setTfaStep("method"); setTfaSecret(""); setTfaUri(""); setQrDataUrl(""); setTfaCode(""); setError(null); }}
                  className="flex items-center gap-1 text-xs text-zinc-500 hover:text-zinc-400"
                >
                  <ArrowLeft className="h-3 w-3" /> Back to methods
                </button>
                {qrDataUrl && (
                  <div className="flex justify-center">
                    <img src={qrDataUrl} alt="QR code" className="rounded-xl" />
                  </div>
                )}
                <div className="space-y-1">
                  <p className="text-xs text-zinc-500">Secret key (manual entry):</p>
                  <p className="select-all break-all font-mono text-sm text-zinc-300">{tfaSecret}</p>
                </div>
                <p className="text-xs text-zinc-500">
                  Scan the QR code with your authenticator app, then enter the 6-digit code below to confirm.
                </p>
                <div className="flex gap-2">
                  <Input
                    value={tfaCode}
                    onChange={(e) => setTfaCode(e.target.value.replace(/\D/g, "").slice(0, 6))}
                    placeholder="000000"
                    className="max-w-40 font-mono text-center tracking-widest"
                    maxLength={6}
                    disabled={enablingTfa}
                  />
                  <Button
                    type="button"
                    variant="secondary"
                    disabled={tfaCode.length !== 6 || enablingTfa}
                    onClick={handleTfaEnable}
                  >
                    {enablingTfa ? "Enabling..." : "Confirm & Enable"}
                  </Button>
                </div>
              </div>
            )}

            {tfaStep === "email" && (
              <div className="space-y-4">
                <button
                  type="button"
                  onClick={() => { setTfaStep("method"); setTfaCode(""); setCodeSent(false); setError(null); }}
                  className="flex items-center gap-1 text-xs text-zinc-500 hover:text-zinc-400"
                >
                  <ArrowLeft className="h-3 w-3" /> Back to methods
                </button>
                <div className="flex flex-col items-center gap-3 text-center">
                  <Mail className="h-10 w-10 text-blue-400" />
                  <div>
                    <h3 className="font-medium text-white">Check your email</h3>
                    <p className="mt-1 text-sm text-zinc-400">
                      We sent a 6-digit code to <span className="text-zinc-200">{user?.email}</span>
                    </p>
                  </div>
                </div>
                <div className="flex gap-2">
                  <Input
                    value={tfaCode}
                    onChange={(e) => setTfaCode(e.target.value.replace(/\D/g, "").slice(0, 6))}
                    placeholder="000000"
                    className="max-w-40 font-mono text-center tracking-widest"
                    maxLength={6}
                    disabled={enablingTfa}
                  />
                  <Button
                    type="button"
                    variant="secondary"
                    disabled={tfaCode.length !== 6 || enablingTfa}
                    onClick={handleTfaEnable}
                  >
                    {enablingTfa ? "Enabling..." : "Enable"}
                  </Button>
                </div>
                {codeSent ? (
                  <p className="text-xs text-emerald-400">Code sent! Check your inbox.</p>
                ) : null}
                <button
                  type="button"
                  onClick={handleTfaResendCode}
                  className="text-xs text-blue-300 hover:text-blue-200"
                >
                  Resend code
                </button>
              </div>
            )}

            {error && (
              <div className="mt-4 flex items-center gap-2 rounded-xl bg-red-500/10 px-4 py-3 text-sm text-red-300">
                <AlertCircle className="h-4 w-4 shrink-0" />
                {error}
              </div>
            )}
          </div>
        </div>
      ) : null}

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
