import { SiteShell } from "@/components/site-shell";
import { Card, SectionHeading } from "@/components/ui";

const sections = [
  {
    title: "What we collect",
    body: "Prism collects account information (email, display name, avatar) when you sign up via Google OAuth or email. We store the clips you upload, along with metadata such as titles, timestamps, tags, and view counts. Server logs capture IP addresses, browser/user-agent strings, and request timestamps for operational purposes.",
  },
  {
    title: "How we use it",
    body: "Account data is used to authenticate you and personalise your experience. Clip storage enables sharing and playback. Metadata powers features like clip listing, search, and analytics. Server logs are used for debugging, rate-limiting, and abuse prevention.",
  },
  {
    title: "Clip visibility",
    body: "Clips are private by default. Only you can see them until you explicitly share a clip or make your profile public. Shared clips are accessible via a unique URL. If you make your profile public, your shared clips and display name are visible to anyone who visits your profile page.",
  },
  {
    title: "Third-party services",
    body: "We use Google OAuth for optional social login — no other data is shared with Google beyond the scope you approve during sign-in. Cloud storage is hosted on infrastructure managed by Prism. We do not sell your data to advertisers or third parties.",
  },
  {
    title: "Data retention",
    body: "Your account and clips are retained until you delete them. You may delete individual clips or your entire account at any time from the dashboard. Server logs are retained for up to 90 days. Deleted clips are removed from active storage within 24 hours; cached or backup copies are purged within 30 days.",
  },
  {
    title: "Security",
    body: "Passwords are hashed with bcrypt. All traffic is served over TLS. Clip access is authorised per-request. Administrative access to the database is restricted to the core team and logged. We perform regular dependency audits and follow best practices for session management.",
  },
  {
    title: "Changes to this policy",
    body: "We may update this policy as the service evolves. Material changes will be announced via the dashboard or a notice on the website. Continued use after an update constitutes acceptance of the revised policy.",
  },
  {
    title: "Contact",
    body: "If you have questions about this policy or want to request data deletion, file an issue at the Prism GitHub repository or reach out through the dashboard support channel.",
  },
];

export default function PrivacyPage() {
  return (
    <SiteShell>
      <div className="mx-auto max-w-4xl px-5 py-16 lg:px-8 lg:py-24">
        <SectionHeading
          eyebrow="Legal"
          title="Privacy Policy"
          description="Last updated July 2026"
        />
        <Card className="mt-8 space-y-6 p-6 text-sm leading-7 text-zinc-300">
          <p className="text-zinc-400">
            This policy describes how Prism handles your information when you use the website, desktop app, and
            related services.
          </p>
          {sections.map((s) => (
            <div key={s.title}>
              <h2 className="mb-2 text-base font-semibold text-white">{s.title}</h2>
              <p>{s.body}</p>
            </div>
          ))}
        </Card>
      </div>
    </SiteShell>
  );
}
