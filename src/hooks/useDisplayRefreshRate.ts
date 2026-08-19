import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

/**
 * Main display refresh rate in Hz, or 0 if undetectable.
 * Best-effort — used to label the "Auto" FPS setting.
 */
export function useDisplayRefreshRate() {
  const [refreshRate, setRefreshRate] = useState(0);

  useEffect(() => {
    let cancelled = false;
    invoke<number>("get_display_refresh_rate")
      .then((rate) => {
        if (!cancelled) setRefreshRate(rate);
      })
      .catch(() => {
        // Best-effort; keep 0.
      });
    return () => {
      cancelled = true;
    };
  }, []);

  return refreshRate;
}