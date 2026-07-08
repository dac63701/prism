import { NavLink } from "react-router-dom";
import { Home, Film, Settings } from "lucide-react";
import { cn } from "@/lib/utils";
import RecordingIndicator from "@/components/common/RecordingIndicator";

const navItems = [
  { to: "/", label: "Home", icon: Home },
  { to: "/library", label: "Library", icon: Film },
  { to: "/settings", label: "Settings", icon: Settings },
];

export default function Sidebar() {
  return (
    <aside className="w-56 h-screen flex flex-col bg-zinc-950 border-r border-zinc-800/50">
      <div className="px-5 pt-6 pb-5">
        <h1 className="text-lg font-semibold text-zinc-100 tracking-tight">
          Prism
        </h1>
        <p className="text-[11px] text-zinc-500 mt-0.5">Game clipping</p>
      </div>

      <nav className="flex-1 px-3 space-y-1">
        {navItems.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            end={item.to === "/"}
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
      </nav>

      <div className="px-5 py-3 border-t border-zinc-800/50">
        <RecordingIndicator />
      </div>

      <div className="px-5 py-3 border-t border-zinc-800/50">
        <p className="text-[11px] text-zinc-600">Prism v0.1.0</p>
      </div>
    </aside>
  );
}
