import { Skeleton, SkeletonPanel, SkeletonCard } from "@/components/skeleton";
import { SiteShell } from "@/components/site-shell";

export default function ShareLoading() {
  return (
    <SiteShell>
      <div className="mx-auto max-w-6xl px-5 py-16 lg:px-8 lg:py-24">
        <div className="grid gap-6 lg:grid-cols-[1.3fr_0.7fr]">
          <SkeletonCard className="overflow-hidden p-3">
            <Skeleton className="aspect-video w-full rounded-[1.35rem]" />
          </SkeletonCard>

          <SkeletonPanel className="space-y-4 p-6">
            <Skeleton className="h-6 w-20 rounded-full" />
            <div>
              <Skeleton className="h-8 w-48" />
              <Skeleton className="mt-2 h-4 w-32" />
            </div>
            <div className="grid grid-cols-2 gap-3">
              {Array.from({ length: 4 }).map((_, i) => (
                <SkeletonPanel key={i} className="p-4">
                  <Skeleton className="h-3 w-12" />
                  <Skeleton className="mt-2 h-4 w-20" />
                </SkeletonPanel>
              ))}
            </div>
          </SkeletonPanel>
        </div>
      </div>
    </SiteShell>
  );
}
