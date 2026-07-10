import type { Metadata } from "next";
import Link from "next/link";
import "./globals.css";

export const metadata: Metadata = {
  title: {
    default: "Prism",
    template: "%s • Prism",
  },
  description: "Clip, store, and share game moments with a polished cloud dashboard.",
  metadataBase: new URL(process.env.SITE_URL ?? "http://localhost:3000"),
  keywords: ["game clips", "cloud clips", "game highlights", "video sharing"],
  openGraph: {
    type: "website",
    siteName: "Prism",
    title: "Prism",
    description: "Clip, store, and share game moments with a polished cloud dashboard.",
  },
  twitter: {
    card: "summary_large_image",
    title: "Prism",
    description: "Clip, store, and share game moments with a polished cloud dashboard.",
  },
  robots: {
    index: true,
    follow: true,
  },
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body>
        {children}
        <footer className="border-t border-white/5 py-8 text-center text-xs text-zinc-500">
          <div className="mx-auto flex max-w-7xl items-center justify-between gap-4 px-5 lg:px-8">
            <span>Prism</span>
            <div className="flex items-center gap-4">
              <Link href="/privacy" className="hover:text-zinc-300">
                Privacy
              </Link>
              <Link href="/login" className="hover:text-zinc-300">
                Login
              </Link>
            </div>
          </div>
        </footer>
      </body>
    </html>
  );
}
