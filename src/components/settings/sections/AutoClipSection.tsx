import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { cn } from "@/lib/utils";
import { SettingRow, SectionHeading } from "@/components/settings/SettingRow";
import { SettingCard } from "@/components/settings/SettingRow";
import { Switch } from "@/components/ui/switch";
import { Slider } from "@/components/ui/slider";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/brand";
import { useSettingsActions } from "@/hooks/useSettingsActions";

type DetectedGame = { name: string; pid: number };

const AUTO_CLIP_EVENTS: Record<string, Array<{ key: string; label: string }>> = {
  "Counter-Strike 2": [
    { key: "kill", label: "Kills" },
    { key: "death", label: "Deaths" },
    { key: "headshot", label: "Headshots" },
    { key: "win", label: "Round wins" },
  ],
  Rust: [
    { key: "combat", label: "Gunfights" },
    { key: "headshot", label: "Headshot dings" },
    { key: "explosion", label: "Rockets / C4" },
  ],
};

export default function AutoClipSection() {
  const { settings, setField, updateAutoClipGame } = useSettingsActions();
  const s = settings.auto_clip;
  const [detectedGame, setDetectedGame] = useState<DetectedGame | null>(null);

  useEffect(() => {
    let disposed = false;
    let unlistenDetected: (() => void) | undefined;
    let unlistenLost: (() => void) | undefined;
    void (async () => {
      try {
        const active = await invoke<DetectedGame | null>("get_detected_game");
        if (!disposed) setDetectedGame(active);
        unlistenDetected = await listen<DetectedGame>("game-detected", (event) => {
          setDetectedGame(event.payload);
        });
        unlistenLost = await listen("game-lost", () => setDetectedGame(null));
      } catch (error) {
        console.error("Failed to read game detection status:", error);
      }
    })();
    return () => {
      disposed = true;
      unlistenDetected?.();
      unlistenLost?.();
    };
  }, []);

  return (
    <section>
      <div className="mb-3 space-y-1">
        <span className="text-xs uppercase tracking-[0.28em] text-blue-300/70">AUTO-CLIPPING</span>
        <SectionHeading>Automatic Highlights</SectionHeading>
      </div>
      <div className="mt-3 space-y-1 border-t border-border pt-3">
        <SettingRow label="Enable auto-clipping">
          <Switch
            ariaLabel="Enable auto-clipping"
            checked={s.enabled}
            onChange={(checked) => setField("auto_clip", "enabled", checked as never)}
          />
        </SettingRow>

        <SettingRow label="Clip cooldown">
          <span className="min-w-[4ch] text-right text-sm tabular-nums text-zinc-100">
            {s.cooldown_secs}s
          </span>
          <Slider
            ariaLabel="Clip cooldown in seconds"
            value={s.cooldown_secs}
            min={5}
            max={120}
            step={5}
            onChange={(value) => setField("auto_clip", "cooldown_secs", value as never)}
            className="w-40"
          />
        </SettingRow>

        <SettingRow label="Rust audio sensitivity">
          <span className="min-w-[4ch] text-right text-sm tabular-nums text-zinc-100">
            {Math.round(s.audio_sensitivity * 100)}%
          </span>
          <Slider
            ariaLabel="Rust audio sensitivity"
            value={s.audio_sensitivity}
            min={0.1}
            max={1}
            step={0.05}
            onChange={(value) => setField("auto_clip", "audio_sensitivity", value as never)}
            className="w-40"
          />
        </SettingRow>

        <div className="space-y-3 pt-3">
          {s.games.map((game) => {
            const events = AUTO_CLIP_EVENTS[game.game_name] ?? [];
            const isDetected = detectedGame?.name === game.game_name;
            const method = game.game_name === "Counter-Strike 2" ? "Official GSI" : "Private process audio";
            return (
              <SettingCard key={game.game_name}>
                <div className="flex items-center justify-between gap-4">
                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      <h3 className="text-sm font-medium text-zinc-100">{game.game_name}</h3>
                      <Badge
                        className={cn(
                          "px-2 py-0.5 text-[10px]",
                          isDetected
                            ? "border-emerald-500/20 bg-emerald-500/15 text-emerald-300"
                            : "border-border bg-white/5 text-zinc-500"
                        )}
                      >
                        {isDetected ? "Detected" : "Waiting"}
                      </Badge>
                    </div>
                    <p className="mt-1 text-xs text-zinc-500">{method}</p>
                  </div>
                  <Switch
                    ariaLabel={`${game.game_name} auto-clipping`}
                    checked={game.enabled}
                    onChange={(enabled) => updateAutoClipGame(game.game_name, { enabled })}
                  />
                </div>

                <div className="mt-4 flex flex-wrap gap-2">
                  {events.map((event) => {
                    const selected = game.events.includes(event.key);
                    return (
                      <button
                        key={event.key}
                        type="button"
                        onClick={() =>
                          updateAutoClipGame(game.game_name, {
                            events: selected
                              ? game.events.filter((key) => key !== event.key)
                              : [...game.events, event.key],
                          })
                        }
                        className={cn(
                          "rounded-lg border px-2.5 py-1 text-xs transition active:scale-[0.98] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20 focus-visible:border-blue-400/70",
                          selected
                            ? "border-blue-400/50 bg-blue-500/15 text-blue-200"
                            : "border-border bg-white/[0.03] text-zinc-500 hover:text-zinc-300"
                        )}
                      >
                        {event.label}
                      </button>
                    );
                  })}
                </div>

                <div className="mt-4 grid grid-cols-3 gap-3">
                  <label className="text-xs text-zinc-500">
                    Kill clip
                    <Input
                      type="number"
                      min={5}
                      max={120}
                      value={game.kill_clip_duration}
                      onChange={(e) =>
                        updateAutoClipGame(game.game_name, {
                          kill_clip_duration: parseInt(e.target.value, 10) || 20,
                        })
                      }
                      className="mt-1"
                    />
                  </label>
                  <label className="text-xs text-zinc-500">
                    Death clip
                    <Input
                      type="number"
                      min={5}
                      max={120}
                      value={game.death_clip_duration}
                      onChange={(e) =>
                        updateAutoClipGame(game.game_name, {
                          death_clip_duration: parseInt(e.target.value, 10) || 30,
                        })
                      }
                      className="mt-1"
                    />
                  </label>
                  <label className="text-xs text-zinc-500">
                    Combat clip
                    <Input
                      type="number"
                      min={5}
                      max={120}
                      value={game.combat_event_duration}
                      onChange={(e) =>
                        updateAutoClipGame(game.game_name, {
                          combat_event_duration: parseInt(e.target.value, 10) || 20,
                        })
                      }
                      className="mt-1"
                    />
                  </label>
                </div>

                {game.game_name === "Rust" && (
                  <div className="mt-3 flex items-center justify-between">
                    <span className="text-xs text-zinc-500">Listen to Rust audio</span>
                    <Switch
                      ariaLabel="Listen to Rust audio"
                      checked={game.audio_enabled}
                      onChange={(audio_enabled) => updateAutoClipGame(game.game_name, { audio_enabled })}
                    />
                  </div>
                )}
              </SettingCard>
            );
          })}
        </div>

        <p className="pt-2 text-xs text-zinc-500">
          Enable Game detection above. CS2 uses Valve&apos;s localhost API; Rust reads only final process audio through Windows WASAPI.
        </p>
      </div>
    </section>
  );
}