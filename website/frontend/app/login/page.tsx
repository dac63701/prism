import type { Metadata } from "next";
import { AuthCard } from "@/components/auth-card";
import { PrismLogo } from "@/components/brand-icons";

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
  searchParams?: Promise<{ desktop?: string; verified?: string }>;
}) {
  const resolved = searchParams ? await searchParams : undefined;
  const desktop = resolved?.desktop === "1" || resolved?.desktop === "true";
  const verified = resolved?.verified === "1";

  return (
    <div className="relative flex min-h-screen items-center justify-center bg-[#050816] px-5">
      <div className="pointer-events-none fixed inset-0 overflow-hidden">
        <div className="absolute -left-40 -top-40 h-[500px] w-[500px] rounded-full bg-blue-500/10 blur-[120px]" />
        <div className="absolute -bottom-40 -right-40 h-[500px] w-[500px] rounded-full bg-blue-600/8 blur-[120px]" />
        <div className="absolute left-1/2 top-1/3 h-[300px] w-[600px] -translate-x-1/2 rounded-full bg-blue-400/5 blur-[100px]" />
      </div>
      <div className="relative z-10 flex w-full max-w-sm flex-col items-center gap-8">
        <div className="flex items-center gap-3">
          <PrismLogo className="h-10 w-10" />
          <span className="text-3xl font-bold tracking-tight text-white">PRISM</span>
        </div>
        <AuthCard desktop={desktop} mode="login" verified={verified} />
      </div>
    </div>
  );
}
