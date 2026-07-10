import { cn } from "@/lib/utils";

export default function PrismLogo({ className }: { className?: string }) {
  return (
    <span
      className={cn(
        "inline-flex h-10 w-10 items-center justify-center rounded-2xl border border-blue-400/20 bg-[linear-gradient(135deg,rgba(79,140,255,0.22),rgba(119,168,255,0.08))] text-blue-200 shadow-lg shadow-blue-500/10",
        className
      )}
      aria-hidden="true"
    >
      <svg viewBox="0 0 24 24" className="h-5 w-5" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
        <path d="M12 3 5 7.2v9.6L12 21l7-4.2V7.2L12 3Z" />
        <path d="M12 7.2v13.8" />
        <path d="M5 7.2 12 12l7-4.8" />
      </svg>
    </span>
  );
}
