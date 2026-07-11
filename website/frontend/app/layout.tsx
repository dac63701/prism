import type { Metadata } from "next";
import Link from "next/link";
import "./globals.css";

export const metadata: Metadata = {
  title: {
    default: "Prism — Clip-Based Screen Recording & Cloud Sharing",
    template: "%s • Prism",
  },
  description:
    "Prism captures your best gaming moments as instant clips. A lightweight screen recorder with H.264 encoding, cloud storage, public profiles, and polished share cards.",
  metadataBase: new URL(process.env.SITE_URL ?? "http://localhost:3000"),
  keywords: [
    "Prism screen recorder",
    "clip capture tool",
    "screen recording software",
    "game clip sharing",
    "cloud clips",
    "lightweight recorder",
    "H.264 screen capture",
  ],
  openGraph: {
    type: "website",
    siteName: "Prism",
    title: "Prism — Clip-Based Screen Recording & Cloud Sharing",
    description:
      "Prism captures your best gaming moments as instant clips. A lightweight screen recorder with H.264 encoding, cloud storage, public profiles, and polished share cards.",
    images: [{ url: "/og-image.jpg", width: 1200, height: 630 }],
  },
  twitter: {
    card: "summary_large_image",
    title: "Prism — Clip-Based Screen Recording & Cloud Sharing",
    description:
      "Prism captures your best gaming moments as instant clips. A lightweight screen recorder with H.264 encoding, cloud storage, public profiles, and polished share cards.",
    images: ["/og-image.jpg"],
  },
  icons: {
    icon: [
      { url: "/favicon.ico", sizes: "any" },
      { url: "/brand/logo.svg", type: "image/svg+xml" },
    ],
  },
  robots: {
    index: true,
    follow: true,
  },
};

const jsonLd = {
  "@context": "https://schema.org",
  "@type": "SoftwareApplication",
  name: "Prism",
  applicationCategory: "MultimediaApplication",
  operatingSystem: "Windows, macOS",
  description:
    "Clip-based screen recording tool with cloud sharing. Captures, stores, and shares game moments as instant clips with H.264 encoding.",
  offers: {
    "@type": "Offer",
    price: "0",
    priceCurrency: "USD",
  },
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en">
      <body className="antialiased">
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{ __html: JSON.stringify(jsonLd) }}
        />
        {children}
        <footer className="border-t border-white/5 py-8 text-center text-xs text-zinc-500">
          <div className="mx-auto flex max-w-7xl items-center justify-between gap-4 px-5 lg:px-8">
            <span>Prism</span>
            <div className="flex items-center gap-4">
              <Link href="/features" className="hover:text-zinc-300">
                Features
              </Link>
              <Link href="/docs" className="hover:text-zinc-300">
                Docs
              </Link>
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
