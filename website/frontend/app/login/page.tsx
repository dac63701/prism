import type { Metadata } from "next";
import { AuthCard } from "@/components/auth-card";

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
    <div className="relative flex min-h-screen items-center justify-center bg-[#050816] px-5">
      <div className="pointer-events-none fixed inset-0 overflow-hidden">
        <div className="absolute -left-40 -top-40 h-[500px] w-[500px] rounded-full bg-blue-500/10 blur-[120px]" />
        <div className="absolute -bottom-40 -right-40 h-[500px] w-[500px] rounded-full bg-blue-600/8 blur-[120px]" />
        <div className="absolute left-1/2 top-1/3 h-[300px] w-[600px] -translate-x-1/2 rounded-full bg-blue-400/5 blur-[100px]" />
      </div>
      <div className="relative z-10 flex w-full max-w-sm flex-col items-center gap-8">
        <div className="text-center">
          <div className="text-sm uppercase tracking-[0.3em] text-blue-300/50">Prism</div>
          <p className="mt-2 text-sm text-zinc-500">Clip-based screen recording</p>
        </div>
        <AuthCard desktop={desktop} mode="login" />
      </div>
    </div>
  );
}
