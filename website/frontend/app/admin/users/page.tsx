import Link from "next/link";
import { listAdminUsers } from "@/lib/server-api";
import { Card, Panel, SectionHeading } from "@/components/ui";
import { DeleteUserButton } from "@/components/delete-user-button";

export default async function AdminUsersPage({
  searchParams,
}: {
  searchParams?: Promise<{ q?: string }>;
}) {
  const resolved = searchParams ? await searchParams : undefined;
  const data = await listAdminUsers(resolved?.q ?? "");

  return (
    <div className="mx-auto max-w-7xl space-y-8 px-5 py-8 lg:px-8 lg:py-10">
      <SectionHeading
        eyebrow="Accounts"
        title="Users"
        description="Basic information only. Video content stays out of the admin list."
      />

      <Card className="overflow-hidden p-5">
        <div className="space-y-3">
          {data.users.map((user) => (
            <Panel key={user.id} className="flex items-center justify-between gap-4 p-4">
              <div>
                <div className="text-sm font-medium text-white">{user.display_name}</div>
                <div className="text-xs text-zinc-500">{user.email}</div>
              </div>
              <div className="flex items-center gap-4 text-xs text-zinc-400">
                <span>{user.role}</span>
                <span>{user.clip_count} clips</span>
                <Link href={`/admin/users/${user.id}`} className="text-blue-300 hover:text-blue-200">
                  Details
                </Link>
                <DeleteUserButton userId={user.id} userName={user.display_name || user.email} />
              </div>
            </Panel>
          ))}
          {data.users.length === 0 ? <div className="text-sm text-zinc-500">No users found.</div> : null}
        </div>
      </Card>
    </div>
  );
}
