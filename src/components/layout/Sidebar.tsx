import { useMemo } from "react";
import { NavLink } from "react-router-dom";
import { Home, Film, Settings, Cloud } from "lucide-react";
import { cn } from "@/lib/utils";
import RecordingIndicator from "@/components/common/RecordingIndicator";
import PrismLogo from "@/components/common/PrismLogo";
import { useCloudStore } from "@/stores/cloud";

const navItems = [
  { to: "/", label: "Home", icon: Home },
  { to: "/library", label: "Library", icon: Film },
  { to: "/settings", label: "Settings", icon: Settings },
];

function CloudStatus() {
  const authenticated = useCloudStore((s) => s.authenticated);
  const email = useCloudStore((s) => s.email);
  const uploads = useCloudStore((s) => s.uploads);
  const pendingCount = useMemo(
    () => uploads.filter((t) => t.status === "Uploading" || t.status === "Pending").length,
    [uploads],
  );

  return (
    <div className="flex items-center gap-2 text-[11px]">
      <Cloud
        className={cn(
          "size-3 shrink-0",
          authenticated ? "text-emerald-500" : "text-zinc-600",
        )}
      />
      <span className={authenticated ? "text-zinc-400" : "text-zinc-600"}>
        {authenticated ? (email || "Connected") : "Cloud off"}
      </span>
      {pendingCount > 0 ? (
        <span className="ml-auto text-blue-400 font-medium">{pendingCount}</span>
      ) : null}
    </div>
  );
}

export default function Sidebar() {
  return (
    <aside className="w-56 h-screen flex flex-col bg-[#07101f]/90 border-r border-border">
      <div className="px-5 pt-6 pb-5">
        <div className="flex items-center gap-3">
          <PrismLogo className="h-10 w-10" />
          <div>
            <h1 className="text-lg font-semibold text-white tracking-tight">Prism</h1>
            <p className="text-[11px] text-zinc-500 mt-0.5">Game clipping</p>
          </div>
        </div>
      </div>

      <nav className="flex-1 px-3 space-y-1">
        {navItems.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            end={item.to === "/"}
            className={({ isActive }) =>
              cn(
                "flex items-center gap-3 rounded-xl px-4 py-3 text-sm font-medium transition active:scale-[0.98]",
                isActive
                  ? "bg-surface text-white"
                  : "text-zinc-400 hover:text-white hover:bg-white/5"
              )
            }
          >
            <item.icon className="size-4 shrink-0 text-blue-300" />
            <span>{item.label}</span>
          </NavLink>
        ))}
      </nav>

      <div className="px-5 py-3 border-t border-border">
        <RecordingIndicator />
      </div>

      <div className="px-5 py-3 border-t border-border space-y-1">
        <CloudStatus />
        <p className="text-[11px] text-zinc-600">Prism v0.2.3</p>
      </div>
    </aside>
  );
}
