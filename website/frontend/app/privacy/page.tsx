import { SiteShell } from "@/components/site-shell";
import { Card, SectionHeading } from "@/components/ui";

export default function PrivacyPage() {
  return (
    <SiteShell>
      <div className="mx-auto max-w-4xl px-5 py-16 lg:px-8 lg:py-24">
        <SectionHeading
          eyebrow="Legal"
          title="Privacy Policy"
          description="This is a working draft. Replace it with your final legal copy before launch."
        />
        <Card className="mt-8 space-y-6 p-6 text-sm leading-7 text-zinc-300">
          <p>
            Prism stores account data, uploaded clips, profile metadata, and usage logs needed to run the service.
          </p>
          <p>
            Google login is used to create and verify accounts. Email/password accounts are stored securely with
            password hashing. Clip files remain private until you choose to share them.
          </p>
          <p>
            Shared clip pages and public profiles may expose clip titles, thumbnails, and limited metadata. Admin
            users can see account-level information but not private clip playback in the admin dashboard.
          </p>
        </Card>
      </div>
    </SiteShell>
  );
}
