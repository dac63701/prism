import type { Metadata } from "next";
import { SiteShell } from "@/components/site-shell";
import { Card, SectionHeading } from "@/components/ui";

export const metadata: Metadata = {
  title: "Docs",
  description:
    "Learn how to use Prism — from installation and recording to sharing clips and managing your account.",
};

interface DocSection {
  title: string;
  items: { heading: string; body: string }[];
}

const sections: DocSection[] = [
  {
    title: "Getting started",
    items: [
      {
        heading: "Installation",
        body: "Download the Prism installer for Windows or macOS from the dashboard. Run the installer — no additional dependencies are required. On first launch, Prism will request screen recording permissions. macOS users need to grant permission in System Settings > Privacy & Security > Screen Recording.",
      },
      {
        heading: "Creating an account",
        body: "Sign up with your email and a password, or use Google OAuth for a one-click login. Accounts are required to store and manage clips in the cloud. After signing in, you'll land on the dashboard where all your clips are organized.",
      },
      {
        heading: "Default hotkeys",
        body: "By default, Prism uses Ctrl+Shift+S (Windows) or Cmd+Shift+S (macOS) to save a clip. You can customize hotkeys in the Settings panel of the desktop app. Changes take effect immediately.",
      },
    ],
  },
  {
    title: "Recording clips",
    items: [
      {
        heading: "How the shadow buffer works",
        body: "Prism continuously records your screen into a compressed H.264 shadow buffer. When you trigger a clip save, the last N seconds are written to disk and uploaded. This means you never miss a moment — you save clips after they happen, not before. The buffer holds roughly 7 minutes of 1080p video within a 256 MB memory budget.",
      },
      {
        heading: "Adjusting clip duration",
        body: "Open the desktop app settings and adjust the capture duration slider. Longer durations give you more context per clip but use more of the shadow buffer. Changes apply to future clips only.",
      },
      {
        heading: "Resolution and quality",
        body: "Prism captures at your display's native resolution. The encoder targets a configurable bitrate — higher bitrates produce better quality at the cost of larger files. The default is optimized for a good balance of quality and file size.",
      },
    ],
  },
  {
    title: "Sharing",
    items: [
      {
        heading: "Share links",
        body: "Every clip can be shared via a unique URL. Open the clip in your dashboard, click Share, and copy the link. The share page includes a video player, clip metadata, and auto-generated Open Graph preview cards for social media.",
      },
      {
        heading: "Public profiles",
        body: "Your profile at goprism.studio/u/username shows all your publicly shared clips in a grid. Visitors can browse your highlights without needing an account. You can toggle clip visibility individually.",
      },
      {
        heading: "Privacy controls",
        body: "Clips are private by default. Only you can see them in your dashboard until you explicitly mark a clip as public. You can change visibility at any time.",
      },
    ],
  },
  {
    title: "Account & settings",
    items: [
      {
        heading: "Managing your clips",
        body: "The dashboard shows all your clips sorted by date. Each clip has controls to play, rename, toggle visibility, copy share link, or delete. Use the Dashboard link at goprism.studio/dashboard.",
      },
      {
        heading: "Settings",
        body: "Account settings are available at goprism.studio/dashboard/settings. From here you can update your username, display name, and other preferences.",
      },
      {
        heading: "Troubleshooting",
        body: "If clips aren't saving, check that Prism has screen recording permissions. On Windows, ensure your GPU drivers are up to date. On macOS, check that ScreenCaptureKit access is granted in System Settings. For persistent issues, check the app logs or reach out via the support channel.",
      },
    ],
  },
];

export default function DocsPage() {
  return (
    <SiteShell>
      <div className="mx-auto max-w-4xl px-5 py-16 lg:px-8 lg:py-24">
        <SectionHeading
          eyebrow="Documentation"
          title="How Prism works"
          description="Everything you need to know about recording, sharing, and managing your clips."
        />

        {sections.map((section) => (
          <section key={section.title} className="mt-12">
            <h2 className="text-xl font-semibold text-white">{section.title}</h2>
            <div className="mt-4 space-y-4">
              {section.items.map((item) => (
                <Card key={item.heading} className="p-6">
                  <h3 className="text-base font-semibold text-white">{item.heading}</h3>
                  <p className="mt-2 text-sm leading-6 text-zinc-400">{item.body}</p>
                </Card>
              ))}
            </div>
          </section>
        ))}
      </div>
    </SiteShell>
  );
}
