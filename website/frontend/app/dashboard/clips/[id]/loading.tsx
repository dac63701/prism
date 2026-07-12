import { SkeletonSectionHeading, Skeleton, SkeletonPanel, SkeletonCard } from "@/components/skeleton";

export default function ClipDetailLoading() {
  return (
    <div className="mx-auto max-w-6xl space-y-8 px-5 py-8 lg:px-8 lg:py-10">
      <SkeletonSectionHeading
        eyebrow="Clip details"
        title="Loading clip..."
        description="Edit metadata later; this view gives you the essentials and preview."
      />

      <div className="grid gap-6 lg:grid-cols-[1.2fr_0.8fr]">
        <SkeletonCard className="overflow-hidden p-3">
          <Skeleton className="aspect-video w-full rounded-[1.35rem]" />
        </SkeletonCard>

        <SkeletonPanel className="space-y-4 p-6">
          <Skeleton className="h-6 w-20 rounded-full" />
          <div className="space-y-1">
            <Skeleton className="h-3 w-12" />
            <Skeleton className="h-4 w-32" />
          </div>
          <div className="space-y-1">
            <Skeleton className="h-3 w-16" />
            <Skeleton className="h-4 w-48" />
          </div>
          <div className="space-y-1">
            <Skeleton className="h-3 w-14" />
            <Skeleton className="h-4 w-20" />
          </div>
          <div className="space-y-1">
            <Skeleton className="h-3 w-18" />
            <Skeleton className="h-4 w-24" />
          </div>
          <div className="pt-2">
            <Skeleton className="h-10 w-28 rounded-xl" />
          </div>
        </SkeletonPanel>
      </div>
    </div>
  );
}
