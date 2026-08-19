import { cn } from "@/lib/utils";

interface SliderProps {
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (value: number) => void;
  className?: string;
  ariaLabel?: string;
}

export function Slider({
  value,
  min,
  max,
  step = 1,
  onChange,
  className,
  ariaLabel,
}: SliderProps) {
  const pct = max > min ? ((value - min) / (max - min)) * 100 : 0;
  return (
    <div className={cn("flex min-w-0 flex-1 items-center gap-2", className)}>
      <div className="relative h-1.5 min-w-0 flex-1 rounded-full bg-surface-2">
        <div
          className="absolute inset-y-0 left-0 rounded-full bg-accent"
          style={{ width: `${Math.min(100, Math.max(0, pct))}%` }}
        />
        <input
          type="range"
          aria-label={ariaLabel}
          min={min}
          max={max}
          step={step}
          value={value}
          onChange={(e) => onChange(Number(e.target.value))}
          className="absolute inset-0 h-full w-full cursor-pointer opacity-0"
        />
      </div>
    </div>
  );
}