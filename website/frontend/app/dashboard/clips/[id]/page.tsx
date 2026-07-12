import { notFound } from "next/navigation";
import { getClip } from "@/lib/server-api";
import { Card, Panel, SectionHeading, Badge } from "@/components/ui";
import { DeleteClipButton } from "@/components/delete-clip-button";

export default async function ClipDetailPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;

  let clip;
  try {
    clip = await getClip(id);
  } catch {
    notFound();
  }

  return (
    <div className="mx-auto max-w-6xl space-y-8 px-5 py-8 lg:px-8 lg:py-10">
      <SectionHeading
        eyebrow="Clip details"
        title={clip.title || "Untitled clip"}
        description="Edit metadata later; this view gives you the essentials and preview."
      />

      <div className="grid gap-6 lg:grid-cols-[1.2fr_0.8fr]">
        <Card className="overflow-hidden p-3">
          <div className="aspect-video overflow-hidden rounded-[1.35rem] bg-black">
            {clip.video_url ? (
              <video controls playsInline poster={clip.thumbnail_url ?? undefined} className="h-full w-full object-cover">
                <source src={clip.video_url} />
              </video>
            ) : null}
          </div>
        </Card>

        <Panel className="space-y-4 p-6">
          <Badge>{clip.visibility}</Badge>
          <div className="space-y-1">
            <div className="text-sm text-zinc-400">Game</div>
            <div className="text-white">{clip.game || "Unknown"}</div>
          </div>
          <div className="space-y-1">
            <div className="text-sm text-zinc-400">Filename</div>
            <div className="text-white">{clip.original_filename}</div>
          </div>
          <div className="space-y-1">
            <div className="text-sm text-zinc-400">Duration</div>
            <div className="text-white">{Math.round(clip.duration_secs)} seconds</div>
          </div>
          <div className="space-y-1">
            <div className="text-sm text-zinc-400">Resolution</div>
            <div className="text-white">{clip.width} × {clip.height}</div>
          </div>

          <div className="pt-2">
            <DeleteClipButton clipId={clip.id} clipTitle={clip.title || "Untitled clip"} redirectTo="/dashboard/clips" />
          </div>
        </Panel>
      </div>
    </div>
  );
}
