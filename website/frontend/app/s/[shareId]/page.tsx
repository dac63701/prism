import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { getShareMeta } from "@/lib/server-api";
import { Card, Badge, Panel } from "@/components/ui";
import { SiteShell } from "@/components/site-shell";
import VideoPlayer from "@/components/video-player";

export async function generateMetadata({ params }: { params: Promise<{ shareId: string }> }): Promise<Metadata> {
  const { shareId } = await params;

  try {
    const data = await getShareMeta(shareId);
    const title = data.clip.title || "Prism clip";
    const description = `${data.user.display_name} · ${data.clip.game || "Unknown game"} · ${Math.round(data.clip.duration_secs)}s`;

    return {
      title,
      description,
      openGraph: {
        title,
        description,
        images: data.clip.thumbnail_url ? [data.clip.thumbnail_url] : undefined,
        type: "video.other",
      },
    };
  } catch {
    return { title: "Prism clip" };
  }
}

export default async function SharePage({ params }: { params: Promise<{ shareId: string }> }) {
  const { shareId } = await params;

  let data;
  try {
    data = await getShareMeta(shareId);
  } catch {
    notFound();
  }

  const clip = data.clip;
  const title = clip.title || "Untitled clip";

  return (
    <SiteShell>
      <div className="mx-auto max-w-6xl px-5 py-16 lg:px-8 lg:py-24">
        <div className="grid gap-6 lg:grid-cols-[1.3fr_0.7fr]">
          <Card className="overflow-hidden p-3">
            {clip.video_url ? (
              <VideoPlayer src={clip.video_url} poster={clip.thumbnail_url ?? undefined} />
            ) : (
              <div className="aspect-video rounded-[1.35rem] bg-black" />
            )}
          </Card>

          <Panel className="space-y-4 p-6">
            <Badge>{clip.visibility}</Badge>
            <div>
              <h1 className="text-3xl font-semibold tracking-tight text-white">{title}</h1>
              <p className="mt-2 text-sm text-zinc-400">{clip.game || "Unknown game"}</p>
            </div>
            <div className="grid grid-cols-2 gap-3 text-sm text-zinc-300">
              <div className="rounded-2xl border border-border bg-white/[0.03] p-4">
                <div className="text-zinc-500">Creator</div>
                <div className="mt-1 font-medium text-white">{data.user.display_name}</div>
              </div>
              <div className="rounded-2xl border border-border bg-white/[0.03] p-4">
                <div className="text-zinc-500">Duration</div>
                <div className="mt-1 font-medium text-white">{Math.round(clip.duration_secs)}s</div>
              </div>
              <div className="rounded-2xl border border-border bg-white/[0.03] p-4">
                <div className="text-zinc-500">Resolution</div>
                <div className="mt-1 font-medium text-white">{clip.width} × {clip.height}</div>
              </div>
              <div className="rounded-2xl border border-border bg-white/[0.03] p-4">
                <div className="text-zinc-500">Views</div>
                <div className="mt-1 font-medium text-white">Share link</div>
              </div>
            </div>
          </Panel>
        </div>
      </div>
    </SiteShell>
  );
}
