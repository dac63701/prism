import Link from "next/link";
import { listClips } from "@/lib/server-api";
import { Card, SectionHeading } from "@/components/ui";
import { ClipsGrid } from "@/components/clips-grid";

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
        <ClipsGrid clips={clips.clips} />
        {clips.clips.length === 0 ? <div className="text-sm text-zinc-500">No clips uploaded yet.</div> : null}
      </Card>
    </div>
  );
}
