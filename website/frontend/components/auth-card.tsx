"use client";

import { useState, useRef, useCallback } from "react";
import Link from "next/link";
import { ArrowRight, Mail, ShieldCheck, Eye, EyeOff, CheckCircle2, RefreshCw, KeyRound, Smartphone } from "lucide-react";
import { login, register, googleLoginUrl, resendVerification, verifyCode, tfaLogin, tfaSendCodeLogin } from "@/lib/api";
import { Button, Card, Input } from "@/components/ui";
import { GoogleLogo } from "@/components/brand-icons";

function useDebounceSubmit() {
  const lastSubmit = useRef(0);
  const minInterval = 1000;

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

export function AuthCard({
  desktop = false,
  mode = "login",
  verified,
}: {
  desktop?: boolean;
  mode?: "login" | "register";
  verified?: boolean;
}) {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [showConfirm, setShowConfirm] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(false);
  const [registeredEmail, setRegisteredEmail] = useState<string | null>(null);
  const [resending, setResending] = useState(false);
  const [resendSent, setResendSent] = useState(false);
  const [showCodeInput, setShowCodeInput] = useState(false);
  const [verificationCode, setVerificationCode] = useState("");
  const [verifyingCode, setVerifyingCode] = useState(false);
  const [requires2fa, setRequires2fa] = useState(false);
  const [tempToken, setTempToken] = useState("");
  const [tfaCode, setTfaCode] = useState("");
  const [tfaMethod, setTfaMethod] = useState("totp");
  const [verifyingTfa, setVerifyingTfa] = useState(false);
  const debounceSubmit = useDebounceSubmit();

  function validate(): boolean {
    const errs: Record<string, string> = {};

    if (!/^[^\s@]+@[^\s@]+\.[^\s@]{2,}$/i.test(email)) {
      errs.email = "Enter a valid email address";
    }

    if (password.length < 8) {
      errs.password = "At least 8 characters";
    }

    if (mode === "register" && password !== confirmPassword) {
      errs.confirmPassword = "Passwords do not match";
    }

    setFieldErrors(errs);
    return Object.keys(errs).length === 0;
  }

  async function onSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);

    if (!validate()) return;
    if (!debounceSubmit()) return;

    setLoading(true);

    try {
      if (mode === "register") {
        const result = await register(email, password, email.split("@")[0]);
        setRegisteredEmail(result.email);
      } else {
        try {
          const response = await login(email, password);
          const data = response as unknown as Record<string, unknown>;
          if (data?.requires_2fa === true && typeof data?.temp_token === "string") {
            setRequires2fa(true);
            setTempToken(data.temp_token as string);
            setTfaMethod((data.method as string) || "totp");
            setError(null);
            return;
          }
          window.location.href = "/dashboard";
        } catch (err) {
          const msg = err instanceof Error ? err.message : "Something went wrong";
          if (msg.toLowerCase().includes("verify your email")) {
            setShowCodeInput(true);
          }
          throw err;
        }
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Something went wrong";

      // Parse lockout messages
      if (msg.toLowerCase().includes("locked") || msg.toLowerCase().includes("try again in")) {
        setError(msg);
      } else if (msg.toLowerCase().includes("rate limit") || msg.toLowerCase().includes("too many requests")) {
        setError("Too many attempts. Please wait a moment before trying again.");
      } else if (msg.toLowerCase().includes("timed out")) {
        setError("Request timed out. Please check your connection and try again.");
      } else {
        setError(msg);
      }
    } finally {
      setLoading(false);
    }
  }

  async function handleResend() {
    if (!registeredEmail) return;
    setResending(true);
    setResendSent(false);
    try {
      await resendVerification(registeredEmail);
      setResendSent(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to resend verification email");
    } finally {
      setResending(false);
    }
  }

  async function handleVerifyCode() {
    if (!email || verificationCode.length !== 6) return;
    setVerifyingCode(true);
    setError(null);
    try {
      await verifyCode(email, verificationCode);
      window.location.href = "/dashboard";
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Verification failed";
      if (msg.toLowerCase().includes("locked") || msg.toLowerCase().includes("try again in")) {
        setError(msg);
      } else {
        setError(msg);
      }
    } finally {
      setVerifyingCode(false);
    }
  }

  async function handleResendFromLogin() {
    if (!email) return;
    if (!debounceSubmit()) return;
    setResending(true);
    setResendSent(false);
    setError(null);
    try {
      await resendVerification(email);
      setResendSent(true);
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Failed to resend verification email";
      if (msg.toLowerCase().includes("please wait")) {
        setError(msg);
      } else {
        setError(msg);
      }
    } finally {
      setResending(false);
    }
  }

  async function handleTfaLogin() {
    if (!tempToken || tfaCode.length !== 6) return;
    if (!debounceSubmit()) return;
    setVerifyingTfa(true);
    setError(null);
    try {
      await tfaLogin(tempToken, tfaCode, tfaMethod);
      window.location.href = "/dashboard";
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Two-factor verification failed";
      if (msg.toLowerCase().includes("locked") || msg.toLowerCase().includes("try again in")) {
        setError(msg);
      } else if (msg.toLowerCase().includes("recently sent")) {
        setError("A code was recently sent. Please wait before requesting a new one.");
      } else {
        setError(msg);
      }
    } finally {
      setVerifyingTfa(false);
    }
  }

  const isRegister = mode === "register";

  if (isRegister && registeredEmail) {
    return (
      <Card className="w-full max-w-sm p-6 sm:p-8">
        <div className="flex flex-col items-center gap-4 text-center">
          <CheckCircle2 className="h-12 w-12 text-emerald-400" />
          <div className="space-y-1">
            <h1 className="text-2xl font-semibold tracking-tight text-white">
              Check your email
            </h1>
            <p className="text-sm leading-6 text-zinc-400">
              We sent a verification link to{" "}
              <span className="font-medium text-zinc-200">{registeredEmail}</span>
            </p>
          </div>
          <div className="w-full rounded-2xl border border-blue-400/15 bg-blue-500/10 px-4 py-3 text-left text-sm text-blue-100">
            <p>
              Click the link in the email to verify your account. You won&apos;t be
              able to sign in until your email is confirmed.
            </p>
          </div>

          {resendSent ? (
            <p className="text-xs text-emerald-400">Verification email resent!</p>
          ) : (
            <button
              type="button"
              disabled={resending}
              onClick={handleResend}
              className="inline-flex items-center gap-2 text-sm text-blue-300 hover:text-blue-200 disabled:opacity-50"
            >
              <RefreshCw className={`h-3.5 w-3.5 ${resending ? "animate-spin" : ""}`} />
              {resending ? "Sending..." : "Resend verification email"}
            </button>
          )}

          <div className="pt-2 text-xs text-zinc-500">
            Wrong address?{" "}
            <button
              type="button"
              onClick={() => setRegisteredEmail(null)}
              className="text-blue-300 hover:text-blue-200"
            >
              Go back
            </button>
          </div>
        </div>
      </Card>
    );
  }

  return (
    <Card className="w-full max-w-sm p-6 sm:p-8">
      <div className="mb-6 space-y-2">
        <div className="text-xs uppercase tracking-[0.3em] text-blue-300/70">
          {isRegister ? "Get started" : "Welcome back"}
        </div>
        <h1 className="text-3xl font-semibold tracking-tight text-white">
          {isRegister ? "Create your account" : "Sign in"}
        </h1>
        <p className="text-sm leading-6 text-zinc-400">
          {isRegister
            ? "Start saving your best moments to the cloud."
            : "Access your dashboard and clips."}
          {desktop ? " Prism will open after login." : ""}
        </p>
      </div>

      {verified && (
        <div className="mb-4 flex items-center gap-2 rounded-xl border border-emerald-400/15 bg-emerald-500/10 px-4 py-3 text-sm text-emerald-200">
          <CheckCircle2 className="h-4 w-4 shrink-0" />
          Email verified! You can now sign in.
        </div>
      )}

      <div className="space-y-3">
        <a
          href={googleLoginUrl("/dashboard", desktop)}
          className="inline-flex w-full items-center justify-center gap-2 rounded-xl bg-white/15 px-4 py-2.5 text-sm font-medium text-white shadow-sm transition hover:bg-white/20 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-400 focus-visible:ring-offset-2 focus-visible:ring-offset-[#050816]"
        >
          <GoogleLogo />
          {isRegister ? "Sign up with Google" : "Continue with Google"}
          <ArrowRight className="h-4 w-4" />
        </a>

        {desktop ? (
          <div className="rounded-2xl border border-blue-400/15 bg-blue-500/10 px-4 py-3 text-sm text-blue-100">
            Desktop sign-in is enabled. After logging in, you&rsquo;ll be sent
            back to the Prism app.
          </div>
        ) : null}

        <div className="flex items-center gap-3 py-2 text-xs uppercase tracking-[0.25em] text-zinc-500">
          <span className="h-px flex-1 bg-white/10" />
          or
          <span className="h-px flex-1 bg-white/10" />
        </div>
      </div>

      <form className="space-y-4" onSubmit={onSubmit} noValidate>
        <label className="block space-y-2">
          <span className="text-sm text-zinc-300">Email</span>
          <div className="relative">
            <Mail className="pointer-events-none absolute left-4 top-1/2 h-4 w-4 -translate-y-1/2 text-zinc-500" />
            <Input
              value={email}
              onChange={(e) => {
                setEmail(e.target.value);
                setFieldErrors((prev) => ({ ...prev, email: "" }));
              }}
              placeholder="you@example.com"
              className="pl-11"
              type="email"
              required
            />
          </div>
          {fieldErrors.email ? (
            <p className="text-xs text-red-300">{fieldErrors.email}</p>
          ) : null}
        </label>

        <label className="block space-y-2">
          <span className="text-sm text-zinc-300">Password</span>
          <div className="relative">
            <Input
              value={password}
              onChange={(e) => {
                setPassword(e.target.value);
                setFieldErrors((prev) => ({ ...prev, password: "" }));
              }}
              placeholder="At least 8 characters"
              type={showPassword ? "text" : "password"}
              required
              className="pr-11"
            />
            <button
              type="button"
              onClick={() => setShowPassword(!showPassword)}
              className="absolute right-3 top-1/2 -translate-y-1/2 text-zinc-500 hover:text-zinc-300"
              tabIndex={-1}
            >
              {showPassword ? (
                <EyeOff className="h-4 w-4" />
              ) : (
                <Eye className="h-4 w-4" />
              )}
            </button>
          </div>
          {fieldErrors.password ? (
            <p className="text-xs text-red-300">{fieldErrors.password}</p>
          ) : null}
        </label>

        {isRegister ? (
          <label className="block space-y-2">
            <span className="text-sm text-zinc-300">Confirm password</span>
            <div className="relative">
              <Input
                value={confirmPassword}
                onChange={(e) => {
                  setConfirmPassword(e.target.value);
                  setFieldErrors((prev) => ({
                    ...prev,
                    confirmPassword: "",
                  }));
                }}
                placeholder="Re-enter your password"
                type={showConfirm ? "text" : "password"}
                required
                className="pr-11"
              />
              <button
                type="button"
                onClick={() => setShowConfirm(!showConfirm)}
                className="absolute right-3 top-1/2 -translate-y-1/2 text-zinc-500 hover:text-zinc-300"
                tabIndex={-1}
              >
                {showConfirm ? (
                  <EyeOff className="h-4 w-4" />
                ) : (
                  <Eye className="h-4 w-4" />
                )}
              </button>
            </div>
            {fieldErrors.confirmPassword ? (
              <p className="text-xs text-red-300">
                {fieldErrors.confirmPassword}
              </p>
            ) : null}
          </label>
        ) : null}

        {isRegister ? (
          <div className="flex items-center gap-2 rounded-xl border border-border bg-white/[0.03] px-4 py-3 text-xs text-zinc-400">
            <ShieldCheck className="h-4 w-4 shrink-0 text-blue-300" />
            Passwords are hashed with Argon2id. We never store plain text.
          </div>
        ) : null}

        {error ? (
          <p className="rounded-xl bg-red-500/10 px-4 py-3 text-sm text-red-300">
            {error}
          </p>
        ) : null}

        {!isRegister && showCodeInput ? (
          <div className="space-y-3 rounded-2xl border border-blue-400/15 bg-blue-500/10 p-4">
            <div className="flex items-center gap-2 text-sm text-blue-100">
              <KeyRound className="h-4 w-4 shrink-0" />
              <span>Enter the 6-digit code from your email</span>
            </div>
            <div className="flex gap-2">
              <Input
                value={verificationCode}
                onChange={(e) => setVerificationCode(e.target.value.replace(/\D/g, "").slice(0, 6))}
                placeholder="000000"
                className="flex-1 text-center font-mono text-lg tracking-widest"
                maxLength={6}
                disabled={verifyingCode}
              />
              <Button
                type="button"
                variant="secondary"
                disabled={verificationCode.length !== 6 || verifyingCode}
                onClick={handleVerifyCode}
                className="shrink-0"
              >
                {verifyingCode ? "Verifying..." : "Verify"}
              </Button>
            </div>
            {resendSent ? (
              <p className="text-xs text-emerald-400">Code sent! Check your inbox.</p>
            ) : (
              <button
                type="button"
                disabled={resending}
                onClick={handleResendFromLogin}
                className="flex items-center gap-1.5 text-xs text-blue-300 hover:text-blue-200 disabled:opacity-50"
              >
                <RefreshCw className={`h-3 w-3 ${resending ? "animate-spin" : ""}`} />
                {resending ? "Sending..." : "Resend code"}
              </button>
            )}
          </div>
        ) : null}

        {!isRegister && requires2fa ? (
          <div className="space-y-3 rounded-2xl border border-blue-400/15 bg-blue-500/10 p-4">
            <div className="flex items-center gap-2 text-sm text-blue-100">
              {tfaMethod === "email" ? (
                <Mail className="h-4 w-4 shrink-0" />
              ) : (
                <Smartphone className="h-4 w-4 shrink-0" />
              )}
              <span>
                {tfaMethod === "email"
                  ? "Enter the code sent to your email"
                  : "Enter the code from your authenticator app"}
              </span>
            </div>
            <div className="flex gap-2">
              <Input
                value={tfaCode}
                onChange={(e) => setTfaCode(e.target.value.replace(/\D/g, "").slice(0, 6))}
                placeholder="000000"
                className="flex-1 text-center font-mono text-lg tracking-widest"
                maxLength={6}
                disabled={verifyingTfa}
              />
              <Button
                type="button"
                variant="secondary"
                disabled={tfaCode.length !== 6 || verifyingTfa}
                onClick={handleTfaLogin}
                className="shrink-0"
              >
                {verifyingTfa ? "Verifying..." : "Verify"}
              </Button>
            </div>
            {tfaMethod === "email" && (
              <button
                type="button"
                disabled={verifyingTfa}
                onClick={async () => {
                  if (!debounceSubmit()) return;
                  try {
                    await tfaSendCodeLogin(tempToken);
                  } catch (err) {
                    const msg = err instanceof Error ? err.message : "Failed to send code";
                    if (msg.toLowerCase().includes("recently sent")) {
                      setError("A code was recently sent. Please wait.");
                    } else {
                      setError(msg);
                    }
                  }
                }}
                className="flex items-center gap-1.5 text-xs text-blue-300 hover:text-blue-200 disabled:opacity-50"
              >
                Resend code
              </button>
            )}
            <button
              type="button"
              onClick={() => { setRequires2fa(false); setError(null); }}
              className="text-xs text-zinc-500 hover:text-zinc-400"
            >
              Go back
            </button>
          </div>
        ) : null}

        <Button
          type="submit"
          className="w-full"
          variant="secondary"
          disabled={loading || requires2fa}
          style={requires2fa ? { display: "none" } : undefined}
        >
          {loading
            ? isRegister
              ? "Creating account..."
              : "Signing in..."
            : isRegister
              ? "Create account"
              : "Sign in"}
        </Button>
      </form>

      <div className="mt-6 flex items-center justify-between text-sm text-zinc-400">
        <span>
          {isRegister ? "Already have an account?" : "Need an account?"}
        </span>
        <Link
          href={isRegister ? "/login" : "/register"}
          className="text-blue-300 hover:text-blue-200"
        >
          {isRegister ? "Sign in" : "Create one"}
        </Link>
      </div>
    </Card>
  );
}
