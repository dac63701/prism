import { cn } from "@/lib/utils";

interface EmptyStateProps {
  icon?: React.ComponentType<{ className?: string }>;
  title: string;
  description?: string;
  action?: React.ReactNode;
  className?: string;
}

export function EmptyState({
  icon: Icon,
  title,
  description,
  action,
  className,
}: EmptyStateProps) {
  return (
    <div
      className={cn(
        "flex flex-col items-center justify-center gap-2 text-center",
        className
      )}
    >
      {Icon ? <Icon className="size-10 text-zinc-700" /> : null}
      <p className="text-sm font-medium text-zinc-400">{title}</p>
      {description ? (
        <p className="max-w-xs text-xs text-zinc-600">{description}</p>
      ) : null}
      {action ? <div className="mt-2">{action}</div> : null}
    </div>
  );
}