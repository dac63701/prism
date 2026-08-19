import { useEffect, useState } from "react";
import { Minimize, Square, X, Copy } from "lucide-react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { cn } from "@/lib/utils";
import { useRouteTitle } from "@/hooks/useRouteTitle";
import PrismLogo from "@/components/common/PrismLogo";

function isMacPlatform() {
  return typeof navigator !== "undefined" && /mac/i.test(navigator.platform);
}

const isMac = isMacPlatform();

export default function TitleBar() {
  const title = useRouteTitle();
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    const win = getCurrentWindow();
    let unlisten: (() => void) | undefined;
    void win
      .onResized(() => {
        void win.isMaximized().then(setMaximized);
      })
      .then((fn) => {
        unlisten = fn;
      });
    void win.isMaximized().then(setMaximized);
    return () => {
      unlisten?.();
    };
  }, []);

  const controlClass = cn(
    "flex items-center justify-center size-10 transition-colors text-zinc-400",
    "hover:bg-white/10 hover:text-white focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20"
  );

  return (
    <div
      data-tauri-drag-region="deep"
      className={cn(
        "relative flex h-10 shrink-0 items-center border-b border-border select-none",
        "bg-[#050816]/85 backdrop-blur",
        isMac && "pl-20"
      )}
    >
      <div className="flex min-w-0 flex-1 items-center gap-2.5 px-3">
        <PrismLogo className="h-5 w-5" />
        <span className="truncate text-[13px] font-medium text-zinc-300">
          Prism
        </span>
        <span className="text-zinc-600">/</span>
        <span className="truncate text-[13px] text-zinc-500">{title}</span>
      </div>

      {!isMac && (
        <div className="flex h-full items-stretch">
          <button
            type="button"
            aria-label="Minimize"
            onClick={() => void getCurrentWindow().minimize()}
            className={controlClass}
          >
            <Minimize className="size-3.5" />
          </button>
          <button
            type="button"
            aria-label={maximized ? "Restore" : "Maximize"}
            onClick={() => void getCurrentWindow().toggleMaximize()}
            className={controlClass}
          >
            {maximized ? (
              <Copy className="size-3" />
            ) : (
              <Square className="size-3" />
            )}
          </button>
          <button
            type="button"
            aria-label="Close"
            onClick={() => void getCurrentWindow().close()}
            className={cn(
              controlClass,
              "hover:bg-red-500 hover:text-white active:scale-90"
            )}
          >
            <X className="size-4" />
          </button>
        </div>
      )}
    </div>
  );
}