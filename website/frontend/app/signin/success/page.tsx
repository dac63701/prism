import type { Metadata } from "next";
import { PrismLogo } from "@/components/brand-icons";
import { CheckCircle } from "lucide-react";

export const metadata: Metadata = {
  title: "Signed In",
  robots: {
    index: false,
    follow: false,
  },
};

export default function SignInSuccessPage() {
  return (
    <div className="relative flex min-h-screen items-center justify-center bg-[#050816] px-5">
      <div className="pointer-events-none fixed inset-0 overflow-hidden">
        <div className="absolute -left-40 -top-40 h-[500px] w-[500px] rounded-full bg-blue-500/10 blur-[120px]" />
        <div className="absolute -bottom-40 -right-40 h-[500px] w-[500px] rounded-full bg-blue-600/8 blur-[120px]" />
        <div className="absolute left-1/2 top-1/3 h-[300px] w-[600px] -translate-x-1/2 rounded-full bg-blue-400/5 blur-[100px]" />
      </div>
      <div className="relative z-10 flex w-full max-w-sm flex-col items-center gap-8">
        <PrismLogo className="h-12 w-12" />
        <div className="w-full rounded-3xl border border-border bg-[linear-gradient(180deg,rgba(16,25,46,0.95),rgba(8,13,26,0.95))] p-8 text-center shadow-lg shadow-black/20">
          <CheckCircle className="mx-auto mb-4 h-10 w-10 text-green-400" />
          <h1 className="text-2xl font-semibold tracking-tight text-white">
            Signed in to Prism
          </h1>
          <p className="mt-2 text-sm leading-6 text-zinc-400">
            Your account is now connected.
          </p>
          <p className="mt-1 text-sm leading-6 text-zinc-500">
            You can close this tab.
          </p>
        </div>
      </div>
    </div>
  );
}
