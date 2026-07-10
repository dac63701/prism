import type { Metadata } from "next";
import { AuthCard } from "@/components/auth-card";
import { SiteShell } from "@/components/site-shell";
import { Badge, Panel, StatCard } from "@/components/ui";

export const metadata: Metadata = {
  title: "Register",
  robots: {
    index: false,
    follow: false,
  },
};

export default function RegisterPage() {
  return (
    <SiteShell>
      <div className="mx-auto grid max-w-6xl gap-10 px-5 py-16 lg:grid-cols-[1fr_0.95fr] lg:px-8 lg:py-24">
        <div className="space-y-6">
          <Badge>Get started</Badge>
          <h1 className="max-w-xl text-5xl font-semibold tracking-tight text-white">Create a Prism account</h1>
          <p className="max-w-xl text-lg leading-8 text-zinc-400">
            Use Google or email/password to create your account and start saving clips to the cloud.
          </p>
          <div className="grid gap-3 sm:grid-cols-3">
            <StatCard label="Login" value="Google" hint="Fast, low-friction sign-in" />
            <StatCard label="Storage" value="Cloud" hint="Keep clips organized" />
            <StatCard label="Flow" value="Desktop" hint="Return to the app automatically" />
          </div>
          <Panel className="space-y-3 p-5">
            <div className="text-sm font-medium text-white">What you get</div>
            <ul className="space-y-2 text-sm leading-6 text-zinc-400">
              <li>• A clean dashboard for clips, tags, and settings.</li>
              <li>• Public share pages with good previews for social sites.</li>
              <li>• A desktop login flow that hands you back to Prism.</li>
            </ul>
          </Panel>
        </div>
        <AuthCard mode="register" />
      </div>
    </SiteShell>
  );
}
