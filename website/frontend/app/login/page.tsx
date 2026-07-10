import type { Metadata } from "next";
import { AuthCard } from "@/components/auth-card";
import { SiteShell } from "@/components/site-shell";

export const metadata: Metadata = {
  title: "Login",
  robots: {
    index: false,
    follow: false,
  },
};

export default async function LoginPage({
  searchParams,
}: {
  searchParams?: Promise<{ desktop?: string }>;
}) {
  const resolved = searchParams ? await searchParams : undefined;
  const desktop = resolved?.desktop === "1" || resolved?.desktop === "true";

  return (
    <SiteShell>
      <div className="mx-auto grid max-w-6xl gap-10 px-5 py-16 lg:grid-cols-[1fr_0.95fr] lg:px-8 lg:py-24">
        <div className="space-y-6">
          <div className="text-xs uppercase tracking-[0.3em] text-blue-300/70">Welcome back</div>
          <h1 className="max-w-xl text-5xl font-semibold tracking-tight text-white">Sign in to Prism</h1>
          <p className="max-w-xl text-lg leading-8 text-zinc-400">
            Access your dashboard, manage clips, and continue the desktop login flow if you came from the app.
          </p>
          <div className="rounded-3xl border border-white/10 bg-white/[0.03] p-6 text-sm leading-7 text-zinc-300">
            <p className="font-medium text-white">What happens next?</p>
            <ul className="mt-3 space-y-2 text-zinc-400">
              <li>• Google sign-in opens the system browser like Medal and Outplayed.</li>
              <li>• Email/password works for direct dashboard login.</li>
              <li>• Desktop logins return to Prism automatically after authentication.</li>
            </ul>
          </div>
        </div>
        <AuthCard desktop={desktop} mode="login" />
      </div>
    </SiteShell>
  );
}
