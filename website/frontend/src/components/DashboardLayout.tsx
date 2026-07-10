import { NavLink, Outlet } from "react-router-dom";
import {
  Home,
  Film,
  Settings,
  Shield,
  Users,
  Clapperboard,
  ScrollText,
  LogOut,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { useAuthStore } from "@/stores/auth";

interface NavItem {
  to: string;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  exact?: boolean;
}

const navItems: NavItem[] = [
  { to: "/", label: "Home", icon: Home, exact: true },
  { to: "/library", label: "Library", icon: Film },
  { to: "/settings", label: "Settings", icon: Settings },
];

const adminItems: NavItem[] = [
  { to: "/admin", label: "Dashboard", icon: Shield },
  { to: "/admin/users", label: "Users", icon: Users },
  { to: "/admin/clips", label: "All Clips", icon: Clapperboard },
  { to: "/admin/logs", label: "Activity Log", icon: ScrollText },
];

export default function DashboardLayout() {
  const user = useAuthStore((s) => s.user);
  const logout = useAuthStore((s) => s.logout);

  return (
    <div className="flex h-screen bg-zinc-950 text-zinc-100">
      <aside className="w-56 flex flex-col border-r border-zinc-800/50 bg-zinc-950">
        <div className="px-5 pt-6 pb-5">
          <h1 className="text-lg font-semibold tracking-tight text-zinc-100">
            Prism
          </h1>
          <p className="text-[11px] text-zinc-500 mt-0.5">Clip sharing</p>
        </div>

        <nav className="flex-1 px-3 space-y-1">
          {navItems.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              end={!!item.exact}
              className={({ isActive }) =>
                cn(
                  "flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-colors",
                  isActive
                    ? "bg-zinc-800/70 text-zinc-100"
                    : "text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800/40"
                )
              }
            >
              <item.icon className="size-4 shrink-0" />
              <span>{item.label}</span>
            </NavLink>
          ))}

          {user?.role === "admin" && (
            <>
              <div className="pt-3 pb-1 px-3">
                <p className="text-[11px] font-medium text-zinc-600 uppercase tracking-wider">
                  Admin
                </p>
              </div>
              {adminItems.map((item) => (
                <NavLink
                  key={item.to}
                  to={item.to}
                  end={!!item.exact}
                  className={({ isActive }) =>
                    cn(
                      "flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-colors",
                      isActive
                        ? "bg-amber-900/30 text-amber-300"
                        : "text-zinc-500 hover:text-zinc-200 hover:bg-zinc-800/40"
                    )
                  }
                >
                  <item.icon className="size-4 shrink-0" />
                  <span>{item.label}</span>
                </NavLink>
              ))}
            </>
          )}
        </nav>

        <div className="px-4 py-3 border-t border-zinc-800/50">
          <div className="flex items-center gap-2">
            <div className="flex-1 min-w-0">
              <p className="text-xs text-zinc-400 truncate">
                {user?.display_name || user?.email}
              </p>
              <p className="text-[10px] text-zinc-600">{user?.role}</p>
            </div>
            <button
              onClick={logout}
              className="p-1.5 rounded-md text-zinc-500 hover:text-zinc-300 hover:bg-zinc-800 transition-colors"
              title="Logout"
            >
              <LogOut className="size-4" />
            </button>
          </div>
        </div>
      </aside>

      <main className="flex-1 overflow-y-auto">
        <Outlet />
      </main>
    </div>
  );
}
