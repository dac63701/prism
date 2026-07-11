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
} from "lucide-react";
import { SiteShell } from "@/components/site-shell";
import { Badge, Card, Panel, SectionHeading } from "@/components/ui";

export const metadata: Metadata = {
  title: "Features",
  description:
    "Prism is a lightweight screen recorder with H.264 encoding, cloud storage, public profiles, and polished share cards.",
};

interface Feature {
  icon: React.ComponentType<{ className?: string }>;
  title: string;
  copy: string;
}

const features: Feature[] = [
  {
    icon: Clapperboard,
    title: "Clip instantly",
    copy: "Save the last moments of a match without slowing the game or juggling files. Configurable clip duration and hotkey triggers.",
  },
  {
    icon: Cloud,
    title: "Cloud storage",
    copy: "Clips are automatically uploaded and stored in the cloud. No local files to manage, no risk of losing your best moments.",
  },
  {
    icon: Users,
    title: "Public profiles",
    copy: "Show off highlights on a clean profile page with a shareable public identity. Each profile gets its own URL at goprism.studio/u/username.",
  },
  {
    icon: Lock,
    title: "Private by default",
    copy: "Keep clips in your account until you explicitly publish or share them. Full control over visibility per clip.",
  },
  {
    icon: Share2,
    title: "Polished share cards",
    copy: "Every shared clip gets an auto-generated preview card with Open Graph tags, a thumbnail, and a clean player — looks great anywhere you post it.",
  },
  {
    icon: Zap,
    title: "H.264 hardware encoding",
    copy: "Zero-lag capture using GPU hardware encoding. Produces small, high-quality files ready for instant sharing with no re-encode delay.",
  },
  {
    icon: Monitor,
    title: "Multi-platform",
    copy: "Works on Windows and macOS. Windows uses DXGI output duplication with Media Foundation encoding; macOS uses ScreenCaptureKit with VideoToolbox.",
  },
  {
    icon: FileVideo,
    title: "Thumbnail previews",
    copy: "Each clip gets an auto-generated thumbnail at save time. Browse your library visually at a glance instead of reading file names.",
  },
  {
    icon: Palette,
    title: "Dark theme",
    copy: "A clean, dark blue UI designed for long sessions. High-contrast text, subtle gradients, and a consistent visual language throughout.",
  },
  {
    icon: ScrollText,
    title: "Clip dashboard",
    copy: "Browse, search, and manage all your clips from a single dashboard. Sort by date, toggle visibility, and copy share links in one click.",
  },
];

interface FaqItem {
  q: string;
  a: string;
}

const faqs: FaqItem[] = [
  {
    q: "What clip formats does Prism support?",
    a: "Prism records H.264 video in MP4 containers. This gives broad compatibility across browsers, social platforms, and media players while keeping file sizes small.",
  },
  {
    q: "Is there a clip length limit?",
    a: "The shadow buffer holds roughly 7 minutes of compressed H.264 video at 1080p with its 256 MB budget. You can adjust the buffer size and clip duration in settings.",
  },
  {
    q: "Can I use Prism on multiple computers?",
    a: "Your clips are tied to your account, not your device. Sign in from any supported machine and your full library is available immediately.",
  },
];

export default function FeaturesPage() {
  return (
    <SiteShell>
      <div className="mx-auto max-w-6xl px-5 py-16 lg:px-8 lg:py-24">
        <SectionHeading
          eyebrow="Features"
          title="Everything Prism offers"
          description="A lightweight screen recorder with a cloud-first approach. Capture, store, and share your best moments."
        />

        <div className="mt-10 grid gap-4 md:grid-cols-2">
          {features.map((f) => (
            <Panel key={f.title} className="p-6">
              <f.icon className="h-5 w-5 text-blue-300" />
              <h3 className="mt-4 text-lg font-semibold text-white">{f.title}</h3>
              <p className="mt-2 text-sm leading-6 text-zinc-400">{f.copy}</p>
            </Panel>
          ))}
        </div>

        <div className="mt-20">
          <SectionHeading
            eyebrow="FAQ"
            title="Frequently asked questions"
            description="Quick answers to common questions about how Prism works."
          />

          <div className="mt-8 space-y-4">
            {faqs.map((faq) => (
              <Card key={faq.q} className="p-6">
                <h3 className="text-base font-semibold text-white">{faq.q}</h3>
                <p className="mt-2 text-sm leading-6 text-zinc-400">{faq.a}</p>
              </Card>
            ))}
          </div>
        </div>
      </div>
    </SiteShell>
  );
}
