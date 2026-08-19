import type { LucideIcon } from "lucide-react";
import { cn } from "@/lib/utils";

export function GroupTitle({
  icon: Icon,
  title,
  description,
}: {
  icon: LucideIcon;
  title: string;
  description?: string;
}) {
  return (
    <div className="flex items-start gap-2.5">
      <span className="mt-0.5 flex size-7 shrink-0 items-center justify-center rounded-lg border border-border bg-surface text-zinc-400">
        <Icon className="size-3.5" />
      </span>
      <div className="min-w-0">
        <h3 className="text-sm font-medium text-zinc-100">{title}</h3>
        {description ? <p className="mt-0.5 text-xs text-zinc-500">{description}</p> : null}
      </div>
    </div>
  );
}

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