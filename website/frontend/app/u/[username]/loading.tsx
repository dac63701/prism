import { Skeleton, SkeletonPanel } from "@/components/skeleton";
import { SiteShell } from "@/components/site-shell";

export default function ProfileLoading() {
  return (
    <SiteShell>
      <div className="mx-auto max-w-7xl px-5 py-16 lg:px-8 lg:py-24">
        <div className="flex flex-col gap-6 lg:flex-row lg:items-end lg:justify-between">
          <div className="space-y-4">
            <Skeleton className="h-6 w-28 rounded-full" />
            <Skeleton className="h-12 w-64" />
            <Skeleton className="h-4 w-80" />
          </div>
          <Skeleton className="h-4 w-24" />
        </div>

        <div className="mt-10 grid gap-4 md:grid-cols-2 xl:grid-cols-3">
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
      </div>
    </SiteShell>
  );
}
