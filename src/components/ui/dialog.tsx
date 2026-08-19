import { useEffect, useRef } from "react";
import { createPortal } from "react-dom";
import { X } from "lucide-react";
import { cn } from "@/lib/utils";

interface DialogProps {
  open: boolean;
  onClose: () => void;
  title?: string;
  description?: string;
  children?: React.ReactNode;
  footer?: React.ReactNode;
  className?: string;
}

export function Dialog({
  open,
  onClose,
  title,
  description,
  children,
  footer,
  className,
}: DialogProps) {
  const panelRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onKey);
    const prev = document.activeElement as HTMLElement | null;
    const focusable = panelRef.current?.querySelector<HTMLElement>(
      "button, [href], input, select, textarea, [tabindex]:not([tabindex='-1'])"
    );
    focusable?.focus();
    return () => {
      document.removeEventListener("keydown", onKey);
      prev?.focus();
    };
  }, [open, onClose]);

  if (!open) return null;

  return createPortal(
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
      <div
        className="absolute inset-0 bg-black/60 backdrop-blur-sm animate-fade-in"
        onClick={onClose}
      />
      <div
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        className={cn(
          "relative w-full max-w-md animate-scale-in rounded-2xl border border-border",
          "bg-[linear-gradient(180deg,rgba(16,25,46,0.98),rgba(8,13,26,0.98))]",
          "p-5 shadow-2xl shadow-black/50",
          className
        )}
      >
        <button
          type="button"
          aria-label="Close dialog"
          onClick={onClose}
          className="absolute right-3 top-3 rounded-md p-1 text-zinc-500 transition hover:text-white hover:bg-white/5 active:scale-90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20"
        >
          <X className="size-4" />
        </button>
        {title ? (
          <h2 className="text-base font-semibold text-white">{title}</h2>
        ) : null}
        {description ? (
          <p className="mt-1 text-sm text-zinc-400">{description}</p>
        ) : null}
        {children ? <div className="mt-4">{children}</div> : null}
        {footer ? (
          <div className="mt-5 flex items-center justify-end gap-2">{footer}</div>
        ) : null}
      </div>
    </div>,
    document.body
  );
}