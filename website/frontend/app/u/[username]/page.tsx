import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";
import { getProfile } from "@/lib/server-api";
import { Badge, Card, Panel } from "@/components/ui";
import { SiteShell } from "@/components/site-shell";

export async function generateMetadata({ params }: { params: Promise<{ username: string }> }): Promise<Metadata> {
  const { username } = await params;

  try {
    const data = await getProfile(username);
    return {
      title: `${data.user.display_name} • Prism`,
      description: `Public Prism profile for ${data.user.display_name}`,
      openGraph: {
        title: `${data.user.display_name} • Prism`,
        description: `Public Prism profile for ${data.user.display_name}`,
      },
    };
  } catch {
    return { title: "Prism profile" };
  }
}

export default async function ProfilePage({ params }: { params: Promise<{ username: string }> }) {
  const { username } = await params;

  let data;
  try {
    data = await getProfile(username);
  } catch {
    notFound();
  }

  return (
    <SiteShell>
      <div className="mx-auto max-w-7xl px-5 py-16 lg:px-8 lg:py-24">
        <div className="flex flex-col gap-6 lg:flex-row lg:items-end lg:justify-between">
          <div className="space-y-4">
            <Badge>Public profile</Badge>
            <h1 className="text-5xl font-semibold tracking-tight text-white">{data.user.display_name}</h1>
            <p className="max-w-2xl text-zinc-400">A public gallery of approved clips and highlights.</p>
          </div>
          <div className="text-sm text-zinc-500">@{username}</div>
        </div>

        <div className="mt-10 grid gap-4 md:grid-cols-2 xl:grid-cols-3">
          {data.clips.map((clip) => (
            <Link key={clip.id} href={`/s/${clip.share_id}`} className="block">
              <Card className="cursor-pointer overflow-hidden p-3 transition hover:scale-[1.02]">
                <div className="aspect-video overflow-hidden rounded-2xl bg-[#09111f]">
                  {clip.thumbnail_path ? (
                    // eslint-disable-next-line @next/next/no-img-element
                    <img src={`/api/media/${clip.thumbnail_path}`} alt={clip.title} className="h-full w-full object-cover" />
                  ) : null}
                </div>
                <div className="p-3">
                  <div className="text-sm font-medium text-white">{clip.title || "Untitled clip"}</div>
                  <div className="mt-1 text-xs text-zinc-500">{clip.game || "Unknown game"}</div>
                </div>
              </Card>
            </Link>
          ))}
        </div>

        {data.clips.length === 0 ? (
          <Panel className="mt-8 p-8 text-center text-zinc-400">No public clips yet.</Panel>
        ) : null}
      </div>
    </SiteShell>
  );
}
