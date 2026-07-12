import { SkeletonSectionHeading, Skeleton, SkeletonCard } from "@/components/skeleton";

export default function ClipsLoading() {
  return (
    <div className="mx-auto max-w-7xl space-y-8 px-5 py-8 lg:px-8 lg:py-10">
      <SkeletonSectionHeading
        eyebrow="Library"
        title="Your clips"
        description="Search, sort, and manage the recordings saved to your Prism account."
      />

      <SkeletonCard className="p-5">
        <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          {Array.from({ length: 6 }).map((_, i) => (
            <div
              key={i}
              className="animate-pulse rounded-2xl border border-[#1f2a44] bg-white/[0.03]"
            >
              <Skeleton className="aspect-video w-full rounded-2xl" />
              <div className="space-y-2 p-4">
                <Skeleton className="h-[14px] w-3/4" />
                <Skeleton className="h-3 w-1/2" />
                <Skeleton className="h-3 w-1/3" />
              </div>
            </div>
          ))}
        </div>
      </SkeletonCard>
    </div>
  );
}
