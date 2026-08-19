import { cn } from "@/lib/utils";

export interface SegmentedOption {
  value: string;
  label: string;
}

interface SegmentedControlProps {
  value: string;
  options: SegmentedOption[];
  onChange: (value: string) => void;
  ariaLabel?: string;
  className?: string;
}

/**
 * Chip-row picker for mutually exclusive string options. Styled to match the
 * event-chip buttons used in the Auto-clip section.
 */
export default function SegmentedControl({
  value,
  options,
  onChange,
  ariaLabel,
  className,
}: SegmentedControlProps) {
  return (
    <div
      role="radiogroup"
      aria-label={ariaLabel}
      className={cn("flex flex-wrap gap-1", className)}
    >
      {options.map((option) => {
        const active = option.value === value;
        return (
          <button
            key={option.value}
            type="button"
            role="radio"
            aria-checked={active}
            onClick={() => onChange(option.value)}
            className={cn(
              "rounded-lg border px-2.5 py-1 text-xs font-medium transition active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20 focus-visible:border-blue-400/70",
              active
                ? "border-blue-400/50 bg-blue-500/15 text-blue-200"
                : "border-border bg-white/[0.03] text-zinc-500 hover:text-zinc-300"
            )}
          >
            {option.label}
          </button>
        );
      })}
    </div>
  );
}