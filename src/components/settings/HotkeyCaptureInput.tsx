import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { cn } from "@/lib/utils";

interface HotkeyCaptureInputProps {
  value: string;
  onChange: (next: string) => Promise<void>;
}

function isMacPlatform() {
  return typeof navigator !== "undefined" && /mac/i.test(navigator.platform);
}

function normalizeKey(event: KeyboardEvent) {
  const special: Record<string, string> = {
    Escape: "Escape",
    Tab: "Tab",
    Enter: "Enter",
    Space: "Space",
    Backspace: "Backspace",
    Delete: "Delete",
    Home: "Home",
    End: "End",
    PageUp: "PageUp",
    PageDown: "PageDown",
    ArrowUp: "Up",
    ArrowDown: "Down",
    ArrowLeft: "Left",
    ArrowRight: "Right",
  };

  if (event.code.startsWith("Key")) {
    return event.code.slice(3);
  }
  if (event.code.startsWith("Digit")) {
    return event.code.slice(5);
  }

  if (event.key.length === 1) {
    return event.key.toUpperCase();
  }

  return special[event.key] ?? event.key;
}

function buildShortcut(event: KeyboardEvent) {
  if (["Meta", "Control", "Alt", "Shift"].includes(event.key)) {
    return null;
  }

  const parts: string[] = [];

  if (event.metaKey) parts.push(isMacPlatform() ? "Cmd" : "Win");
  if (event.ctrlKey) parts.push("Ctrl");
  if (event.altKey) parts.push(isMacPlatform() ? "Option" : "Alt");
  if (event.shiftKey) parts.push("Shift");

  const key = normalizeKey(event);
  if (!key || key === "Escape") return key === "Escape" ? "__cancel__" : null;

  parts.push(key);
  return parts.join("+");
}

export default function HotkeyCaptureInput({ value, onChange }: HotkeyCaptureInputProps) {
  const [capturing, setCapturing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const label = useMemo(() => (capturing ? "Press shortcut..." : value || "Set hotkey"), [capturing, value]);

  useEffect(() => {
    if (!capturing) return;

    const handleKeyDown = async (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();

      const combo = buildShortcut(event);
      if (!combo) return;
      if (combo === "__cancel__") {
        setCapturing(false);
        setError(null);
        return;
      }

      try {
        await invoke("validate_hotkey", { hotkeyStr: combo });
        await onChange(combo);
        setError(null);
        setCapturing(false);
      } catch (err) {
        setError(typeof err === "string" ? err : "Invalid hotkey");
      }
    };

    window.addEventListener("keydown", handleKeyDown, true);
    return () => window.removeEventListener("keydown", handleKeyDown, true);
  }, [capturing, onChange]);

  return (
    <div className="flex items-center gap-2">
      <button
        type="button"
        onClick={() => {
          setCapturing(true);
          setError(null);
        }}
        onBlur={() => {
          setCapturing(false);
          setError(null);
        }}
        className={cn(
          "min-w-44 rounded-xl border px-3 py-1.5 text-left text-sm font-mono transition active:scale-[0.98] focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-400/70",
          capturing
            ? "border-accent bg-accent text-white"
            : "border-border bg-surface text-white hover:border-border"
        )}
      >
        {label}
      </button>
      {error && <span className="text-xs text-red-400">{error}</span>}
    </div>
  );
}
