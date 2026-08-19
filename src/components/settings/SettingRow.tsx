import { cn } from "@/lib/utils";

export function SettingRow({
  label,
  help,
  children,
  className,
}: {
  label: string;
  help?: string;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("flex items-center justify-between gap-4 py-2", className)}>
      <div className="min-w-0">
        <label className="block text-sm text-zinc-400">{label}</label>
        {help ? <p className="mt-0.5 text-xs text-zinc-600">{help}</p> : null}
      </div>
      <div className="flex min-w-0 flex-1 items-center justify-end gap-2">{children}</div>
    </div>
  );
}

export function SettingCard({
  children,
  className,
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div
      className={cn(
        "rounded-2xl border border-border bg-surface/70 p-4",
        className
      )}
    >
      {children}
    </div>
  );
}

export function SectionHeading({ children }: { children: React.ReactNode }) {
  return (
    <h2 className="text-lg font-semibold tracking-tight text-white">
      {children}
    </h2>
  );
}