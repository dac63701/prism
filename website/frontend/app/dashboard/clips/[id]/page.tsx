import { notFound } from "next/navigation";
import { getClip } from "@/lib/server-api";
import { Card, Panel, Badge } from "@/components/ui";
import { DeleteClipButton } from "@/components/delete-clip-button";
import { ClipRename } from "@/components/clip-rename";
import { ShareButton } from "@/components/share-button";
import VideoPlayer from "@/components/video-player";

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
      <div className="space-y-2">
        <div className="text-xs uppercase tracking-[0.28em] text-blue-300/70">Clip details</div>
        <ClipRename clipId={clip.id} initialTitle={clip.title || "Untitled clip"} />
        <p className="max-w-2xl text-sm leading-6 text-zinc-400">
          Click the title to rename. Use the share button to copy a link or change visibility.
        </p>
      </div>

      <div className="grid gap-6 lg:grid-cols-[1.2fr_0.8fr]">
        <Card className="overflow-hidden p-3">
          {clip.video_url ? (
            <VideoPlayer src={clip.video_url} poster={clip.thumbnail_url ?? undefined} />
          ) : (
            <div className="aspect-video rounded-[1.35rem] bg-black" />
          )}
        </Card>

        <Panel className="space-y-4 p-6">
          <div className="flex items-center gap-2">
            <Badge>{clip.visibility}</Badge>
            <ShareButton clipId={clip.id} shareUrl={clip.share_url} currentVisibility={clip.visibility} />
          </div>
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

          <div className="flex gap-2 pt-2">
            <DeleteClipButton clipId={clip.id} clipTitle={clip.title || "Untitled clip"} redirectTo="/dashboard/clips" />
          </div>
        </Panel>
      </div>
    </div>
  );
}
