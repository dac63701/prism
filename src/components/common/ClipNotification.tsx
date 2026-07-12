import { useEffect, useState, useRef } from "react";
import { CheckCircle2 } from "lucide-react";
import { cn } from "@/lib/utils";
import { useRecordingStore } from "@/stores/recording";

export default function ClipNotification() {
  const lastClipPath = useRecordingStore((s) => s.lastClipPath);
  const clearLastClipPath = useRecordingStore((s) => s.clearLastClipPath);
  const [visible, setVisible] = useState(false);
  const [filename, setFilename] = useState("");
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (lastClipPath) {
      const parts = lastClipPath.split("/").pop()?.split("\\").pop() ?? lastClipPath;
      setFilename(parts);
      setVisible(true);

      if (timerRef.current) clearTimeout(timerRef.current);
      timerRef.current = setTimeout(() => {
        setVisible(false);
        clearLastClipPath();
      }, 4000);
    }

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [lastClipPath, clearLastClipPath]);

  return (
    <div
      className={cn(
        "fixed bottom-6 right-6 z-50 flex items-center gap-3 px-4 py-3 rounded-xl",
        "bg-surface/95 border border-border backdrop-blur-sm",
        "shadow-2xl transition-all duration-300 ease-out",
        visible
          ? "translate-y-0 opacity-100"
          : "translate-y-4 opacity-0 pointer-events-none"
      )}
    >
      <CheckCircle2 className="size-5 text-green-500 shrink-0" />
      <div className="flex flex-col">
        <span className="text-sm font-medium text-white">Clip saved</span>
        <span className="text-xs text-zinc-500 truncate max-w-[200px]">
          {filename}
        </span>
      </div>
    </div>
  );
}
