import type { Metadata } from "next";
import { DashboardShell } from "@/components/dashboard-shell";
import { requireUser } from "@/lib/server";

export const metadata: Metadata = {
  title: "Dashboard",
  robots: {
    index: false,
    follow: false,
  },
};

export default async function DashboardLayout({ children }: { children: React.ReactNode }) {
  const user = await requireUser();
  return <DashboardShell user={user}>{children}</DashboardShell>;
}
