import { Skeleton, SkeletonSectionHeading, SkeletonDownloadCards } from "@/components/skeleton";
import { SiteShell } from "@/components/site-shell";

export default function DownloadLoading() {
  return (
    <SiteShell>
      <div className="relative mx-auto max-w-6xl px-5 py-16 lg:px-8 lg:py-24">
        <div className="pointer-events-none fixed inset-0 overflow-hidden">
          <div className="absolute -left-40 -top-40 h-[500px] w-[500px] rounded-full bg-blue-500/10 blur-[120px]" />
          <div className="absolute -bottom-40 -right-40 h-[500px] w-[500px] rounded-full bg-blue-600/8 blur-[120px]" />
        </div>

        <div className="relative z-10 text-center">
          <Skeleton className="mx-auto h-3 w-28" />
          <Skeleton className="mx-auto mt-3 h-11 w-44" />
          <Skeleton className="mx-auto mt-3 h-7 w-96" />
        </div>

        <div className="relative z-10 mt-12">
          <SkeletonDownloadCards />
        </div>

        <div className="relative z-10 mx-auto mt-16 max-w-3xl">
          <Skeleton className="h-5 w-28" />
          <div className="mt-4 space-y-4">
            {Array.from({ length: 5 }).map((_, i) => (
              <Skeleton key={i} className="h-4 w-full" />
            ))}
          </div>
        </div>
      </div>
    </SiteShell>
  );
}
