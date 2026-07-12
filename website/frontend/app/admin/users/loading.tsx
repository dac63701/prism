import { SkeletonSectionHeading, SkeletonTable, SkeletonCard } from "@/components/skeleton";

export default function AdminUsersLoading() {
  return (
    <div className="mx-auto max-w-7xl space-y-8 px-5 py-8 lg:px-8 lg:py-10">
      <SkeletonSectionHeading
        eyebrow="Accounts"
        title="Users"
        description="Basic information only. Video content stays out of the admin list."
      />

      <SkeletonCard className="overflow-hidden p-5">
        <SkeletonTable rows={5} />
      </SkeletonCard>
    </div>
  );
}
