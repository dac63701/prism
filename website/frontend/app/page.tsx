import Link from "next/link";
import { ArrowRight, Clapperboard, Lock, Sparkles, Users } from "lucide-react";
import { SiteShell } from "@/components/site-shell";
import { Badge, Button, Card, Panel, SectionHeading, StatCard } from "@/components/ui";


const features = [
  {
    icon: Clapperboard,
    title: "Clip instantly",
    copy: "Save the last moments of a match without slowing the game or juggling files.",
  },
  {
    icon: Users,
    title: "Public profiles",
    copy: "Show off highlights on a clean profile page with a shareable public identity.",
  },
  {
    icon: Lock,
    title: "Private by default",
    copy: "Keep clips in your account until you explicitly publish or share them.",
  },
];

export default function LandingPage() {
  return (
    <SiteShell>
      <section className="mx-auto max-w-7xl px-5 py-16 lg:px-8 lg:py-24">
        <div className="grid gap-10 lg:grid-cols-[1.1fr_0.9fr] lg:items-center">
          <div className="space-y-8">
            <Badge>Capture. Clip. Share.</Badge>
            <div className="space-y-5">
              <h1 className="max-w-3xl text-5xl font-semibold tracking-tight text-white sm:text-6xl">
                Prism — screen recording, clipped and shared instantly.
              </h1>
              <p className="max-w-2xl text-lg leading-8 text-zinc-400">
                Prism is a lightweight screen recorder that captures your best gaming moments
                as instant clips. Built with H.264 hardware encoding for zero-lag capture,
                cloud storage so nothing gets lost, and polished share cards that look great
                everywhere. No file juggling, no huge recordings — just the moments that matter.
              </p>
            </div>
            <div className="flex flex-wrap items-center gap-3">
              <Button asChild>
                <Link href="/login">
                  Go to dashboard
                  <ArrowRight className="h-4 w-4" />
                </Link>
              </Button>
              <Button asChild variant="secondary">
                <Link href="/privacy">Privacy policy</Link>
              </Button>
            </div>
            <div className="grid gap-3 sm:grid-cols-3">
              <StatCard label="Fast share cards" value="OG tags" hint="Social previews and public pages" />
              <StatCard label="User profiles" value="Public" hint="Own your clip identity" />
              <StatCard label="Theme" value="Dark blue" hint="Clean, high-contrast UI" />
            </div>
          </div>

          <Card className="overflow-hidden p-5">
            <div className="space-y-4">
              <div className="flex items-center justify-between rounded-2xl border border-border bg-white/[0.03] px-4 py-3">
                <div>
                  <div className="text-sm text-zinc-400">Shared clip</div>
                  <div className="text-lg font-semibold text-white">Ace round finish</div>
                </div>
                <Badge>Public</Badge>
              </div>
              <div className="aspect-video overflow-hidden rounded-2xl border border-border bg-[linear-gradient(135deg,rgba(14,20,38,0.9),rgba(44,76,144,0.5))]">
                <div className="flex h-full items-end justify-between p-5 text-sm text-white/80">
                  <div className="space-y-1">
                    <div className="text-xs uppercase tracking-[0.3em] text-blue-200/60">Prism share link</div>
                    <div>Clean player, quick load, rich previews.</div>
                  </div>
                  <Sparkles className="h-7 w-7 text-blue-200" />
                </div>
              </div>
              <div className="grid gap-3 sm:grid-cols-2">
                <Panel className="p-4">
                  <div className="text-xs uppercase tracking-[0.25em] text-blue-300/70">Landing</div>
                  <div className="mt-2 text-sm text-zinc-300">A polished front door for the whole service.</div>
                </Panel>
                <Panel className="p-4">
                  <div className="text-xs uppercase tracking-[0.25em] text-blue-300/70">Dashboard</div>
                  <div className="mt-2 text-sm text-zinc-300">Manage clips, settings, and API keys.</div>
                </Panel>
              </div>
            </div>
          </Card>
        </div>
      </section>

      <section className="mx-auto max-w-7xl px-5 pb-16 lg:px-8 lg:pb-24">
        <SectionHeading
          eyebrow="Why Prism"
          title="Built for a larger full-stack product"
          description="The public site, auth flow, dashboard, admin area, public profiles, and share pages all live in one system and share the same API contract."
        />
        <div className="mt-8 grid gap-4 md:grid-cols-3">
          {features.map((feature) => (
            <Panel key={feature.title} className="p-6">
              <feature.icon className="h-5 w-5 text-blue-300" />
              <h3 className="mt-4 text-lg font-semibold text-white">{feature.title}</h3>
              <p className="mt-2 text-sm leading-6 text-zinc-400">{feature.copy}</p>
            </Panel>
          ))}
        </div>
      </section>
    </SiteShell>
  );
}
