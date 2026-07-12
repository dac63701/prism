"use client";

import Link from "next/link";
import { Panel } from "@/components/ui";
import { DeleteClipButton } from "@/components/delete-clip-button";
import type { ClipListItem } from "@/lib/types";

export function ClipsGrid({ clips }: { clips: ClipListItem[] }) {
  return (
    <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
      {clips.map((clip) => (
        <Panel key={clip.id} className="group relative overflow-hidden">
          <DeleteClipButton
            clipId={clip.id}
            clipTitle={clip.title || "Untitled clip"}
            compact
          />
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
  );
}
