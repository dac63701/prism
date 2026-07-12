import { SkeletonSectionHeading, Skeleton, SkeletonPanel } from "@/components/skeleton";

export default function ClipsLoading() {
  return (
    <div className="mx-auto max-w-7xl space-y-8 px-5 py-8 lg:px-8 lg:py-10">
      <SkeletonSectionHeading
        eyebrow="Library"
        title="Your clips"
        description="Search, sort, and manage the recordings saved to your Prism account."
      />

      <SkeletonPanel className="p-5">
        <div className="grid gap-6 md:grid-cols-2 xl:grid-cols-3">
          {Array.from({ length: 6 }).map((_, i) => (
            <div
              key={i}
              className="animate-pulse rounded-3xl border border-[#1f2a44] bg-[linear-gradient(180deg,rgba(16,25,46,0.95),rgba(8,13,26,0.95))] p-3"
            >
              <Skeleton className="aspect-video w-full rounded-2xl" />
              <div className="space-y-2 p-3">
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
