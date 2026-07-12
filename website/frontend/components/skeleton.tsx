import React from "react";
import { cn } from "@/lib/utils";

export function Skeleton({ className }: { className?: string }) {
  return (
    <div
      className={cn(
        "animate-pulse rounded-xl bg-[#10192e]",
        className
      )}
    />
  );
}

export function SkeletonCard({ className, children }: { className?: string; children?: React.ReactNode }) {
  if (children) {
    return (
      <div
        className={cn(
          "animate-pulse rounded-3xl border border-[#1f2a44] bg-[linear-gradient(180deg,rgba(16,25,46,0.95),rgba(8,13,26,0.95))]",
          className
        )}
      >
        {children}
      </div>
    );
  }

  return (
    <div
      className={cn(
        "animate-pulse rounded-3xl border border-[#1f2a44] bg-[linear-gradient(180deg,rgba(16,25,46,0.95),rgba(8,13,26,0.95))]",
        className
      )}
    >
      <Skeleton className="h-full w-full rounded-3xl bg-white/[0.03]" />
    </div>
  );
}

export function SkeletonPanel({ className, children }: { className?: string; children?: React.ReactNode }) {
  return (
    <div
      className={cn(
        "animate-pulse rounded-2xl border border-[#1f2a44] bg-white/[0.03]",
        className
      )}
    >
      {children}
    </div>
  );
}

export function SkeletonStatCard({ className }: { className?: string }) {
  return (
    <SkeletonPanel className={cn("p-5", className)}>
      <Skeleton className="h-3 w-20" />
      <Skeleton className="mt-3 h-7 w-24" />
      <Skeleton className="mt-2 h-3 w-32" />
    </SkeletonPanel>
  );
}

export function SkeletonSectionHeading({
  className,
  eyebrow: _eyebrow,
  title: _title,
  description: _description,
}: {
  className?: string;
  eyebrow?: string;
  title?: string;
  description?: string;
}) {
  return (
    <div className={cn("space-y-2", className)}>
      <Skeleton className="h-3 w-16" />
      <Skeleton className="h-7 w-48" />
      <Skeleton className="h-4 w-80" />
    </div>
  );
}

export function SkeletonVideoPlayer({ className }: { className?: string }) {
  return (
    <SkeletonPanel className={cn("aspect-video w-full", className)} />
  );
}

export function SkeletonClipsGrid({ count = 6, className }: { count?: number; className?: string }) {
  return (
    <div className={cn("grid gap-4 md:grid-cols-2 xl:grid-cols-3", className)}>
      {Array.from({ length: count }).map((_, i) => (
        <div key={i} className="animate-pulse rounded-3xl border border-[#1f2a44] bg-[linear-gradient(180deg,rgba(16,25,46,0.95),rgba(8,13,26,0.95))] p-3">
          <Skeleton className="aspect-video w-full rounded-2xl" />
          <Skeleton className="mt-3 h-4 w-3/4" />
          <Skeleton className="mt-2 h-3 w-1/2" />
        </div>
      ))}
    </div>
  );
}

export function SkeletonTable({ rows = 5, className }: { rows?: number; className?: string }) {
  return (
    <div className={cn("space-y-3", className)}>
      {Array.from({ length: rows }).map((_, i) => (
        <SkeletonPanel key={i} className="flex items-center justify-between gap-4 p-4">
          <div className="space-y-2">
            <Skeleton className="h-4 w-40" />
            <Skeleton className="h-3 w-56" />
          </div>
          <div className="flex items-center gap-4">
            <Skeleton className="h-3 w-12" />
            <Skeleton className="h-3 w-10" />
            <Skeleton className="h-3 w-12" />
          </div>
        </SkeletonPanel>
      ))}
    </div>
  );
}

export function SkeletonUserDetail({ className }: { className?: string }) {
  return (
    <div className={cn("grid gap-4 md:grid-cols-2", className)}>
      {Array.from({ length: 4 }).map((_, i) => (
        <SkeletonPanel key={i} className="p-4">
          <Skeleton className="h-3 w-12" />
          <Skeleton className="mt-2 h-4 w-32" />
        </SkeletonPanel>
      ))}
    </div>
  );
}

export function SkeletonDashboardClips({ className }: { className?: string }) {
  return (
    <div className={cn("space-y-4", className)}>
      <SkeletonPanel className="p-5">
        <div className="grid gap-4 md:grid-cols-2">
          {Array.from({ length: 4 }).map((_, i) => (
            <div key={i} className="animate-pulse rounded-2xl border border-[#1f2a44] bg-white/[0.03]">
              <Skeleton className="aspect-video w-full rounded-2xl" />
              <div className="p-4 space-y-2">
                <Skeleton className="h-4 w-3/4" />
                <Skeleton className="h-3 w-1/2" />
              </div>
            </div>
          ))}
        </div>
      </SkeletonPanel>
    </div>
  );
}

export function SkeletonDownloadCards({ className }: { className?: string }) {
  return (
    <div className={cn("grid gap-6 md:grid-cols-3", className)}>
      {Array.from({ length: 3 }).map((_, i) => (
        <div key={i} className="animate-pulse rounded-3xl border border-[#1f2a44] bg-[linear-gradient(180deg,rgba(16,25,46,0.95),rgba(8,13,26,0.95))] p-6">
          <Skeleton className="mx-auto h-12 w-12 rounded-xl" />
          <Skeleton className="mx-auto mt-4 h-5 w-20" />
          <Skeleton className="mx-auto mt-2 h-3 w-32" />
          <Skeleton className="mx-auto mt-6 h-10 w-36 rounded-xl" />
          <Skeleton className="mx-auto mt-3 h-3 w-28" />
        </div>
      ))}
    </div>
  );
}
