"use client";

import { useState } from "react";
import Link from "next/link";
import { ArrowRight, Mail, ShieldCheck, Eye, EyeOff } from "lucide-react";
import { login, register, googleLoginUrl } from "@/lib/api";
import { Button, Card, Input } from "@/components/ui";
import { GoogleLogo } from "@/components/brand-icons";

export function AuthCard({
  desktop = false,
  mode = "login",
}: {
  desktop?: boolean;
  mode?: "login" | "register";
}) {
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [showConfirm, setShowConfirm] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [fieldErrors, setFieldErrors] = useState<Record<string, string>>({});
  const [loading, setLoading] = useState(false);

  function validate(): boolean {
    const errs: Record<string, string> = {};

    if (!email.includes("@")) {
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

    setLoading(true);

    try {
      if (mode === "register") {
        await register(email, password, email.split("@")[0]);
      } else {
        await login(email, password);
      }
      window.location.href = "/dashboard";
    } catch (err) {
      setError(err instanceof Error ? err.message : "Something went wrong");
    } finally {
      setLoading(false);
    }
  }

  const isRegister = mode === "register";

  return (
    <Card className="w-full max-w-sm p-6 sm:p-8">
      <div className="mb-6 space-y-2">
        <div className="text-xs uppercase tracking-[0.3em] text-blue-300/70">
          {isRegister ? "Get started" : "Welcome back"}
        </div>
        <h1 className="text-3xl font-semibold tracking-tight text-white">
          {isRegister ? "Create your account" : "Sign in to Prism"}
        </h1>
        <p className="text-sm leading-6 text-zinc-400">
          {isRegister
            ? "Start saving your best moments to the cloud."
            : "Access your dashboard and clips."}
          {desktop ? " Prism will open after login." : ""}
        </p>
      </div>

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
          <div className="flex items-center gap-2 rounded-xl border border-white/10 bg-white/[0.03] px-4 py-3 text-xs text-zinc-400">
            <ShieldCheck className="h-4 w-4 shrink-0 text-blue-300" />
            Passwords are hashed with Argon2id. We never store plain text.
          </div>
        ) : null}

        {error ? (
          <p className="rounded-xl bg-red-500/10 px-4 py-3 text-sm text-red-300">
            {error}
          </p>
        ) : null}

        <Button
          type="submit"
          className="w-full"
          variant="secondary"
          disabled={loading}
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
