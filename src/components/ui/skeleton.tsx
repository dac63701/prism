import { cn } from "@/lib/utils";

export function Skeleton({ className }: { className?: string }) {
  return (
    <div
      className={cn(
        "rounded-lg bg-[linear-gradient(90deg,rgba(31,42,68,0.4)_25%,rgba(79,140,255,0.15)_50%,rgba(31,42,68,0.4)_75%)]",
        "animate-shimmer bg-[length:200%_100%]",
        className
      )}
    />
  );
}

export function SkeletonClipCard() {
  return (
    <div className="aspect-video overflow-hidden rounded-2xl border border-border bg-surface">
      <Skeleton className="h-full w-full rounded-none" />
    </div>
  );
}

export function SkeletonClipsGrid({ count = 6 }: { count?: number }) {
  return (
    <div className="grid gap-4 grid-cols-[repeat(auto-fill,minmax(220px,1fr))]">
      {Array.from({ length: count }).map((_, i) => (
        <SkeletonClipCard key={i} />
      ))}
    </div>
  );
}