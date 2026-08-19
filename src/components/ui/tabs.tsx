import { cn } from "@/lib/utils";

export interface TabItem<T extends string> {
  value: T;
  label: string;
  icon?: React.ComponentType<{ className?: string }>;
}

interface TabsProps<T extends string> {
  value: T;
  items: TabItem<T>[];
  onChange: (value: T) => void;
  orientation?: "horizontal" | "vertical";
  className?: string;
}

export function Tabs<T extends string>({
  value,
  items,
  onChange,
  orientation = "horizontal",
  className,
}: TabsProps<T>) {
  const vertical = orientation === "vertical";
  return (
    <div
      role="tablist"
      aria-orientation={orientation}
      className={cn(
        vertical
          ? "flex flex-col gap-1"
          : "flex gap-1 rounded-lg border border-border bg-surface p-0.5",
        className
      )}
    >
      {items.map((item) => {
        const active = item.value === value;
        const Icon = item.icon;
        return (
          <button
            key={item.value}
            type="button"
            role="tab"
            aria-selected={active}
            onClick={() => onChange(item.value)}
            className={cn(
              "flex items-center gap-2 text-sm font-medium transition active:scale-[0.98]",
              "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20",
              vertical
                ? cn(
                    "w-full rounded-lg px-3 py-2 text-left",
                    active
                      ? "bg-surface text-white shadow-sm shadow-black/20"
                      : "text-zinc-400 hover:bg-white/5 hover:text-white"
                  )
                : cn(
                    "flex-1 justify-center rounded-md px-3 py-1.5 text-xs",
                    active
                      ? "bg-surface-2 text-white"
                      : "text-zinc-400 hover:text-white"
                  )
            )}
          >
            {Icon ? <Icon className="size-3.5 shrink-0" /> : null}
            {item.label}
          </button>
        );
      })}
    </div>
  );
}