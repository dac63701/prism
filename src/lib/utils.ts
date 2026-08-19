import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

/** Format a seconds count as a clock string ("0:30", "5:00", "1:02:03"). */
export function formatClock(totalSeconds: number): string {
  const secs = Math.max(0, Math.round(totalSeconds))
  const h = Math.floor(secs / 3600)
  const m = Math.floor((secs % 3600) / 60)
  const s = secs % 60
  if (h > 0) {
    return `${h}:${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`
  }
  return `${m}:${String(s).padStart(2, "0")}`
}

/** Approximate file size in MB recorded per minute at a given bitrate. */
export function estimateMBperMin(bitrateKbps: number): number {
  return Math.round((bitrateKbps * 60) / 8 / 1000)
}
