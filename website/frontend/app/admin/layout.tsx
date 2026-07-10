import type { Metadata } from "next";
import { DashboardShell } from "@/components/dashboard-shell";
import { requireAdmin } from "@/lib/server";

export const metadata: Metadata = {
  title: "Admin",
  robots: {
    index: false,
    follow: false,
  },
};

export default async function AdminLayout({ children }: { children: React.ReactNode }) {
  const user = await requireAdmin();
  return <DashboardShell user={user}>{children}</DashboardShell>;
}
