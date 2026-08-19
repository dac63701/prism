import { ChevronDown } from "lucide-react";
import { cn } from "@/lib/utils";

export function Select({
  className,
  children,
  ...props
}: React.SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <div className="relative inline-flex">
      <select
        className={cn(
          "appearance-none rounded-xl border border-border bg-surface py-1.5 pl-3 pr-8 text-sm text-white",
          "transition-colors outline-none cursor-pointer",
          "focus:border-blue-400/70 focus:ring-2 focus:ring-blue-500/20",
          "hover:border-border",
          className
        )}
        {...props}
      >
        {children}
      </select>
      <ChevronDown className="pointer-events-none absolute right-2.5 top-1/2 size-3.5 -translate-y-1/2 text-zinc-500" />
    </div>
  );
}