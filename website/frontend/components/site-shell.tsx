import Link from "next/link";
import { Button } from "@/components/ui";
import { PrismLogo } from "@/components/brand-icons";

export function SiteShell({ children }: { children: React.ReactNode }) {
  return (
    <div className="min-h-screen">
      <header className="sticky top-0 z-40 border-b border-white/5 bg-[#050816]/95">
        <div className="mx-auto flex max-w-7xl items-center justify-between px-5 py-4 lg:px-8">
          <Link href="/" className="flex items-center gap-3 font-semibold text-white">
            <PrismLogo />
            <span>
              Prism
              <span className="block text-[11px] font-medium tracking-[0.3em] text-blue-300/70">cloud clips</span>
            </span>
          </Link>
          <nav className="hidden items-center gap-2 md:flex">
            <Link href="/features" className="rounded-lg px-3 py-2 text-sm text-zinc-300 transition hover:bg-white/5 hover:text-white">
              Features
            </Link>
            <Link href="/docs" className="rounded-lg px-3 py-2 text-sm text-zinc-300 transition hover:bg-white/5 hover:text-white">
              Docs
            </Link>
            <Link href="/privacy" className="rounded-lg px-3 py-2 text-sm text-zinc-300 transition hover:bg-white/5 hover:text-white">
              Privacy
            </Link>
            <Button asChild>
              <Link href="/login">Login</Link>
            </Button>
          </nav>
        </div>
      </header>
      <main>{children}</main>
    </div>
  );
}
