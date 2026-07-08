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
}

export default function PresetSlider({
  label,
  value,
  options,
  onChange,
}: PresetSliderProps) {
  const index = Math.max(
    0,
    options.findIndex((option) => option.value === value)
  );
  const current = options[index] ?? options[0];

  return (
    <div className="w-full max-w-[24rem]">
      <div className="flex items-center gap-3">
        {label && <span className="text-sm text-zinc-400 shrink-0">{label}</span>}
        <div className="flex flex-1 items-center gap-3">
          <input
            type="range"
            min={0}
            max={Math.max(0, options.length - 1)}
            step={1}
            value={index}
            onChange={(e) => {
              const next = options[Number(e.target.value)];
              if (next) onChange(next.value);
            }}
            className="w-full accent-zinc-100"
          />
          <span className="w-20 text-right text-sm text-zinc-100 tabular-nums">
            {current?.label ?? `${value} kbps`}
          </span>
        </div>
      </div>

      <div className="mt-2 flex flex-wrap gap-1.5">
        {options.map((option) => {
          const active = option.value === value;
          return (
            <button
              key={option.value}
              type="button"
              onClick={() => onChange(option.value)}
              className={cn(
                "rounded-full border px-2.5 py-1 text-[11px] transition-colors",
                active
                  ? "border-zinc-100 bg-zinc-100 text-zinc-950"
                  : "border-zinc-800 bg-zinc-900 text-zinc-400 hover:text-zinc-200 hover:border-zinc-700"
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
