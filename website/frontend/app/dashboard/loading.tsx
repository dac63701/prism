import { SkeletonSectionHeading, SkeletonStatCard, SkeletonCard, Skeleton } from "@/components/skeleton";

export default function DashboardLoading() {
  return (
    <div className="mx-auto max-w-7xl space-y-8 px-5 py-8 lg:px-8 lg:py-10">
      <SkeletonSectionHeading
        eyebrow="Dashboard"
        title="Welcome back"
        description="Your clips, account, and storage usage at a glance."
      />

      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        {Array.from({ length: 4 }).map((_, i) => (
          <SkeletonStatCard key={i} />
        ))}
      </div>

      <div className="grid gap-6 xl:grid-cols-[1.2fr_0.8fr]">
        <SkeletonCard className="p-5">
          <div className="flex items-center justify-between">
            <div>
              <Skeleton className="h-3 w-20" />
              <Skeleton className="mt-1 h-5 w-28" />
            </div>
            <Skeleton className="h-4 w-16" />
          </div>
          <div className="mt-5 grid gap-4 md:grid-cols-2">
            {Array.from({ length: 4 }).map((_, i) => (
              <div key={i} className="rounded-2xl border border-[#1f2a44] bg-white/[0.03]">
                <Skeleton className="aspect-video w-full rounded-2xl" />
                <div className="space-y-2 p-4">
                  <Skeleton className="h-[14px] w-3/4" />
                  <Skeleton className="h-3 w-1/2" />
                </div>
              </div>
            ))}
          </div>
        </SkeletonCard>

        <div className="space-y-4">
          <SkeletonCard className="p-5">
            <div className="flex items-center gap-3">
              <Skeleton className="h-5 w-5 rounded" />
              <div className="space-y-1">
                <Skeleton className="h-[14px] w-20" />
                <Skeleton className="h-[18px] w-24" />
              </div>
            </div>
            <div className="mt-4 space-y-3">
              {Array.from({ length: 3 }).map((_, i) => (
                <Skeleton key={i} className="h-[52px] w-full rounded-2xl" />
              ))}
            </div>
          </SkeletonCard>

          <SkeletonCard className="p-5">
            <div className="flex items-center gap-3">
              <Skeleton className="h-5 w-5 rounded" />
              <div className="space-y-1">
                <Skeleton className="h-[14px] w-20" />
                <Skeleton className="h-[18px] w-16" />
              </div>
            </div>
            <Skeleton className="mt-4 h-[20px] w-48" />
          </SkeletonCard>
        </div>
      </div>
    </div>
  );
}
