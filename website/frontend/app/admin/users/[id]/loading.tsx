import { SkeletonSectionHeading, SkeletonUserDetail, SkeletonCard } from "@/components/skeleton";

export default function AdminUserDetailLoading() {
  return (
    <div className="mx-auto max-w-5xl space-y-8 px-5 py-8 lg:px-8 lg:py-10">
      <SkeletonSectionHeading
        eyebrow="Account details"
        title="Loading user..."
        description="Basic account metrics only. No clip playback in admin."
      />

      <SkeletonCard className="p-6">
        <SkeletonUserDetail />
      </SkeletonCard>
    </div>
  );
}
