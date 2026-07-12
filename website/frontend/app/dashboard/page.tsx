import Link from "next/link";
import { Activity, Sparkles } from "lucide-react";
import { currentUser } from "@/lib/server";
import { listClips } from "@/lib/server-api";
import { Card, Panel, SectionHeading, StatCard } from "@/components/ui";

export default async function DashboardPage() {
  const [user, clipsData] = await Promise.all([currentUser(), listClips()]);
  if (!user) return null;

  const storageUsedGb = user.storage_used_bytes / 1_073_741_824;
  const storageMaxGb = user.max_storage_bytes / 1_073_741_824;

  return (
    <div className="mx-auto max-w-7xl space-y-8 px-5 py-8 lg:px-8 lg:py-10">
      <SectionHeading
        eyebrow="Dashboard"
        title="Welcome back"
        description="Your clips, account, and storage usage at a glance."
      />

      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        <StatCard label="Your clips" value={String(clipsData.total)} hint="All your uploaded clips" />
        <StatCard label="Storage used" value={`${storageUsedGb.toFixed(2)} GB`} hint="Total storage consumed" />
        <StatCard label="Storage capacity" value={`${storageMaxGb.toFixed(2)} GB`} hint="Your max storage limit" />
        <StatCard label="Free space" value={`${(storageMaxGb - storageUsedGb).toFixed(2)} GB`} hint="Remaining upload capacity" />
      </div>

      <div className="grid gap-6 xl:grid-cols-[1.2fr_0.8fr]">
        <Card className="p-5">
          <div className="flex items-center justify-between">
            <div>
              <div className="text-xs uppercase tracking-[0.25em] text-blue-300/70">Recent clips</div>
              <h2 className="mt-1 text-xl font-semibold text-white">Latest uploads</h2>
            </div>
            <Link href="/dashboard/clips" className="text-sm text-blue-300 hover:text-blue-200">
              View all
            </Link>
          </div>
          <div className="mt-5 grid gap-4 md:grid-cols-2">
            {clipsData.clips.slice(0, 4).map((clip) => (
              <Panel key={clip.id} className="overflow-hidden">
                <div className="aspect-video bg-[#09111f]">
                  {clip.thumbnail_path ? (
                    // eslint-disable-next-line @next/next/no-img-element
                    <img src={`/api/media/${clip.thumbnail_path}`} alt={clip.title} className="h-full w-full object-cover" />
                  ) : null}
                </div>
                <div className="p-4">
                  <div className="text-sm font-medium text-white">{clip.title || "Untitled"}</div>
                  <div className="mt-1 text-xs text-zinc-500">{clip.game || "Unknown game"}</div>
                </div>
              </Panel>
            ))}
            {clipsData.clips.length === 0 ? <div className="text-sm text-zinc-500">No clips yet.</div> : null}
          </div>
        </Card>

        <div className="space-y-4">
          <Card className="p-5">
            <div className="flex items-center gap-3">
              <Sparkles className="h-5 w-5 text-blue-300" />
              <div>
                <div className="text-sm text-zinc-400">Quick actions</div>
                <div className="text-lg font-semibold text-white">Next steps</div>
              </div>
            </div>
            <div className="mt-4 space-y-3 text-sm text-zinc-300">
              <div className="rounded-2xl border border-border bg-white/[0.03] p-4">Upload a clip and add a title/game.</div>
              <div className="rounded-2xl border border-border bg-white/[0.03] p-4">Generate an API key for the desktop app.</div>
              <div className="rounded-2xl border border-border bg-white/[0.03] p-4">Publish a share link or public profile.</div>
            </div>
          </Card>

          <Card className="p-5">
            <div className="flex items-center gap-3">
              <Activity className="h-5 w-5 text-blue-300" />
              <div>
                <div className="text-sm text-zinc-400">Storage</div>
                <div className="text-lg font-semibold text-white">Usage</div>
              </div>
            </div>
            <div className="mt-4 text-sm text-zinc-300">
              {storageUsedGb.toFixed(2)} GB of {storageMaxGb.toFixed(2)} GB used.
            </div>
          </Card>
        </div>
      </div>
    </div>
  );
}
