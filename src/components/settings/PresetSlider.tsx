import { cn } from "@/lib/utils";

export interface PresetOption {
  value: number;
  label: string;
}

interface PresetSliderProps {
  value: number;
  options: PresetOption[];
  onChange: (value: number) => void;
  className?: string;
  ariaLabel?: string;
}

export default function PresetSlider({
  value,
  options,
  onChange,
  className,
  ariaLabel,
}: PresetSliderProps) {
  const index = Math.max(
    0,
    options.findIndex((option) => option.value === value)
  );
  const current = options[index] ?? options[0];
  const max = Math.max(0, options.length - 1);
  const pct = max > 0 ? (index / max) * 100 : 0;

  return (
    <div className={cn("flex w-full min-w-0 flex-col gap-2", className)}>
      {/* Track + value label */}
      <div className="flex items-center gap-3">
        <div className="relative h-1.5 min-w-0 flex-1 rounded-full bg-surface-2">
          <div
            className="absolute inset-y-0 left-0 rounded-full bg-accent"
            style={{ width: `${pct}%` }}
          />
          {options.map((option, i) => {
            const left = max > 0 ? `${(i / max) * 100}%` : "0%";
            return (
              <span
                key={option.value}
                className={cn(
                  "absolute top-1/2 size-[5px] -translate-x-1/2 -translate-y-1/2 rounded-full",
                  i <= index ? "bg-white/80" : "bg-border"
                )}
                style={{ left }}
              />
            );
          })}
          <input
            type="range"
            min={0}
            max={max}
            step={1}
            value={index}
            aria-label={ariaLabel ?? "Preset"}
            onChange={(e) => {
              const next = options[Number(e.target.value)];
              if (next) onChange(next.value);
            }}
            className="absolute inset-0 h-full w-full cursor-pointer opacity-0"
          />
        </div>
        <span className="shrink-0 whitespace-nowrap text-right text-sm tabular-nums text-zinc-100">
          {current?.label ?? ""}
        </span>
      </div>

      {/* Selectable chips */}
      <div className="flex flex-wrap items-center gap-1">
        {options.map((option) => {
          const active = option.value === value;
          return (
            <button
              key={option.value}
              type="button"
              onClick={() => onChange(option.value)}
              className={cn(
                "rounded-full border px-2 py-0.5 text-[11px] whitespace-nowrap transition active:scale-95 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20 focus-visible:border-blue-400/70",
                active
                  ? "border-accent bg-accent text-white"
                  : "border-border bg-surface text-zinc-400 hover:border-border hover:text-white"
              )}
            >
              {option.label}
            </button>
          );
        })}
      </div>
    </div>
  );
}