import { cn } from "@/lib/utils";

export function Kbd({
  children,
  className,
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <kbd
      className={cn(
        "inline-flex min-w-6 items-center justify-center rounded-md border border-border bg-surface-2",
        "px-1.5 py-0.5 font-mono text-[10px] font-medium text-zinc-300",
        className
      )}
    >
      {children}
    </kbd>
  );
}