import { cn } from "@/lib/utils";

export function Card({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn(
        "rounded-3xl border border-border bg-[linear-gradient(180deg,rgba(16,25,46,0.95),rgba(8,13,26,0.95))] shadow-lg shadow-black/20",
        className
      )}
      {...props}
    />
  );
}

export function Panel({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn(
        "rounded-2xl border border-border bg-white/[0.03]",
        className
      )}
      {...props}
    />
  );
}

export function Badge({ className, ...props }: React.HTMLAttributes<HTMLSpanElement>) {
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-full border border-blue-400/20 bg-blue-500/10 px-3 py-1 text-xs font-medium text-blue-200",
        className
      )}
      {...props}
    />
  );
}

export function SectionHeading({
  eyebrow,
  title,
  description,
}: {
  eyebrow?: string;
  title: string;
  description?: string;
}) {
  return (
    <div className="space-y-1">
      {eyebrow ? (
        <span className="text-xs uppercase tracking-[0.28em] text-blue-300/70">
          {eyebrow}
        </span>
      ) : null}
      <h2 className="text-lg font-semibold tracking-tight text-white">{title}</h2>
      {description ? (
        <p className="max-w-2xl text-sm leading-6 text-zinc-400">{description}</p>
      ) : null}
    </div>
  );
}

export function InfoBar({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div
      className={cn(
        "rounded-full border border-border bg-white/[0.03] px-4 py-1.5 text-xs text-zinc-500 flex items-center gap-4",
        className
      )}
      {...props}
    />
  );
}
