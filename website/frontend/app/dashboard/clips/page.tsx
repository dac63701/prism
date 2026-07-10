import Link from "next/link";
import { listClips } from "@/lib/server-api";
import { Card, Panel, SectionHeading } from "@/components/ui";

export default async function ClipsPage() {
  const clips = await listClips();

  return (
    <div className="mx-auto max-w-7xl space-y-8 px-5 py-8 lg:px-8 lg:py-10">
      <SectionHeading
        eyebrow="Library"
        title="Your clips"
        description="Search, sort, and manage the recordings saved to your Prism account."
      />

      <Card className="p-5">
        <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          {clips.clips.map((clip) => (
            <Panel key={clip.id} className="overflow-hidden">
              <div className="aspect-video bg-[#09111f]">
                {clip.thumbnail_path ? (
                  // eslint-disable-next-line @next/next/no-img-element
                  <img src={`/api/media/${clip.thumbnail_path}`} alt={clip.title} className="h-full w-full object-cover" />
                ) : null}
              </div>
              <div className="space-y-2 p-4">
                <div>
                  <div className="text-sm font-medium text-white">{clip.title || "Untitled"}</div>
                  <div className="text-xs text-zinc-500">{clip.game || "Unknown game"}</div>
                </div>
                <div className="text-xs text-zinc-400">{Math.round(clip.duration_secs)}s • {clip.visibility}</div>
                <Link href={`/dashboard/clips/${clip.id}`} className="text-sm text-blue-300 hover:text-blue-200">
                  Open details
                </Link>
              </div>
            </Panel>
          ))}
        </div>
        {clips.clips.length === 0 ? <div className="text-sm text-zinc-500">No clips uploaded yet.</div> : null}
      </Card>
    </div>
  );
}
