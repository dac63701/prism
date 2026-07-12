"use client";

import { useEffect, useState } from "react";
import { Monitor, Apple, Laptop, ChevronDown } from "lucide-react";
import { cn } from "@/lib/utils";
import { Panel } from "@/components/ui";

interface Asset {
  name: string;
  browser_download_url: string;
  size: number;
}

interface Release {
  tag_name: string;
  name: string;
  published_at: string;
}

type Platform = "windows" | "macos" | "linux";

const PLATFORM_INFO: Record<Platform, { label: string; icon: React.ComponentType<{ className?: string }>; gradient: string; badge: string }> = {
  windows: {
    label: "Windows",
    icon: Monitor,
    gradient: "from-blue-500/20 to-blue-600/10",
    badge: "Download for Windows",
  },
  macos: {
    label: "macOS",
    icon: Apple,
    gradient: "from-purple-500/20 to-purple-600/10",
    badge: "Download for macOS",
  },
  linux: {
    label: "Linux",
    icon: Laptop,
    gradient: "from-zinc-500/20 to-zinc-600/10",
    badge: "Download for Linux",
  },
};

function detectPlatform(): Platform {
  if (typeof navigator === "undefined") return "windows";
  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes("mac") || ua.includes("darwin")) return "macos";
  if (ua.includes("linux")) return "linux";
  return "windows";
}

function pickPrimaryAsset(assets: Asset[], platform: Platform): Asset | null {
  const preferred = assets.find((a) => {
    const n = a.name.toLowerCase();
    if (platform === "windows") return n.includes("setup") || n.endsWith(".msi") || n.endsWith(".exe");
    if (platform === "macos") return n.endsWith(".dmg");
    return n.endsWith(".appimage");
  });
  return preferred ?? assets[0] ?? null;
}

function formatBytes(bytes: number): string {
  if (bytes < 1_000_000) return `${(bytes / 1000).toFixed(0)} KB`;
  return `${(bytes / 1_000_000).toFixed(1)} MB`;
}

function Step({ n, children }: { n: string; children: React.ReactNode }) {
  return (
    <div className="flex items-start gap-3">
      <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded-full bg-blue-500/15 text-xs font-medium text-blue-200">
        {n}
      </span>
      <div className="text-sm text-zinc-300">{children}</div>
    </div>
  );
}

export function DownloadCard({
  platforms,
  release,
}: {
  platforms: Record<Platform, Asset[]>;
  release: Release | null;
}) {
  const [detected, setDetected] = useState<Platform>("windows");
  const [selected, setSelected] = useState<Platform>("windows");

  useEffect(() => {
    const p = detectPlatform();
    setDetected(p);
    setSelected(p);
  }, []);

  const currentAssets = platforms[selected];
  const primaryAsset = pickPrimaryAsset(currentAssets, selected);

  return (
    <div className="mx-auto max-w-4xl">
      {/* Platform tabs */}
      <div className="mb-8 flex justify-center gap-2">
        {(Object.entries(PLATFORM_INFO) as [Platform, typeof PLATFORM_INFO[Platform]][]).map(
          ([key, info]) => {
            const Icon = info.icon;
            const isDetected = key === detected;
            const isSelected = key === selected;
            return (
              <button
                key={key}
                onClick={() => setSelected(key)}
                className={cn(
                  "flex items-center gap-2 rounded-xl px-5 py-3 text-sm font-medium transition",
                  isSelected
                    ? "bg-blue-500/15 text-blue-200 ring-1 ring-blue-400/30"
                    : "text-zinc-400 hover:bg-white/5 hover:text-zinc-200"
                )}
              >
                <Icon className="h-4 w-4" />
                {info.label}
                {isDetected ? (
                  <span className="rounded bg-green-500/15 px-1.5 py-0.5 text-[10px] text-green-300">
                    Detected
                  </span>
                ) : null}
              </button>
            );
          }
        )}
      </div>

      {/* Main download card */}
      <Panel className={cn("overflow-hidden border p-8 transition", platforms[selected].length > 0 ? "border-blue-400/20" : "border-white/10")}>
        <div key={selected} className="animate-fade-up space-y-8">
          {primaryAsset ? (
            <div className="flex flex-col items-center gap-6 text-center">
              <div className={cn(
                "flex h-20 w-20 items-center justify-center rounded-3xl bg-gradient-to-br shadow-lg",
                PLATFORM_INFO[selected].gradient
              )}>
                {(() => {
                  const Icon = PLATFORM_INFO[selected].icon;
                  return <Icon className="h-9 w-9 text-white/80" />;
                })()}
              </div>
              <div>
                <h2 className="text-2xl font-semibold text-white">
                  {PLATFORM_INFO[selected].label}
                </h2>
                <p className="mt-1 text-sm text-zinc-400">
                  {release ? `Version ${release.tag_name}` : "Latest"} &middot;{" "}
                  {formatBytes(primaryAsset.size)}
                </p>
              </div>
              <a
                href={primaryAsset.browser_download_url}
                className={cn(
                  "inline-flex items-center gap-2 rounded-xl px-8 py-3 text-sm font-medium text-white shadow-lg transition",
                  "bg-[linear-gradient(135deg,var(--color-accent),var(--color-accent-2))]",
                  "hover:brightness-110 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-400"
                )}
              >
                {PLATFORM_INFO[selected].badge}
                <ChevronDown className="h-4 w-4" />
              </a>
              <div className="flex flex-wrap justify-center gap-2">
                {currentAssets.map((asset) => (
                  <a
                    key={asset.name}
                    href={asset.browser_download_url}
                    className="rounded-lg border border-white/10 bg-white/[0.03] px-3 py-1.5 text-xs text-zinc-400 transition hover:bg-white/10 hover:text-zinc-200"
                  >
                    {asset.name} ({formatBytes(asset.size)})
                  </a>
                ))}
              </div>
            </div>
          ) : (
            <div className="py-8 text-center">
              <p className="text-sm text-zinc-500">
                No builds available for {PLATFORM_INFO[selected].label} yet.
              </p>
            </div>
          )}

          {platforms[selected].length > 0 && (
            <>
              <hr className="border-white/10" />
              <div className="space-y-5">
                <div className="text-xs uppercase tracking-[0.28em] text-blue-300/70">Installation Guide</div>
                <div className="space-y-4">
                  {selected === "windows" && (
                    <>
                      <Step n="1">Download the <span className="text-white">.msi</span> or <span className="text-white">.exe</span> installer above.</Step>
                      <Step n="2">Run the installer and follow the setup wizard.</Step>
                      <Step n="3">Launch Prism from the Start Menu.</Step>
                      <pre className="overflow-x-auto rounded-lg border border-white/10 bg-black/30 p-3 text-sm text-zinc-300 font-mono">winget install prism</pre>
                    </>
                  )}
                  {selected === "macos" && (
                    <>
                      <Step n="1">Open the downloaded <span className="text-white">.dmg</span> file.</Step>
                      <Step n="2">Drag Prism into the <span className="text-white">Applications</span> folder.</Step>
                      <Step n="3">Right-click Prism and select <span className="text-white">Open</span> to bypass Gatekeeper on first launch.</Step>
                      <pre className="overflow-x-auto rounded-lg border border-white/10 bg-black/30 p-3 text-sm text-zinc-300 font-mono">brew install --cask prism</pre>
                    </>
                  )}
                  {selected === "linux" && (
                    <>
                      <Step n="1">Download the <span className="text-white">.AppImage</span> file above.</Step>
                      <Step n="2">Make it executable in your terminal.</Step>
                      <Step n="3">Run Prism directly.</Step>
                      <pre className="overflow-x-auto rounded-lg border border-white/10 bg-black/30 p-3 text-sm text-zinc-300 font-mono">
{`chmod +x Prism-*.AppImage
./Prism-*.AppImage`}
                      </pre>
                    </>
                  )}
                </div>
                <p className="text-xs text-zinc-500">
                  Alternatively, choose a different package above.
                </p>
              </div>
            </>
          )}
        </div>
      </Panel>
    </div>
  );
}
