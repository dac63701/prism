import type { Metadata } from "next";
import { SiteShell } from "@/components/site-shell";
import { Card, Panel, SectionHeading } from "@/components/ui";
import { MobileDocsNav } from "@/components/mobile-docs-nav";
import {
  Rocket,
  Clapperboard,
  Share2,
  Settings,
  BookOpen,
  Monitor,
  Key,
  Lock,
  HelpCircle,
} from "lucide-react";

export const metadata: Metadata = {
  title: "Docs",
  description:
    "Learn how to use Prism — from installation and recording to sharing clips and managing your account.",
};

interface DocItem {
  id: string;
  heading: string;
  body: string;
  icon: React.ComponentType<{ className?: string }>;
}

interface DocSection {
  id: string;
  title: string;
  icon: React.ComponentType<{ className?: string }>;
  items: DocItem[];
}

const sections: DocSection[] = [
  {
    id: "getting-started",
    title: "Getting started",
    icon: Rocket,
    items: [
      {
        id: "installation",
        heading: "Installation",
        icon: Monitor,
        body: "Download the Prism installer for Windows or macOS from the dashboard. Run the installer — no additional dependencies are required. On first launch, Prism will request screen recording permissions. macOS users need to grant permission in System Settings > Privacy & Security > Screen Recording.",
      },
      {
        id: "creating-account",
        heading: "Creating an account",
        icon: Key,
        body: "Sign up with your email and a password, or use Google OAuth for a one-click login. Accounts are required to store and manage clips in the cloud. After signing in, you'll land on the dashboard where all your clips are organized.",
      },
      {
        id: "default-hotkeys",
        heading: "Default hotkeys",
        icon: Settings,
        body: "By default, Prism uses Ctrl+Shift+S (Windows) or Cmd+Shift+S (macOS) to save a clip. You can customize hotkeys in the Settings panel of the desktop app. Changes take effect immediately — no restart needed.",
      },
    ],
  },
  {
    id: "recording",
    title: "Recording clips",
    icon: Clapperboard,
    items: [
      {
        id: "shadow-buffer",
        heading: "How the shadow buffer works",
        icon: Clapperboard,
        body: "Prism continuously records your screen into a compressed H.264 shadow buffer. When you trigger a clip save, the last N seconds are written to disk and uploaded. This means you never miss a moment — you save clips after they happen, not before. The buffer holds roughly 7 minutes of 1080p video within a 256 MB memory budget.",
      },
      {
        id: "adjusting-duration",
        heading: "Adjusting clip duration",
        icon: Settings,
        body: "Open the desktop app settings and adjust the capture duration slider. Longer durations give you more context per clip but use more of the shadow buffer. Changes apply to future clips only.",
      },
      {
        id: "resolution-quality",
        heading: "Resolution and quality",
        icon: Monitor,
        body: "Prism captures at your display's native resolution. The encoder targets a configurable bitrate — higher bitrates produce better quality at the cost of larger files. The default is optimized for a good balance of quality and file size.",
      },
    ],
  },
  {
    id: "sharing",
    title: "Sharing",
    icon: Share2,
    items: [
      {
        id: "share-links",
        heading: "Share links",
        icon: Share2,
        body: "Every clip can be shared via a unique URL. Open the clip in your dashboard, click Share, and copy the link. The share page includes a video player, clip metadata, and auto-generated Open Graph preview cards for social media sites like Discord and X/Twitter.",
      },
      {
        id: "public-profiles",
        heading: "Public profiles",
        icon: BookOpen,
        body: "Your profile at goprism.studio/u/username shows all your publicly shared clips in a grid. Visitors can browse your highlights without needing an account. You can toggle clip visibility individually from the dashboard.",
      },
      {
        id: "privacy-controls",
        heading: "Privacy controls",
        icon: Lock,
        body: "Clips are private by default. Only you can see them in your dashboard until you explicitly mark a clip as public. You can change visibility at any time with one click.",
      },
    ],
  },
  {
    id: "account-settings",
    title: "Account & settings",
    icon: Settings,
    items: [
      {
        id: "managing-clips",
        heading: "Managing your clips",
        icon: Clapperboard,
        body: "The dashboard shows all your clips sorted by date. Each clip has controls to play, rename, toggle visibility, copy share link, or delete. Visit goprism.studio/dashboard to manage your library.",
      },
      {
        id: "settings",
        heading: "Settings",
        icon: Settings,
        body: "Account settings are available at goprism.studio/dashboard/settings. From here you can update your username, display name, and manage your API keys for programmatic access.",
      },
      {
        id: "troubleshooting",
        heading: "Troubleshooting",
        icon: HelpCircle,
        body: "If clips aren't saving, check that Prism has screen recording permissions. On Windows, ensure your GPU drivers are up to date. On macOS, check that ScreenCaptureKit access is granted in System Settings. For persistent issues, check the app logs via the tray menu or reach out through the support channel.",
      },
    ],
  },
];

export default function DocsPage() {
  return (
    <SiteShell>
      <div className="mx-auto flex max-w-6xl gap-10 px-5 py-16 lg:px-8 lg:py-24">
        {/* Sidebar navigation */}
        <nav className="sticky top-24 hidden h-fit w-56 shrink-0 space-y-1 lg:block">
          {sections.map((section) => (
            <div key={section.id}>
              <a
                href={`#${section.id}`}
                className="flex items-center gap-2 rounded-lg px-3 py-2 text-sm font-medium text-zinc-300 transition hover:bg-white/5 hover:text-white"
              >
                {(() => {
                  const Icon = section.icon;
                  return <Icon className="h-4 w-4 text-blue-300" />;
                })()}
                {section.title}
              </a>
              <div className="ml-6 space-y-0.5">
                {section.items.map((item) => (
                  <a
                    key={item.id}
                    href={`#${item.id}`}
                    className="block rounded-lg px-3 py-1.5 text-xs text-zinc-500 transition hover:bg-white/5 hover:text-zinc-300"
                  >
                    {item.heading}
                  </a>
                ))}
              </div>
            </div>
          ))}
        </nav>

        {/* Content */}
        <div className="min-w-0 flex-1">
          <SectionHeading
            eyebrow="Documentation"
            title="How Prism works"
            description="Everything you need to know about recording, sharing, and managing your clips."
          />

          {sections.map((section) => (
            <section key={section.id} id={section.id} className="mt-14 scroll-mt-24">
              <div className="flex items-center gap-3">
                {(() => {
                  const Icon = section.icon;
                  return (
                    <div className="flex h-9 w-9 items-center justify-center rounded-xl bg-blue-500/10 text-blue-300">
                      <Icon className="h-5 w-5" />
                    </div>
                  );
                })()}
                <h2 className="text-xl font-semibold text-white">{section.title}</h2>
              </div>
              <div className="mt-4 grid gap-4">
                {section.items.map((item) => (
                  <Card key={item.id} id={item.id} className="scroll-mt-24 p-6">
                    <div className="flex items-start gap-3">
                      {(() => {
                        const Icon = item.icon;
                        return (
                          <div className="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-white/[0.04] text-zinc-400">
                            <Icon className="h-4 w-4" />
                          </div>
                        );
                      })()}
                      <div className="min-w-0">
                        <h3 className="text-base font-semibold text-white">
                          {item.heading}
                        </h3>
                        <p className="mt-2 text-sm leading-6 text-zinc-400">
                          {item.body}
                        </p>
                      </div>
                    </div>
                  </Card>
                ))}
              </div>
            </section>
          ))}
        </div>

        <MobileDocsNav sections={sections.map(s => ({ id: s.id, title: s.title, items: s.items.map(i => ({ id: i.id, heading: i.heading })) }))} />
      </div>
    </SiteShell>
  );
}
