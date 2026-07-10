import { Card, Input, SectionHeading } from "@/components/ui";

export default function SettingsPage() {
  return (
    <div className="mx-auto max-w-4xl space-y-8 px-5 py-8 lg:px-8 lg:py-10">
      <SectionHeading
        eyebrow="Settings"
        title="Account settings"
        description="Manage your password, API keys, and desktop integration here."
      />

      <Card className="space-y-6 p-6">
        <div>
          <h2 className="text-lg font-semibold text-white">Change password</h2>
          <p className="mt-1 text-sm text-zinc-400">Use this if you created an email/password account or linked Google later.</p>
        </div>
        <div className="grid gap-4 md:grid-cols-2">
          <Input type="password" placeholder="Current password" />
          <Input type="password" placeholder="New password" />
        </div>
      </Card>

      <Card className="space-y-4 p-6">
        <h2 className="text-lg font-semibold text-white">API keys</h2>
        <p className="text-sm text-zinc-400">Create a key for the desktop app to upload clips to the cloud.</p>
        <div className="rounded-2xl border border-dashed border-white/10 bg-white/[0.03] p-5 text-sm text-zinc-500">
          API key management will appear here.
        </div>
      </Card>
    </div>
  );
}
