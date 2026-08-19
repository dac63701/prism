import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";
import { ArrowLeft, BookOpen } from "lucide-react";
import { SiteShell } from "@/components/site-shell";
import { MobileDocsNav } from "@/components/mobile-docs-nav";
import { SectionHeading } from "@/components/ui";
import { guides, getGuide, type GuideBlock } from "@/lib/docs-guides";

interface PageProps {
  params: Promise<{ slug: string }>;
}

export function generateStaticParams() {
  return guides.map((guide) => ({ slug: guide.slug }));
}

export async function generateMetadata({ params }: PageProps): Promise<Metadata> {
  const { slug } = await params;
  const guide = getGuide(slug);
  if (!guide) return { title: "Guide not found" };
  return {
    title: guide.title,
    description: guide.description,
  };
}

function renderBlock(block: GuideBlock, index: number) {
  switch (block.kind) {
    case "p":
      return (
        <p key={index} className="text-sm leading-7 text-zinc-400">
          {block.text}
        </p>
      );
    case "ul":
      return (
        <ul key={index} className="space-y-2">
          {block.items.map((item, i) => (
            <li key={i} className="flex items-start gap-2.5 text-sm leading-6 text-zinc-400">
              <span className="mt-2 h-1.5 w-1.5 shrink-0 rounded-full bg-blue-400/60" />
              <span>{item}</span>
            </li>
          ))}
        </ul>
      );
    case "ol":
      return (
        <ol key={index} className="space-y-2">
          {block.items.map((item, i) => (
            <li key={i} className="flex items-start gap-3 text-sm leading-6 text-zinc-400">
              <span className="flex h-5 w-5 shrink-0 items-center justify-center rounded-md bg-blue-500/10 text-[11px] font-semibold text-blue-300">
                {i + 1}
              </span>
              <span>{item}</span>
            </li>
          ))}
        </ol>
      );
    case "code":
      return (
        <div key={index} className="rounded-xl border border-border bg-[#09111f] p-4">
          {block.label && (
            <p className="mb-2 text-[11px] font-medium uppercase tracking-wider text-zinc-500">
              {block.label}
            </p>
          )}
          <pre className="overflow-x-auto text-[13px] leading-6 text-zinc-200">
            <code>{block.text}</code>
          </pre>
        </div>
      );
    case "note":
      return (
        <div
          key={index}
          className="flex items-start gap-3 rounded-xl border border-blue-400/20 bg-blue-500/10 p-4"
        >
          <span className="mt-0.5 text-blue-300">
            <BookOpen className="h-4 w-4" />
          </span>
          <p className="text-sm leading-6 text-blue-100/80">{block.text}</p>
        </div>
      );
  }
}

export default async function DocsGuidePage({ params }: PageProps) {
  const { slug } = await params;
  const guide = getGuide(slug);
  if (!guide) notFound();

  const Icon = guide.icon;

  return (
    <SiteShell>
      <div className="mx-auto flex max-w-6xl gap-10 px-5 py-16 lg:px-8 lg:py-24">
        {/* Sidebar — other guides */}
        <nav className="sticky top-24 hidden h-fit w-56 shrink-0 space-y-1 lg:block">
          <a
            href="/docs"
            className="flex items-center gap-2 rounded-lg px-3 py-2 text-sm font-medium text-zinc-300 transition hover:bg-white/5 hover:text-white"
          >
            <ArrowLeft className="h-4 w-4 text-blue-300" />
            All docs
          </a>
          <div className="my-3 h-px bg-border" />
          {guides.map((item) => {
            const ItemIcon = item.icon;
            const active = item.slug === slug;
            return (
              <a
                key={item.slug}
                href={`/docs/${item.slug}`}
                className={
                  active
                    ? "flex items-center gap-2 rounded-lg bg-white/5 px-3 py-2 text-sm font-medium text-white"
                    : "flex items-center gap-2 rounded-lg px-3 py-2 text-sm text-zinc-400 transition hover:bg-white/5 hover:text-white"
                }
              >
                <ItemIcon className="h-4 w-4 text-blue-300" />
                <span className="truncate">{item.title}</span>
              </a>
            );
          })}
        </nav>

        {/* Article */}
        <article className="min-w-0 flex-1">
          <Link
            href="/docs"
            className="mb-6 inline-flex items-center gap-2 text-sm text-zinc-500 transition hover:text-zinc-200 lg:hidden"
          >
            <ArrowLeft className="h-4 w-4" />
            All docs
          </Link>

          <div className="mb-8 flex items-start gap-4">
            <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-2xl bg-blue-500/10 text-blue-300">
              <Icon className="h-6 w-6" />
            </div>
            <div className="min-w-0">
              <SectionHeading eyebrow="Prism Wiki" title={guide.title} />
              <p className="mt-3 max-w-2xl text-sm leading-6 text-zinc-400">
                {guide.description}
              </p>
            </div>
          </div>

          {guide.sections.map((section) => (
            <section key={section.id} id={section.id} className="mb-12 scroll-mt-24">
              <h2 className="text-lg font-semibold text-white">{section.heading}</h2>
              <div className="mt-4 space-y-4">
                {section.blocks.map(renderBlock)}
              </div>
            </section>
          ))}

          <div className="flex flex-wrap gap-3 border-t border-border pt-8">
            {guides
              .filter((item) => item.slug !== slug)
              .slice(0, 2)
              .map((item) => {
                const ItemIcon = item.icon;
                return (
                  <Link
                    key={item.slug}
                    href={`/docs/${item.slug}`}
                    className="flex flex-1 items-center gap-3 rounded-2xl border border-border bg-white/[0.03] p-4 transition hover:bg-white/[0.06] hover:border-blue-400/30 min-w-0"
                  >
                    <ItemIcon className="h-5 w-5 shrink-0 text-blue-300" />
                    <span className="min-w-0">
                      <span className="block truncate text-sm font-medium text-zinc-200">
                        {item.title}
                      </span>
                      <span className="mt-0.5 block truncate text-xs text-zinc-500">
                        {item.description}
                      </span>
                    </span>
                  </Link>
                );
              })}
          </div>
        </article>

        <MobileDocsNav
          sections={[
            {
              id: "guide",
              title: guide.title,
              items: guide.sections.map((section) => ({
                id: section.id,
                heading: section.heading,
              })),
            },
          ]}
        />
      </div>
    </SiteShell>
  );
}