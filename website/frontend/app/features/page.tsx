import type { Metadata } from "next";
import {
  Clapperboard,
  Cloud,
  Users,
  Lock,
  Share2,
  Zap,
  Monitor,
  FileVideo,
  Palette,
  ScrollText,
  Layers,
  Globe,
  Download,
} from "lucide-react";
import { SiteShell } from "@/components/site-shell";
import { Badge, Card, Panel, SectionHeading } from "@/components/ui";
import { PrismLogo } from "@/components/brand-icons";

export const metadata: Metadata = {
  title: "Features",
  description:
    "Prism is a lightweight screen recorder with H.264 encoding, cloud storage, public profiles, and polished share cards.",
};

interface Feature {
  icon: React.ComponentType<{ className?: string }>;
  title: string;
  copy: string;
  accent: string;
}

const categories: { title: string; icon: React.ComponentType<{ className?: string }>; features: Feature[] }[] = [
  {
    title: "Recording",
    icon: Clapperboard,
    features: [
      {
        icon: Zap,
        title: "H.264 hardware encoding",
        copy: "Zero-lag capture using GPU hardware encoding. Produces small, high-quality files ready for instant sharing with no re-encode delay.",
        accent: "from-yellow-500/20 to-orange-500/10",
      },
      {
        icon: Monitor,
        title: "Multi-platform capture",
        copy: "Works on Windows and macOS. Windows uses DXGI output duplication with Media Foundation encoding; macOS uses ScreenCaptureKit with VideoToolbox.",
        accent: "from-blue-500/20 to-cyan-500/10",
      },
      {
        icon: Layers,
        title: "Shadow buffer technology",
        copy: "Continuously records into a compressed ring buffer. Save clips after the moment happens — never miss a play again. Holds ~7 minutes at 1080p.",
        accent: "from-purple-500/20 to-pink-500/10",
      },
      {
        icon: Palette,
        title: "Configurable quality",
        copy: "Adjust bitrate and resolution to balance quality and file size. Works at your display's native resolution with configurable encode settings.",
        accent: "from-green-500/20 to-emerald-500/10",
      },
    ],
  },
  {
    title: "Cloud & storage",
    icon: Cloud,
    features: [
      {
        icon: Cloud,
        title: "Cloud storage",
        copy: "Clips are automatically uploaded and stored in the cloud. No local files to manage, no risk of losing your best moments.",
        accent: "from-blue-500/20 to-indigo-500/10",
      },
      {
        icon: FileVideo,
        title: "Thumbnail previews",
        copy: "Each clip gets an auto-generated thumbnail at save time. Browse your library visually at a glance instead of reading file names.",
        accent: "from-amber-500/20 to-orange-500/10",
      },
      {
        icon: ScrollText,
        title: "Clip dashboard",
        copy: "Browse, search, and manage all your clips from a single dashboard. Sort by date, toggle visibility, and copy share links in one click.",
        accent: "from-sky-500/20 to-blue-500/10",
      },
      {
        icon: Download,
        title: "Download originals",
        copy: "Download the original MP4 file for any clip. Use it in edits, share on other platforms, or keep a local backup.",
        accent: "from-teal-500/20 to-green-500/10",
      },
    ],
  },
  {
    title: "Sharing & privacy",
    icon: Share2,
    features: [
      {
        icon: Share2,
        title: "Polished share cards",
        copy: "Every shared clip gets an auto-generated preview card with Open Graph tags, a thumbnail, and a clean player — looks great anywhere you post it.",
        accent: "from-violet-500/20 to-purple-500/10",
      },
      {
        icon: Users,
        title: "Public profiles",
        copy: "Show off highlights on a clean profile page with a shareable public identity. Each profile gets its own URL at goprism.studio/u/username.",
        accent: "from-pink-500/20 to-rose-500/10",
      },
      {
        icon: Lock,
        title: "Private by default",
        copy: "Keep clips in your account until you explicitly publish or share them. Full control over visibility per clip, changeable at any time.",
        accent: "from-red-500/20 to-orange-500/10",
      },
      {
        icon: Globe,
        title: "Open Graph previews",
        copy: "Share links auto-generate rich previews on Discord, X/Twitter, and other platforms. Social previews include clip title and thumbnail.",
        accent: "from-indigo-500/20 to-blue-500/10",
      },
    ],
  },
];

export default function FeaturesPage() {
  return (
    <SiteShell>
      <div className="relative mx-auto max-w-6xl px-5 py-16 lg:px-8 lg:py-24">
        <div className="pointer-events-none fixed inset-0 overflow-hidden">
          <div className="absolute -left-40 -top-40 h-[500px] w-[500px] rounded-full bg-blue-500/10 blur-[120px]" />
          <div className="absolute -bottom-40 -right-40 h-[500px] w-[500px] rounded-full bg-blue-600/8 blur-[120px]" />
        </div>

        <div className="relative z-10 text-center">
          <Badge>Features</Badge>
          <h1 className="mt-4 text-4xl font-semibold tracking-tight text-white sm:text-5xl">
            Everything Prism offers
          </h1>
          <p className="mx-auto mt-3 max-w-2xl text-base leading-7 text-zinc-400">
            A lightweight screen recorder with a cloud-first approach.
            Capture, store, and share your best moments.
          </p>
        </div>

        {categories.map((category) => (
          <section key={category.title} className="relative z-10 mt-20">
            <div className="flex items-center gap-3">
              {(() => {
                const Icon = category.icon;
                return (
                  <div className="flex h-9 w-9 items-center justify-center rounded-xl bg-blue-500/10 text-blue-300">
                    <Icon className="h-5 w-5" />
                  </div>
                );
              })()}
              <h2 className="text-xl font-semibold text-white">{category.title}</h2>
            </div>

            <div className="mt-6 grid gap-4 sm:grid-cols-2">
              {category.features.map((f) => (
                <Panel
                  key={f.title}
                  className="group relative overflow-hidden p-6 transition hover:bg-white/[0.05]"
                >
                  <div
                    className={cn(
                      "absolute inset-0 opacity-0 transition group-hover:opacity-100",
                      f.accent
                    )}
                    style={{ maskImage: "linear-gradient(to bottom right, black, transparent)" }}
                  />
                  <div className="relative z-10">
                    <div className={cn(
                      "flex h-10 w-10 items-center justify-center rounded-xl bg-gradient-to-br shadow-lg",
                      f.accent
                    )}>
                      {(() => {
                        const Icon = f.icon;
                        return <Icon className="h-5 w-5 text-white/80" />;
                      })()}
                    </div>
                    <h3 className="mt-5 text-lg font-semibold text-white">{f.title}</h3>
                    <p className="mt-2 text-sm leading-6 text-zinc-400">{f.copy}</p>
                  </div>
                </Panel>
              ))}
            </div>
          </section>
        ))}

        {/* CTA */}
        <div className="relative z-10 mt-24 text-center">
          <div className="mx-auto max-w-lg rounded-3xl border border-border bg-[linear-gradient(180deg,rgba(16,25,46,0.95),rgba(8,13,26,0.95))] p-8 shadow-lg shadow-black/20">
            <div className="flex justify-center">
              <PrismLogo />
            </div>
            <h2 className="mt-5 text-xl font-semibold text-white">Ready to start?</h2>
            <p className="mt-2 text-sm text-zinc-400">
              Download Prism and start saving your best moments in seconds.
            </p>
            <a
              href="/download"
              className="mt-5 inline-flex items-center gap-2 rounded-xl bg-[linear-gradient(135deg,var(--color-accent),var(--color-accent-2))] px-6 py-2.5 text-sm font-medium text-white shadow-lg shadow-blue-500/20 transition hover:brightness-110"
            >
              Download now
            </a>
          </div>
        </div>
      </div>
    </SiteShell>
  );
}

function cn(...classes: (string | boolean | undefined | null)[]): string {
  return classes.filter(Boolean).join(" ");
}
