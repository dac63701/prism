import { useEffect, useMemo, useState } from "react";
import { NavLink } from "react-router-dom";
import { Home, Film, Settings, Cloud, CloudOff, LogOut, ExternalLink } from "lucide-react";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { cn } from "@/lib/utils";
import RecordingIndicator from "@/components/common/RecordingIndicator";
import PrismLogo from "@/components/common/PrismLogo";
import AccountAvatar from "@/components/common/AccountAvatar";
import { Dialog } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { useCloudStore } from "@/stores/cloud";
import { useSettingsStore } from "@/stores/settings";

const navItems = [
  { to: "/", label: "Home", icon: Home },
  { to: "/library", label: "Library", icon: Film },
  { to: "/settings", label: "Settings", icon: Settings },
];

interface AccountCardProps {
  collapsed: boolean;
  onSignInClick: () => void;
}

function AccountCard({ collapsed, onSignInClick }: AccountCardProps) {
  const authenticated = useCloudStore((s) => s.authenticated);
  const uploads = useCloudStore((s) => s.uploads);
  const logout = useCloudStore((s) => s.logout);
  const cloudSettings = useSettingsStore((s) => s.settings.cloud);
  const [accountOpen, setAccountOpen] = useState(false);

  const displayName = cloudSettings.account_display_name;
  const email = cloudSettings.account_email;
  const name = displayName || email.split("@")[0] || "Connected";

  const pendingCount = useMemo(
    () => uploads.filter((t) => t.status === "Uploading" || t.status === "Pending").length,
    [uploads]
  );

  if (!authenticated) {
    return (
      <button
        type="button"
        onClick={onSignInClick}
        title={collapsed ? "Sign in to Prism cloud" : undefined}
        className={cn(
          "flex w-full items-center rounded-xl px-2 py-2 text-left transition",
          collapsed ? "justify-center px-0" : "gap-2",
          "hover:bg-white/5 active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20"
        )}
      >
        <CloudOff className="size-4 shrink-0 text-zinc-500" />
        {!collapsed && (
          <>
            <span className="min-w-0 flex-1 truncate text-xs text-zinc-500">
              Cloud off
            </span>
            <span className="text-[11px] font-medium text-blue-400">Sign in</span>
          </>
        )}
      </button>
    );
  }

  return (
    <>
      <button
        type="button"
        onClick={() => setAccountOpen(true)}
        title={collapsed ? `${name} — ${email}` : undefined}
        className={cn(
          "flex w-full items-center rounded-xl px-2 py-1.5 text-left transition",
          collapsed ? "justify-center px-0" : "gap-2.5",
          "hover:bg-white/5 active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20"
        )}
      >
        <AccountAvatar src={cloudSettings.avatar_url} name={name} />
        {!collapsed && (
          <>
            <span className="min-w-0 flex-1">
              <span className="block truncate text-xs font-medium text-zinc-300">
                {name}
              </span>
              <span className="block truncate text-[11px] text-zinc-600">
                {email || "Connected"}
              </span>
            </span>
            {pendingCount > 0 && (
              <span className="inline-flex min-w-4 items-center justify-center rounded-full bg-accent/20 px-1.5 py-px text-[10px] font-semibold text-blue-300">
                {pendingCount}
              </span>
            )}
          </>
        )}
      </button>

      <Dialog
        open={accountOpen}
        onClose={() => setAccountOpen(false)}
        title="Account"
        className="max-w-sm"
      >
        <div className="flex flex-col items-center gap-4 pt-1 text-center">
          <AccountAvatar
            src={cloudSettings.avatar_url}
            name={name}
            size="size-16 text-lg"
          />
          <div className="min-w-0">
            <h3 className="truncate text-base font-semibold text-white">{name}</h3>
            {email && (
              <p className="mt-0.5 truncate text-sm text-zinc-400">{email}</p>
            )}
          </div>

          {pendingCount > 0 && (
            <div className="flex items-center gap-2 rounded-full border border-border bg-white/[0.03] px-3 py-1 text-xs text-zinc-400">
              <Cloud className="size-3.5 text-blue-300" />
              {pendingCount} upload{pendingCount === 1 ? "" : "s"} queued
            </div>
          )}

          <div className="flex w-full flex-col gap-2">
            <Button
              variant="outline"
              type="button"
              className="w-full"
              onClick={() => {
                const base = cloudSettings.server_url.trim().replace(/\/+$/, "");
                if (base) void openUrl(`${base}/dashboard`);
              }}
            >
              <ExternalLink className="size-4" />
              Open dashboard
            </Button>
            <Button
              variant="ghost"
              size="sm"
              type="button"
              className="w-full text-zinc-500 hover:text-red-400"
              onClick={() => {
                setAccountOpen(false);
                void logout();
              }}
            >
              <LogOut className="size-3.5" />
              Sign out
            </Button>
          </div>
        </div>
      </Dialog>
    </>
  );
}

function useSidebarCollapsed() {
  const [collapsed, setCollapsed] = useState(
    () => typeof window !== "undefined" && window.innerWidth < 768,
  );
  useEffect(() => {
    const mq = window.matchMedia("(max-width: 767px)");
    const on = (e: MediaQueryListEvent) => setCollapsed(e.matches);
    setCollapsed(mq.matches);
    mq.addEventListener("change", on);
    return () => mq.removeEventListener("change", on);
  }, []);
  return collapsed;
}

export default function Sidebar({ onSignInClick }: { onSignInClick: () => void }) {
  const collapsed = useSidebarCollapsed();
  const [version, setVersion] = useState("");

  useEffect(() => {
    void getVersion().then(setVersion).catch(() => setVersion(""));
  }, []);

  return (
    <aside
      className={cn(
        "flex h-full shrink-0 flex-col border-r border-border bg-[#07101f]/90",
        collapsed ? "w-14" : "w-56"
      )}
    >
      <div className={cn("px-3 pt-4 pb-3", collapsed ? "flex justify-center" : "px-5 pt-6 pb-5")}>
        <div className={cn("flex items-center gap-3", collapsed && "justify-center")}>
          <PrismLogo className="h-8 w-8 shrink-0" />
          {!collapsed && (
            <div className="min-w-0">
              <h1 className="truncate text-lg font-semibold tracking-tight text-white">
                Prism
              </h1>
              <p className="mt-0.5 text-[11px] text-zinc-500">Game clipping</p>
            </div>
          )}
        </div>
      </div>

      <nav className={cn("flex-1 space-y-1", collapsed ? "px-2" : "px-3")}>
        {navItems.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            end={item.to === "/"}
            title={collapsed ? item.label : undefined}
            className={({ isActive }) =>
              cn(
                "relative flex items-center gap-3 rounded-xl px-4 py-3 text-sm font-medium transition active:scale-[0.98]",
                collapsed && "justify-center px-0",
                isActive
                  ? "bg-surface text-white"
                  : "text-zinc-400 hover:bg-white/5 hover:text-white"
              )
            }
          >
            {({ isActive }) => (
              <>
                {isActive && (
                  <span className="absolute left-0 top-1/2 h-5 w-0.5 -translate-y-1/2 rounded-full bg-accent shadow-[0_0_8px_rgba(79,140,255,0.7)]" />
                )}
                <item.icon className="size-4 shrink-0 text-blue-300" />
                {!collapsed && <span>{item.label}</span>}
              </>
            )}
          </NavLink>
        ))}
      </nav>

      <div className={cn("border-t border-border py-3", collapsed ? "px-2" : "px-5")}>
        <RecordingIndicator collapsed={collapsed} />
      </div>

      <div className={cn("border-t border-border py-3", collapsed ? "px-2" : "px-4")}>
        <AccountCard collapsed={collapsed} onSignInClick={onSignInClick} />
        {!collapsed && (
          <p className="mt-2 px-1 text-[11px] text-zinc-600">Prism v{version || "0.3.2"}</p>
        )}
      </div>
    </aside>
  );
}