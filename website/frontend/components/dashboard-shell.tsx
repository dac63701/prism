import Link from "next/link";
import { LayoutDashboard, Clapperboard, Settings2, Shield } from "lucide-react";
import type { User } from "@/lib/types";
import { LogoutButton } from "@/components/logout-button";
import { PrismLogo } from "@/components/brand-icons";

const nav = [
  { href: "/dashboard", label: "Overview", icon: LayoutDashboard },
  { href: "/dashboard/clips", label: "Clips", icon: Clapperboard },
  { href: "/dashboard/settings", label: "Settings", icon: Settings2 },
];

export function DashboardShell({ user, children }: { user: User; children: React.ReactNode }) {
  return (
    <div className="min-h-screen lg:flex">
      <aside className="border-r border-white/5 bg-[#07101f]/90 lg:sticky lg:top-0 lg:h-screen lg:w-72">
        <div className="flex h-full flex-col px-5 py-6">
          <Link href="/dashboard" className="mb-8 flex items-center gap-3">
            <PrismLogo className="h-11 w-11" />
            <div>
              <div className="text-sm font-semibold text-white">{user.display_name}</div>
              <div className="text-xs text-zinc-500">{user.email}</div>
            </div>
          </Link>

          <nav className="space-y-1">
            {nav
              .concat(user.role === "admin" ? [{ href: "/admin", label: "Admin", icon: Shield }] : [])
              .map((item) => (
              <Link
                key={item.href}
                href={item.href}
                className="flex items-center gap-3 rounded-xl px-4 py-3 text-sm text-zinc-300 transition hover:bg-white/5 hover:text-white"
              >
                <item.icon className="h-4 w-4 text-blue-300" />
                {item.label}
              </Link>
            ))}
          </nav>

          <div className="mt-auto pt-6">
            <LogoutButton />
          </div>
        </div>
      </aside>

      <div className="min-w-0 flex-1">{children}</div>
    </div>
  );
}
