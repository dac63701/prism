import { useEffect, useRef, useState } from "react";
import { cn } from "@/lib/utils";

interface TooltipProps {
  label: React.ReactNode;
  children: React.ReactNode;
  side?: "top" | "bottom" | "left" | "right";
  className?: string;
}

export function Tooltip({ label, children, side = "top", className }: TooltipProps) {
  const [visible, setVisible] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, []);

  const position = {
    top: "bottom-full left-1/2 mb-2 -translate-x-1/2",
    bottom: "top-full left-1/2 mt-2 -translate-x-1/2",
    left: "right-full top-1/2 mr-2 -translate-y-1/2",
    right: "left-full top-1/2 ml-2 -translate-y-1/2",
  }[side];

  return (
    <span
      className="relative inline-flex"
      onMouseEnter={() => {
        timerRef.current = setTimeout(() => setVisible(true), 350);
      }}
      onMouseLeave={() => {
        if (timerRef.current) clearTimeout(timerRef.current);
        setVisible(false);
      }}
    >
      {children}
      {visible && (
        <span
          role="tooltip"
          className={cn(
            "pointer-events-none absolute z-50 whitespace-nowrap rounded-md border border-border",
            "bg-[#0b1222]/95 px-2 py-1 text-[11px] text-zinc-200 shadow-lg shadow-black/30",
            "animate-fade-in",
            position,
            className
          )}
        >
          {label}
        </span>
      )}
    </span>
  );
}