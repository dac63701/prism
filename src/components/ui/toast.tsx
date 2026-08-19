import { CheckCircle2, AlertCircle, Info, X } from "lucide-react";
import { cn } from "@/lib/utils";
import { useToastStore } from "@/stores/toast";

const ICONS = {
  default: Info,
  success: CheckCircle2,
  error: AlertCircle,
} as const;

const ACCENTS = {
  default: "text-blue-300",
  success: "text-emerald-400",
  error: "text-red-400",
} as const;

export function Toaster() {
  const toasts = useToastStore((s) => s.toasts);
  const dismiss = useToastStore((s) => s.dismiss);

  return (
    <div className="pointer-events-none fixed bottom-5 right-5 z-50 flex w-80 flex-col gap-2">
      {toasts.map((t) => {
        const Icon = ICONS[t.variant ?? "default"];
        return (
          <div
            key={t.id}
            role="status"
            className={cn(
              "pointer-events-auto animate-scale-in rounded-xl border border-border",
              "bg-[#0b1222]/95 p-3 shadow-2xl shadow-black/40 backdrop-blur",
              "flex items-start gap-2.5"
            )}
          >
            <Icon className={cn("mt-0.5 size-4 shrink-0", ACCENTS[t.variant ?? "default"])} />
            <div className="min-w-0 flex-1">
              <p className="text-sm font-medium text-white">{t.title}</p>
              {t.description ? (
                <p className="mt-0.5 text-xs text-zinc-400">{t.description}</p>
              ) : null}
            </div>
            <button
              type="button"
              aria-label="Dismiss"
              onClick={() => dismiss(t.id)}
              className="shrink-0 rounded-md p-0.5 text-zinc-500 transition hover:text-zinc-200 active:scale-90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20"
            >
              <X className="size-3.5" />
            </button>
          </div>
        );
      })}
    </div>
  );
}