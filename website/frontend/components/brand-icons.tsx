import { cn } from "@/lib/utils";
import { LOGO_SVG } from "@/lib/brand";

export function PrismLogo({ className }: { className?: string }) {
  return (
    <span
      className={cn(
        "inline-flex h-10 w-10 items-center justify-center rounded-2xl border border-blue-400/20 bg-[linear-gradient(135deg,rgba(79,140,255,0.22),rgba(119,168,255,0.08))] text-blue-200 shadow-lg shadow-blue-500/10",
        className
      )}
      aria-hidden="true"
    >
      <span
        className="h-5 w-5"
        dangerouslySetInnerHTML={{ __html: LOGO_SVG }}
      />
    </span>
  );
}

export function GoogleLogo({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      aria-hidden="true"
      className={cn("h-4 w-4 shrink-0", className)}
      fill="none"
    >
      <path d="M21.35 11.1H12v2.98h5.35c-.23 1.35-.92 2.49-1.97 3.25v2.7h3.2c1.87-1.72 2.95-4.25 2.95-7.25 0-.72-.07-1.42-.18-1.68Z" fill="#4285F4" />
      <path d="M12 22c2.68 0 4.93-.88 6.57-2.39l-3.2-2.7c-.89.6-2.03.96-3.37.96-2.59 0-4.79-1.75-5.57-4.1H3.11v2.78A9.98 9.98 0 0 0 12 22Z" fill="#34A853" />
      <path d="M6.43 13.77A5.99 5.99 0 0 1 6.12 12c0-.62.11-1.22.31-1.77V7.46H3.11A9.99 9.99 0 0 0 2 12c0 1.61.39 3.13 1.11 4.46l3.32-2.69Z" fill="#FBBC05" />
      <path d="M12 5.98c1.46 0 2.77.5 3.81 1.48l2.86-2.86C16.92 2.98 14.68 2 12 2A9.98 9.98 0 0 0 3.11 7.46l3.32 2.77C7.21 7.73 9.41 5.98 12 5.98Z" fill="#EA4335" />
    </svg>
  );
}
