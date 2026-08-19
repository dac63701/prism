import { cn } from "@/lib/utils";

export interface PresetOption {
  value: number;
  label: string;
}

interface PresetSliderProps {
  label?: string;
  value: number;
  options: PresetOption[];
  onChange: (value: number) => void;
  className?: string;
}

export default function PresetSlider({
  label,
  value,
  options,
  onChange,
  className,
}: PresetSliderProps) {
  const index = Math.max(
    0,
    options.findIndex((option) => option.value === value)
  );
  const current = options[index] ?? options[0];
  const max = Math.max(0, options.length - 1);
  const pct = max > 0 ? (index / max) * 100 : 0;

  return (
    <div className={cn("flex w-full min-w-0 items-center gap-2", className)}>
      {label && <span className="shrink-0 text-sm text-zinc-400">{label}</span>}
      <div className="relative h-1.5 min-w-0 flex-1 rounded-full bg-surface-2">
        <div
          className="absolute inset-y-0 left-0 rounded-full bg-accent"
          style={{ width: `${pct}%` }}
        />
        <input
          type="range"
          min={0}
          max={max}
          step={1}
          value={index}
          onChange={(e) => {
            const next = options[Number(e.target.value)];
            if (next) onChange(next.value);
          }}
          className="absolute inset-0 h-full w-full cursor-pointer opacity-0"
          aria-label="Bitrate preset"
        />
      </div>
      <span className="shrink-0 whitespace-nowrap text-right text-sm tabular-nums text-zinc-100">
        {current?.label ?? `${value} kbps`}
      </span>
      <div className="hidden shrink-0 flex-nowrap items-center gap-1 overflow-x-auto md:flex">
        {options.map((option) => {
          const active = option.value === value;
          return (
            <button
              key={option.value}
              type="button"
              onClick={() => onChange(option.value)}
              className={cn(
                "rounded-full border px-1.5 py-0.5 text-[10px] whitespace-nowrap transition active:scale-95 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20",
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