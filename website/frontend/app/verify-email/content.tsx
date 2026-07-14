"use client";

import { useEffect, useState, useCallback } from "react";
import { useSearchParams, useRouter } from "next/navigation";
import Link from "next/link";
import { CheckCircle2, XCircle, Loader2 } from "lucide-react";
import { PrismLogo } from "@/components/brand-icons";
import { verifyEmail } from "@/lib/api";

type Status = "verifying" | "success" | "error";

export function VerifyEmailContent() {
  const searchParams = useSearchParams();
  const router = useRouter();
  const token = searchParams.get("token");
  const [status, setStatus] = useState<Status>("verifying");
  const [message, setMessage] = useState("");

  const doVerify = useCallback(async () => {
    if (!token) {
      setStatus("error");
      setMessage("Missing verification token. The link may be invalid.");
      return;
    }

    try {
      const response = await verifyEmail(token);

      if (response.status === 200 || response.type === "opaqueredirect") {
        setStatus("success");
        setMessage("Your email has been verified! You can now sign in to your account.");
        setTimeout(() => router.push("/login?verified=1"), 3000);
      } else {
        setStatus("error");
        setMessage("Something went wrong. Please try again.");
      }
    } catch (err) {
      setStatus("error");
      setMessage(
        err instanceof Error
          ? err.message
          : "Verification failed. The link may be invalid or expired."
      );
    }
  }, [token, router]);

  useEffect(() => {
    doVerify();
  }, [doVerify]);

  return (
    <div className="relative flex min-h-screen items-center justify-center bg-[#050816] px-5">
      <div className="pointer-events-none fixed inset-0 overflow-hidden">
        <div className="absolute -left-40 -top-40 h-[500px] w-[500px] rounded-full bg-blue-500/10 blur-[120px]" />
        <div className="absolute -bottom-40 -right-40 h-[500px] w-[500px] rounded-full bg-blue-600/8 blur-[120px]" />
      </div>
      <div className="relative z-10 flex w-full max-w-sm flex-col items-center gap-8">
        <div className="flex items-center gap-3">
          <PrismLogo className="h-10 w-10" />
          <span className="text-3xl font-bold tracking-tight text-white">PRISM</span>
        </div>

        <div className="w-full rounded-3xl border border-border bg-[linear-gradient(180deg,rgba(16,25,46,0.95),rgba(8,13,26,0.95))] p-8 shadow-lg shadow-black/20">
          {status === "verifying" && (
            <div className="flex flex-col items-center gap-4 text-center">
              <Loader2 className="h-10 w-10 animate-spin text-blue-400" />
              <p className="text-sm text-zinc-400">Verifying your email...</p>
            </div>
          )}

          {status === "success" && (
            <div className="flex flex-col items-center gap-4 text-center">
              <CheckCircle2 className="h-12 w-12 text-emerald-400" />
              <h1 className="text-xl font-semibold text-white">Email Verified!</h1>
              <p className="text-sm leading-6 text-zinc-400">{message}</p>
              <Link
                href="/login?verified=1"
                className="mt-2 inline-flex items-center justify-center gap-2 rounded-xl border border-border bg-white/5 px-4 py-2.5 text-sm font-medium text-zinc-100 transition hover:bg-white/10"
              >
                Sign in to your account
              </Link>
            </div>
          )}

          {status === "error" && (
            <div className="flex flex-col items-center gap-4 text-center">
              <XCircle className="h-12 w-12 text-red-400" />
              <h1 className="text-xl font-semibold text-white">Verification Failed</h1>
              <p className="text-sm leading-6 text-zinc-400">{message}</p>
              <div className="mt-2 flex flex-col gap-3">
                <Link
                  href="/login"
                  className="inline-flex items-center justify-center gap-2 rounded-xl border border-border bg-white/5 px-4 py-2.5 text-sm font-medium text-zinc-100 transition hover:bg-white/10"
                >
                  Back to sign in
                </Link>
                <p className="text-xs text-zinc-500">
                  Need a new verification email?{" "}
                  <Link href="/login" className="text-blue-300 hover:text-blue-200">
                    Sign in to resend
                  </Link>
                </p>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
