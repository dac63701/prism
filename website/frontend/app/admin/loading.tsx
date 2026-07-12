import { SkeletonSectionHeading, SkeletonStatCard, SkeletonPanel, Skeleton } from "@/components/skeleton";

export default function AdminLoading() {
  return (
    <div className="mx-auto max-w-7xl space-y-8 px-5 py-8 lg:px-8 lg:py-10">
      <SkeletonSectionHeading
        eyebrow="Admin"
        title="Server overview"
        description="Basic account and usage data without exposing user videos in the admin area."
      />

      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        {Array.from({ length: 4 }).map((_, i) => (
          <SkeletonStatCard key={i} />
        ))}
      </div>

      <div className="grid gap-4 xl:grid-cols-3">
        {Array.from({ length: 3 }).map((_, i) => (
          <div
            key={i}
            className="animate-pulse rounded-3xl border border-[#1f2a44] bg-[linear-gradient(180deg,rgba(16,25,46,0.95),rgba(8,13,26,0.95))] p-5"
          >
            <Skeleton className="h-5 w-5 rounded" />
            <Skeleton className="mt-4 h-7 w-16" />
            <Skeleton className="mt-2 h-5 w-48" />
            <Skeleton className="mt-4 h-5 w-24" />
          </div>
        ))}
      </div>
    </div>
  );
}
