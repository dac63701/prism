import type { Metadata } from "next";
import { SiteShell } from "@/components/site-shell";
import { DownloadCard } from "@/components/download-card";

export const metadata: Metadata = {
  title: "Download",
  description: "Download Prism for Windows, macOS, or Linux. Clip-based screen recording for gamers.",
};

interface GithubAsset {
  name: string;
  browser_download_url: string;
  size: number;
}

interface GithubRelease {
  tag_name: string;
  name: string;
  body: string;
  published_at: string;
  assets: GithubAsset[];
}

async function getLatestRelease(): Promise<GithubRelease | null> {
  try {
    const res = await fetch(
      "https://api.github.com/repos/dac63701/prism/releases/latest",
      {
        next: { revalidate: 3600 },
        headers: { Accept: "application/vnd.github.v3+json" },
      }
    );
    if (!res.ok) return null;
    return res.json();
  } catch {
    return null;
  }
}

function platformFromAsset(name: string): "windows" | "macos" | "linux" | null {
  const lower = name.toLowerCase();
  if (lower.includes("windows") || lower.endsWith(".msi") || lower.endsWith(".exe")) return "windows";
  if (lower.includes("macos") || lower.includes("darwin") || lower.endsWith(".dmg")) return "macos";
  if (lower.includes("linux") || lower.includes("ubuntu") || lower.endsWith(".appimage") || lower.endsWith(".deb")) return "linux";
  return null;
}

export default async function DownloadPage() {
  const release = await getLatestRelease();

  const platformAssets = {
    windows: [] as GithubAsset[],
    macos: [] as GithubAsset[],
    linux: [] as GithubAsset[],
  };

  if (release) {
    for (const asset of release.assets) {
      const platform = platformFromAsset(asset.name);
      if (platform) platformAssets[platform].push(asset);
    }
  }

  return (
    <SiteShell>
      <div className="relative mx-auto max-w-6xl px-5 py-16 lg:px-8 lg:py-24">
        <div className="pointer-events-none fixed inset-0 overflow-hidden">
          <div className="absolute -left-40 -top-40 h-[500px] w-[500px] rounded-full bg-blue-500/10 blur-[120px]" />
          <div className="absolute -bottom-40 -right-40 h-[500px] w-[500px] rounded-full bg-blue-600/8 blur-[120px]" />
        </div>

        <div className="relative z-10 text-center">
          <div className="text-xs uppercase tracking-[0.3em] text-blue-300/70">
            {release ? `Version ${release.tag_name}` : "Download"}
          </div>
          <h1 className="mt-3 text-4xl font-semibold tracking-tight text-white sm:text-5xl">
            Get Prism
          </h1>
          <p className="mx-auto mt-3 max-w-xl text-base leading-7 text-zinc-400">
            Download the desktop app for your platform — clip, store, and share your best
            gaming moments.
          </p>
        </div>

        <div className="relative z-10 mt-12">
          <DownloadCard platforms={platformAssets} release={release} />
        </div>

        {release?.body ? (
          <div className="relative z-10 mx-auto mt-16 max-w-3xl">
            <h2 className="text-lg font-semibold text-white">What&rsquo;s new</h2>
            <div className="mt-4 space-y-4 text-sm leading-6 text-zinc-400">
              {release.body.split("\n").map((line, i) => (
                <p key={i}>{line || <br />}</p>
              ))}
            </div>
          </div>
        ) : null}

        {!release ? (
          <div className="relative z-10 mx-auto mt-8 max-w-md text-center">
            <div className="rounded-2xl border border-white/10 bg-white/[0.03] px-6 py-5 text-sm text-zinc-400">
              No releases published yet. Check back soon or
              {" "}
              <a
                href="https://github.com/dac63701/prism"
                className="text-blue-300 hover:text-blue-200"
              >
                build from source
              </a>.
            </div>
          </div>
        ) : null}
      </div>
    </SiteShell>
  );
}
