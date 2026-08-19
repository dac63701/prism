import { useEffect, useMemo, useRef, useState, Children, isValidElement } from "react";
import { Check, ChevronDown } from "lucide-react";
import { cn } from "@/lib/utils";

interface SelectOption {
  value: string;
  label: string;
}

export function Select({
  value,
  onChange,
  children,
  className,
  disabled,
  "aria-label": ariaLabel,
}: {
  value: string | number;
  onChange: (e: { target: { value: string } }) => void;
  children: React.ReactNode;
  className?: string;
  disabled?: boolean;
  "aria-label"?: string;
}) {
  const options = useMemo<SelectOption[]>(
    () =>
      Children.toArray(children).flatMap((child) => {
        if (isValidElement(child)) {
          const props = child.props as { value?: string | number; children?: React.ReactNode };
          return [
            {
              value: String(props.value),
              label: typeof props.children === "string" ? props.children : "",
            },
          ];
        }
        return [];
      }),
    [children]
  );

  const selected = options.find((opt) => opt.value === String(value));
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onDocPointer = (e: PointerEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("pointerdown", onDocPointer);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("pointerdown", onDocPointer);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <div ref={rootRef} className="relative inline-flex">
      <button
        type="button"
        aria-label={ariaLabel}
        aria-haspopup="listbox"
        aria-expanded={open}
        disabled={disabled}
        onClick={() => setOpen((openValue) => !openValue)}
        className={cn(
          "flex items-center justify-between gap-2 rounded-xl border border-border bg-surface py-1.5 pl-3 pr-2.5 text-sm text-white",
          "max-w-full transition-colors outline-none cursor-pointer",
          "focus:border-blue-400/70 focus:ring-2 focus:ring-blue-500/20",
          "hover:border-border",
          disabled && "cursor-not-allowed opacity-50",
          className
        )}
      >
        <span className="truncate">{selected?.label ?? "Select…"}</span>
        <ChevronDown
          className={cn(
            "size-3.5 shrink-0 text-zinc-500 transition-transform",
            open && "rotate-180"
          )}
        />
      </button>

      {open && (
        <ul
          role="listbox"
          className="absolute left-0 right-0 top-full z-50 mt-1.5 max-h-64 min-w-full overflow-auto rounded-xl border border-border bg-surface/95 p-1 shadow-xl shadow-black/40 backdrop-blur animate-fade-in"
        >
          {options.map((opt) => {
            const isSelected = opt.value === String(value);
            return (
              <li key={opt.value} role="option" aria-selected={isSelected}>
                <button
                  type="button"
                  onClick={() => {
                    onChange({ target: { value: opt.value } });
                    setOpen(false);
                  }}
                  className={cn(
                    "flex w-full items-center justify-between gap-2 rounded-lg px-2.5 py-1.5 text-left text-sm transition",
                    isSelected
                      ? "bg-white/5 text-white"
                      : "text-zinc-300 hover:bg-white/5 hover:text-white"
                  )}
                >
                  <span className="truncate">{opt.label}</span>
                  {isSelected && <Check className="size-3.5 shrink-0 text-blue-300" />}
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}