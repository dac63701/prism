"use client";

import { Suspense } from "react";
import { Loader2 } from "lucide-react";
import { VerifyEmailContent } from "./content";

export default function VerifyEmailPage() {
  return (
    <Suspense
      fallback={
        <div className="relative flex min-h-screen items-center justify-center bg-[#050816] px-5">
          <div className="flex flex-col items-center gap-4">
            <Loader2 className="h-10 w-10 animate-spin text-blue-400" />
            <p className="text-sm text-zinc-400">Loading...</p>
          </div>
        </div>
      }
    >
      <VerifyEmailContent />
    </Suspense>
  );
}
