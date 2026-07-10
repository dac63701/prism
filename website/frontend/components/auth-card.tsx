"use client";

import { useState } from "react";
import Link from "next/link";
import { ArrowRight, Mail } from "lucide-react";
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
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function onSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setLoading(true);

    try {
      if (mode === "register") {
        await register(email, password, email.split("@")[0]);
      } else {
        await login(email, password);
      }
      window.location.href = "/dashboard";
    } catch (err) {
      setError(err instanceof Error ? err.message : "Login failed");
    } finally {
      setLoading(false);
    }
  }

  return (
    <Card className="p-6 sm:p-8">
      <div className="mb-6 space-y-2">
        <div className="text-xs uppercase tracking-[0.3em] text-blue-300/70">Prism account</div>
        <h1 className="text-3xl font-semibold tracking-tight text-white">
          {mode === "register" ? "Create account" : "Sign in"}
        </h1>
        <p className="text-sm leading-6 text-zinc-400">
          Use Google or your email account to open the dashboard.
          {desktop ? " After login, Prism will open automatically." : ""}
        </p>
      </div>

      <div className="space-y-3">
        <Button asChild className="w-full justify-center border border-zinc-200 bg-white text-zinc-900 shadow-sm hover:bg-zinc-100" variant="secondary">
          <a href={googleLoginUrl("/dashboard", desktop)}>
            <GoogleLogo />
            Continue with Google
            <ArrowRight className="h-4 w-4" />
          </a>
        </Button>
        {desktop ? (
          <div className="rounded-2xl border border-blue-400/15 bg-blue-500/10 px-4 py-3 text-sm text-blue-100">
            Desktop sign-in is enabled. After logging in, you’ll be sent back to the Prism app.
          </div>
        ) : null}
        <div className="flex items-center gap-3 py-2 text-xs uppercase tracking-[0.25em] text-zinc-500">
          <span className="h-px flex-1 bg-white/10" />
          or
          <span className="h-px flex-1 bg-white/10" />
        </div>
      </div>

      <form className="space-y-4" onSubmit={onSubmit}>
        <label className="block space-y-2">
          <span className="text-sm text-zinc-300">Email</span>
          <div className="relative">
            <Mail className="pointer-events-none absolute left-4 top-1/2 h-4 w-4 -translate-y-1/2 text-zinc-500" />
            <Input
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="you@prism.app"
              className="pl-11"
              type="email"
              required
            />
          </div>
        </label>

        <label className="block space-y-2">
          <span className="text-sm text-zinc-300">Password</span>
          <Input
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            placeholder="••••••••"
            type="password"
            required
          />
        </label>

        {error ? <p className="text-sm text-red-300">{error}</p> : null}

        <Button type="submit" className="w-full" variant="secondary" disabled={loading}>
          {loading ? (mode === "register" ? "Creating account..." : "Signing in...") : mode === "register" ? "Create account" : "Sign in with email"}
        </Button>
      </form>

      <div className="mt-6 flex items-center justify-between text-sm text-zinc-400">
        <span>{mode === "register" ? "Already have an account?" : "Need an account?"}</span>
        <Link href={mode === "register" ? "/login" : "/register"} className="text-blue-300 hover:text-blue-200">
          {mode === "register" ? "Sign in" : "Create one"}
        </Link>
      </div>
    </Card>
  );
}
