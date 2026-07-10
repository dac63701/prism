import Link from "next/link";
import { ShieldCheck, Users, Layers3 } from "lucide-react";
import { getDashboardStats } from "@/lib/server-api";
import { Card, Panel, SectionHeading, StatCard } from "@/components/ui";

export default async function AdminPage() {
  const stats = await getDashboardStats();

  return (
    <div className="mx-auto max-w-7xl space-y-8 px-5 py-8 lg:px-8 lg:py-10">
      <SectionHeading
        eyebrow="Admin"
        title="Server overview"
        description="Basic account and usage data without exposing user videos in the admin area."
      />

      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        <StatCard label="Users" value={String(stats.total_users)} />
        <StatCard label="Clips" value={String(stats.total_clips)} />
        <StatCard label="Storage" value={`${stats.total_storage_gb.toFixed(2)} GB`} />
        <StatCard label="Uploads today" value={String(stats.uploads_today)} />
      </div>

      <div className="grid gap-4 xl:grid-cols-3">
        <Card className="p-5">
          <Users className="h-5 w-5 text-blue-300" />
          <h2 className="mt-4 text-lg font-semibold text-white">Users</h2>
          <p className="mt-2 text-sm text-zinc-400">Search accounts and inspect basic metadata.</p>
          <Link href="/admin/users" className="mt-4 inline-block text-sm text-blue-300 hover:text-blue-200">
            Open user list
          </Link>
        </Card>

        <Card className="p-5">
          <ShieldCheck className="h-5 w-5 text-blue-300" />
          <h2 className="mt-4 text-lg font-semibold text-white">Roles</h2>
          <p className="mt-2 text-sm text-zinc-400">Promote trusted users to admin and manage bans.</p>
        </Card>

        <Card className="p-5">
          <Layers3 className="h-5 w-5 text-blue-300" />
          <h2 className="mt-4 text-lg font-semibold text-white">Data policy</h2>
          <p className="mt-2 text-sm text-zinc-400">No video previews here. Just accounts, stats, and control surfaces.</p>
        </Card>
      </div>
    </div>
  );
}
