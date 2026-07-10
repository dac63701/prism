import { notFound } from "next/navigation";
import { Card, Panel, SectionHeading } from "@/components/ui";
import { getAdminUser } from "@/lib/server-api";
import type { AdminUserDetail } from "@/lib/types";

export default async function AdminUserDetailPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  let data: AdminUserDetail;
  try {
    data = await getAdminUser(id);
  } catch {
    notFound();
  }

  return (
    <div className="mx-auto max-w-5xl space-y-8 px-5 py-8 lg:px-8 lg:py-10">
      <SectionHeading
        eyebrow="Account details"
        title={data.display_name}
        description="Basic account metrics only. No clip playback in admin."
      />

      <Card className="space-y-4 p-6">
        <div className="grid gap-4 md:grid-cols-2">
          <Panel className="p-4">
            <div className="text-xs uppercase tracking-[0.25em] text-blue-300/70">Email</div>
            <div className="mt-2 text-sm text-white">{data.email}</div>
          </Panel>
          <Panel className="p-4">
            <div className="text-xs uppercase tracking-[0.25em] text-blue-300/70">Role</div>
            <div className="mt-2 text-sm text-white">{data.role}</div>
          </Panel>
          <Panel className="p-4">
            <div className="text-xs uppercase tracking-[0.25em] text-blue-300/70">Clips</div>
            <div className="mt-2 text-sm text-white">{data.clip_count}</div>
          </Panel>
          <Panel className="p-4">
            <div className="text-xs uppercase tracking-[0.25em] text-blue-300/70">Storage used</div>
            <div className="mt-2 text-sm text-white">{Math.round(data.storage_used_bytes / 1_048_576)} MB</div>
          </Panel>
        </div>
      </Card>
    </div>
  );
}
