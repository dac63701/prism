import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useSettingsStore, getDefaultHotkeys } from "@/stores/settings";
import { useCloudStore } from "@/stores/cloud";
import { cn } from "@/lib/utils";
import type { AppSettings } from "@/types/settings";
import { Button } from "@/components/ui/button";
import PresetSlider from "@/components/settings/PresetSlider";
import HotkeyCaptureInput from "@/components/settings/HotkeyCaptureInput";

const RESOLUTION_OPTIONS = [
  { value: "native", label: "Native" },
  { value: "720p", label: "720p" },
  { value: "1080p", label: "1080p" },
  { value: "1440p", label: "1440p" },
  { value: "2160p", label: "4K" },
] as const;

const BITRATE_OPTIONS = [
  { value: 1000, label: "1 Mbps" },
  { value: 2500, label: "2.5" },
  { value: 5000, label: "5" },
  { value: 8000, label: "8" },
  { value: 12000, label: "12" },
  { value: 16000, label: "16" },
  { value: 25000, label: "25" },
  { value: 40000, label: "40" },
  { value: 60000, label: "60 Mbps" },
];

function ToggleSwitch({
  checked,
  onChange,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <button
      type="button"
      onClick={() => onChange(!checked)}
      className={cn(
        "relative inline-flex h-5 w-9 shrink-0 items-center rounded-full transition-colors",
        checked ? "bg-accent" : "bg-surface-2"
      )}
    >
      <span
        className={cn(
          "inline-block h-3.5 w-3.5 rounded-full bg-white transition-transform",
          checked ? "translate-x-[18px]" : "translate-x-[2px]"
        )}
      />
    </button>
  );
}

function SectionHeading({ children }: { children: React.ReactNode }) {
  return (
    <h2 className="text-lg font-semibold tracking-tight text-white">
      {children}
    </h2>
  );
}

function FieldRow({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="flex items-center justify-between py-2 gap-4">
      <label className="text-sm text-zinc-400 min-w-40">{label}</label>
      <div className="flex items-center gap-2">{children}</div>
    </div>
  );
}

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

export default function SettingsPage() {
  const loadSettings = useSettingsStore((s) => s.loadSettings);
  const loaded = useSettingsStore((s) => s.loaded);
  const settings = useSettingsStore((s) => s.settings);
  const updateSettings = useSettingsStore((s) => s.updateSettings);

  const [showManualCode, setShowManualCode] = useState(false);
  const [detectedGame, setDetectedGame] = useState<DetectedGame | null>(null);
  const [authCode, setAuthCode] = useState("");
  const cloudAuthenticated = useCloudStore((s) => s.authenticated);
  const handleAuthCode = useCloudStore((s) => s.handleAuthCode);
  const uploadError = useCloudStore((s) => s.uploadError);
  const cloudLoading = useCloudStore((s) => s.loading);

  useEffect(() => {
    if (!loaded) loadSettings();
  }, [loaded, loadSettings]);

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

  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const debouncedSave = useCallback((newSettings: AppSettings) => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => {
      updateSettings(newSettings);
    }, 300);
  }, [updateSettings]);

  const save = useCallback((next: AppSettings) => updateSettings(next), [updateSettings]);

  const setField = useCallback(<S extends keyof AppSettings, K extends keyof AppSettings[S]>(
    section: S,
    key: K,
    value: AppSettings[S][K]
  ) => {
    save({
      ...settings,
      [section]: { ...settings[section], [key]: value },
    });
  }, [save, settings]);

  const resetHotkeys = useCallback(() => {
    return updateSettings({
      ...settings,
      hotkeys: getDefaultHotkeys(),
    });
  }, [updateSettings, settings]);

  const s = settings;

  const updateAutoClipGame = useCallback(
    (gameName: string, patch: Partial<AppSettings["auto_clip"]["games"][number]>) => {
      save({
        ...settings,
        auto_clip: {
          ...settings.auto_clip,
          games: settings.auto_clip.games.map((game) =>
            game.game_name === gameName ? { ...game, ...patch } : game
          ),
        },
      });
    },
    [save, settings],
  );

  // Force inputs to remount with correct values after async load
  const loadedKey = loaded ? "loaded" : "initial";

  return (
    <div className="h-full overflow-y-auto px-6 py-6">
      <div className="max-w-2xl">
        <div className="mb-6">
          <h1 className="text-xl font-semibold text-zinc-100">Settings</h1>
          <p className="text-sm text-zinc-500 mt-1">
            Changes are saved automatically.
          </p>
        </div>

        {/* Recording Section */}
        <section className="mb-8">
          <div className="space-y-1 mb-3">
            <span className="text-xs uppercase tracking-[0.28em] text-blue-300/70">RECORDING</span>
            <SectionHeading>Recording</SectionHeading>
          </div>
          <div className="mt-3 border-t border-border pt-3 space-y-1">
            <FieldRow label="Clip length">
              <span className="text-sm text-zinc-100 min-w-[4ch] text-right tabular-nums">
                {s.recording.buffer_duration_secs}s
              </span>
              <input
                type="range"
                min={10}
                max={1800}
                step={5}
                value={s.recording.buffer_duration_secs}
                onChange={(e) =>
                  void setField(
                    "recording",
                    "buffer_duration_secs",
                    parseInt(e.target.value, 10) as never
                  )
                }
                className="w-40 h-1.5 bg-surface-2 rounded-full appearance-none cursor-pointer accent-accent"
              />
            </FieldRow>

            <FieldRow label="FPS">
              <select
                value={s.recording.fps}
                onChange={(e) =>
                  void setField(
                    "recording",
                    "fps",
                    parseInt(e.target.value, 10) as never
                  )
                }
                className="bg-surface border border-border rounded-xl px-3 py-1.5 text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-400/70"
              >
                <option value={24}>24</option>
                <option value={30}>30</option>
                <option value={60}>60</option>
              </select>
            </FieldRow>

            <FieldRow label="Resolution">
              <select
                value={s.recording.resolution}
                onChange={(e) =>
                  void setField("recording", "resolution", e.target.value as never)
                }
                className="bg-surface border border-border rounded-xl px-3 py-1.5 text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-400/70"
              >
                {RESOLUTION_OPTIONS.map((option) => (
                  <option key={option.value} value={option.value}>
                    {option.label}
                  </option>
                ))}
              </select>
            </FieldRow>

            <FieldRow label="Bitrate">
              <PresetSlider
                value={s.recording.bitrate_kbps}
                options={BITRATE_OPTIONS}
                onChange={(value) => {
                  void setField("recording", "bitrate_kbps", value as never);
                }}
              />
            </FieldRow>

            <FieldRow label="Output directory">
              <input
                type="text"
                key={`output-${loadedKey}`}
                defaultValue={s.recording.output_directory}
                onChange={(e) =>
                  debouncedSave({
                    ...settings,
                    recording: { ...settings.recording, output_directory: e.target.value },
                  })
                }
                placeholder="~/Videos/Prism"
                className="w-64 bg-surface border border-border rounded-xl px-3 py-1.5 text-sm text-white placeholder-zinc-500 focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-400/70"
              />
            </FieldRow>

            <FieldRow label="Always-on recording">
              <ToggleSwitch
                checked={s.recording.always_on_recording}
                onChange={(checked) =>
                  void setField("recording", "always_on_recording", checked as never)
                }
              />
            </FieldRow>
          </div>
        </section>

        {/* Hotkeys Section */}
        <section className="mb-8">
          <div className="space-y-1 mb-3">
            <span className="text-xs uppercase tracking-[0.28em] text-blue-300/70">HOTKEYS</span>
            <SectionHeading>Hotkeys</SectionHeading>
          </div>
          <div className="mt-3 border-t border-border pt-3 space-y-1">
            <FieldRow label="Save clip">
              <HotkeyCaptureInput
                value={s.hotkeys.save_clip}
                onChange={async (value) => {
                  await setField("hotkeys", "save_clip", value as never);
                }}
              />
            </FieldRow>

            <FieldRow label="Toggle recording">
              <HotkeyCaptureInput
                value={s.hotkeys.toggle_recording}
                onChange={async (value) => {
                  await setField("hotkeys", "toggle_recording", value as never);
                }}
              />
            </FieldRow>

            <FieldRow label="Open library">
              <HotkeyCaptureInput
                value={s.hotkeys.open_library}
                onChange={async (value) => {
                  await setField("hotkeys", "open_library", value as never);
                }}
              />
            </FieldRow>

            <div className="flex justify-end pt-2">
              <Button
                variant="ghost"
                size="xs"
                type="button"
                onClick={() => {
                  void resetHotkeys();
                }}
              >
                Reset to defaults
              </Button>
            </div>
          </div>
        </section>

        {/* General Section */}
        <section className="mb-8">
          <div className="space-y-1 mb-3">
            <span className="text-xs uppercase tracking-[0.28em] text-blue-300/70">GENERAL</span>
            <SectionHeading>General</SectionHeading>
          </div>
          <div className="mt-3 border-t border-border pt-3 space-y-1">
            <FieldRow label="Launch at startup">
              <ToggleSwitch
                checked={s.general.launch_at_startup}
                onChange={(checked) =>
                  void setField("general", "launch_at_startup", checked as never)
                }
              />
            </FieldRow>

            <FieldRow label="Minimize to tray">
              <ToggleSwitch
                checked={s.general.minimize_to_tray}
                onChange={(checked) =>
                  void setField("general", "minimize_to_tray", checked as never)
                }
              />
            </FieldRow>

            <FieldRow label="Show clip notification">
              <ToggleSwitch
                checked={s.general.show_clip_notification}
                onChange={(checked) =>
                  void setField("general", "show_clip_notification", checked as never)
                }
              />
            </FieldRow>

            <FieldRow label="Game detection">
              <ToggleSwitch
                checked={s.general.game_detection_enabled}
                onChange={(checked) =>
                  void setField("general", "game_detection_enabled", checked as never)
                }
              />
            </FieldRow>
            <FieldRow label="CS2 GSI port">
              <input
                type="number"
                min={1024}
                max={65535}
                value={s.general.cs2_gsi_port}
                onChange={(e) =>
                  void setField(
                    "general",
                    "cs2_gsi_port",
                    (parseInt(e.target.value, 10) || 4000) as never
                  )
                }
                className="w-24 bg-surface border border-border rounded-xl px-3 py-1.5 text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus-visible:border-blue-400/70"
              />
            </FieldRow>
            <p className="pt-1 text-xs text-zinc-500">
              Restart Prism after changing the CS2 GSI port.
            </p>
          </div>
        </section>

        {/* Auto-clipping Section */}
        <section className="mb-8">
          <div className="space-y-1 mb-3">
            <span className="text-xs uppercase tracking-[0.28em] text-blue-300/70">AUTO-CLIPPING</span>
            <SectionHeading>Automatic Highlights</SectionHeading>
          </div>
          <div className="mt-3 border-t border-border pt-3 space-y-1">
            <FieldRow label="Enable auto-clipping">
              <ToggleSwitch
                checked={s.auto_clip.enabled}
                onChange={(checked) =>
                  void setField("auto_clip", "enabled", checked as never)
                }
              />
            </FieldRow>
            <FieldRow label="Clip cooldown">
              <span className="text-sm text-zinc-100 min-w-[4ch] text-right tabular-nums">
                {s.auto_clip.cooldown_secs}s
              </span>
              <input
                type="range"
                min={5}
                max={120}
                step={5}
                value={s.auto_clip.cooldown_secs}
                onChange={(e) =>
                  void setField(
                    "auto_clip",
                    "cooldown_secs",
                    parseInt(e.target.value, 10) as never
                  )
                }
                className="w-40 h-1.5 bg-surface-2 rounded-full appearance-none cursor-pointer accent-accent"
              />
            </FieldRow>
            <FieldRow label="Rust audio sensitivity">
              <span className="text-sm text-zinc-100 min-w-[4ch] text-right tabular-nums">
                {Math.round(s.auto_clip.audio_sensitivity * 100)}%
              </span>
              <input
                type="range"
                min={0.1}
                max={1}
                step={0.05}
                value={s.auto_clip.audio_sensitivity}
                onChange={(e) =>
                  void setField(
                    "auto_clip",
                    "audio_sensitivity",
                    parseFloat(e.target.value) as never
                  )
                }
                className="w-40 h-1.5 bg-surface-2 rounded-full appearance-none cursor-pointer accent-accent"
              />
            </FieldRow>

            <div className="pt-3 space-y-3">
              {s.auto_clip.games.map((game) => {
                const events = AUTO_CLIP_EVENTS[game.game_name] ?? [];
                const isDetected = detectedGame?.name === game.game_name;
                const method = game.game_name === "Counter-Strike 2" ? "Official GSI" : "Private process audio";
                return (
                  <div key={game.game_name} className="rounded-2xl border border-border bg-surface/70 p-4">
                    <div className="flex items-center justify-between gap-4">
                      <div>
                        <div className="flex items-center gap-2">
                          <h3 className="text-sm font-medium text-zinc-100">{game.game_name}</h3>
                          <span className={cn(
                            "rounded-full px-2 py-0.5 text-[10px] font-medium",
                            isDetected ? "bg-emerald-500/15 text-emerald-300" : "bg-white/5 text-zinc-500"
                          )}>
                            {isDetected ? "Detected" : "Waiting"}
                          </span>
                        </div>
                        <p className="mt-1 text-xs text-zinc-500">{method}</p>
                      </div>
                      <ToggleSwitch
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
                        <input
                          type="number"
                          min={5}
                          max={120}
                          value={game.kill_clip_duration}
                          onChange={(e) => updateAutoClipGame(game.game_name, { kill_clip_duration: parseInt(e.target.value, 10) || 20 })}
                          className="mt-1 w-full bg-surface border border-border rounded-lg px-2 py-1.5 text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus-visible:border-blue-400/70"
                        />
                      </label>
                      <label className="text-xs text-zinc-500">
                        Death clip
                        <input
                          type="number"
                          min={5}
                          max={120}
                          value={game.death_clip_duration}
                          onChange={(e) => updateAutoClipGame(game.game_name, { death_clip_duration: parseInt(e.target.value, 10) || 30 })}
                          className="mt-1 w-full bg-surface border border-border rounded-lg px-2 py-1.5 text-sm text-white focus:outline-none focus:ring-2 focus-visible:border-blue-400/70"
                        />
                      </label>
                      <label className="text-xs text-zinc-500">
                        Combat clip
                        <input
                          type="number"
                          min={5}
                          max={120}
                          value={game.combat_event_duration}
                          onChange={(e) => updateAutoClipGame(game.game_name, { combat_event_duration: parseInt(e.target.value, 10) || 20 })}
                          className="mt-1 w-full bg-surface border border-border rounded-lg px-2 py-1.5 text-sm text-white focus:outline-none focus:ring-2 focus-visible:border-blue-400/70"
                        />
                      </label>
                    </div>

                    {game.game_name === "Rust" && (
                      <div className="mt-3 flex items-center justify-between">
                        <span className="text-xs text-zinc-500">Listen to Rust audio</span>
                        <ToggleSwitch
                          checked={game.audio_enabled}
                          onChange={(audio_enabled) => updateAutoClipGame(game.game_name, { audio_enabled })}
                        />
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
            <p className="pt-2 text-xs text-zinc-500">
              Enable Game detection above. CS2 uses Valve&apos;s localhost API; Rust reads only final process audio through Windows WASAPI.
            </p>
          </div>
        </section>

        {/* Cloud Section */}
        <section className="mb-8">
          <div className="space-y-1 mb-3">
            <span className="text-xs uppercase tracking-[0.28em] text-blue-300/70">CLOUD</span>
            <SectionHeading>Cloud Upload</SectionHeading>
          </div>
          <div className="mt-3 border-t border-border pt-3 space-y-1">
            <FieldRow label="Server URL">
              <input
                type="text"
                key={`server-url-${loadedKey}`}
                defaultValue={s.cloud.server_url}
                onChange={(e) =>
                  debouncedSave({
                    ...settings,
                    cloud: { ...settings.cloud, server_url: e.target.value },
                  })
                }
                placeholder="https://clips.example.com"
                className="w-64 bg-surface border border-border rounded-xl px-3 py-1.5 text-sm text-white placeholder-zinc-500 focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-400/70"
              />
            </FieldRow>

            <FieldRow label="Account">
              {cloudAuthenticated ? (
                <div className="flex items-center gap-3">
                  <div className="text-sm text-zinc-300">
                    {s.cloud.account_display_name || "Connected"}
                    {s.cloud.account_email ? (
                      <span className="text-zinc-500 ml-2 text-xs">
                        {s.cloud.account_email}
                      </span>
                    ) : null}
                  </div>
                  <Button
                    variant="ghost"
                    size="xs"
                    type="button"
                    onClick={() => {
                      useCloudStore.getState().logout();
                    }}
                    className="text-zinc-500 hover:text-red-400"
                  >
                    Sign out
                  </Button>
                </div>
              ) : (
                <div className="flex flex-col items-start gap-2">
                  <div className="flex items-center gap-3">
                    <span className="text-sm text-zinc-600">Not signed in</span>
                    <Button
                      variant="ghost"
                      size="xs"
                      type="button"
                      onClick={() => {
                        useCloudStore.getState().login();
                      }}
                      className="text-blue-400 hover:text-blue-300"
                    >
                      Sign in with Google
                    </Button>
                  </div>
                  <Button
                    variant="ghost"
                    size="xs"
                    type="button"
                    onClick={() => setShowManualCode(!showManualCode)}
                    className="text-zinc-500 hover:text-zinc-300"
                  >
                    Trouble signing in? Paste auth code manually
                  </Button>
                  {showManualCode && (
                    <div className="flex flex-col gap-2 w-full mt-1">
                      <input
                        type="text"
                        value={authCode}
                        onChange={(e) => setAuthCode(e.target.value)}
                        placeholder="Paste auth code here..."
                        className="w-full bg-surface border border-border rounded-xl px-3 py-1.5 text-sm text-white placeholder-zinc-500 focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-400/70"
                      />
                      <div className="flex items-center gap-2">
                        <Button
                          variant="brand"
                          size="xs"
                          type="button"
                          onClick={() => {
                            handleAuthCode(authCode);
                            setAuthCode("");
                            setShowManualCode(false);
                          }}
                          disabled={!authCode.trim() || cloudLoading}
                        >
                          {cloudLoading ? "Submitting..." : "Submit code"}
                        </Button>
                      </div>
                    </div>
                  )}
                </div>
              )}
            </FieldRow>
            {uploadError && (
              <div className="mt-2 px-4 py-2 rounded-lg bg-red-950/60 border border-red-900/60">
                <p className="text-xs text-red-300">{uploadError}</p>
              </div>
            )}

            <FieldRow label="Auto-upload">
              <ToggleSwitch
                checked={s.cloud.auto_upload}
                onChange={(checked) =>
                  void setField("cloud", "auto_upload", checked as never)
                }
              />
            </FieldRow>

            <FieldRow label="Concurrent uploads">
              <select
                value={s.cloud.max_concurrent_uploads}
                onChange={(e) =>
                  void setField(
                    "cloud",
                    "max_concurrent_uploads",
                    (parseInt(e.target.value, 10) || 1) as never,
                  )
                }
                className="bg-surface border border-border rounded-xl px-3 py-1.5 text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-400/70"
              >
                <option value={1}>1 (sequential)</option>
                <option value={2}>2</option>
                <option value={3}>3</option>
              </select>
            </FieldRow>
          </div>
        </section>

        {/* Storage Section */}
        <section className="mb-8">
          <div className="space-y-1 mb-3">
            <span className="text-xs uppercase tracking-[0.28em] text-blue-300/70">STORAGE</span>
            <SectionHeading>Storage</SectionHeading>
          </div>
          <div className="mt-3 border-t border-border pt-3 space-y-1">
            <FieldRow label="Max clips (GB)">
              <input
                type="number"
                key={`max-gb-${loadedKey}`}
                min={0}
                defaultValue={s.storage.max_clips_gb}
                onChange={(e) =>
                  void setField(
                    "storage",
                    "max_clips_gb",
                    (parseInt(e.target.value, 10) || 0) as never
                  )
                }
                className="w-24 bg-surface border border-border rounded-xl px-3 py-1.5 text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-400/70 [&::-webkit-inner-spin-button]:opacity-50"
              />
              <span className="text-xs text-zinc-500">(0 = unlimited)</span>
            </FieldRow>

            <FieldRow label="Auto-prune (days)">
              <input
                type="number"
                key={`prune-${loadedKey}`}
                min={0}
                defaultValue={s.storage.auto_prune_days ?? ""}
                onChange={(e) => {
                  const val = e.target.value;
                  debouncedSave({
                    ...settings,
                    storage: {
                      ...settings.storage,
                      auto_prune_days: val === "" ? null : parseInt(val, 10),
                    },
                  });
                }}
                className="w-24 bg-surface border border-border rounded-xl px-3 py-1.5 text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-400/70 [&::-webkit-inner-spin-button]:opacity-50"
              />
              <span className="text-xs text-zinc-500">(empty = disabled)</span>
            </FieldRow>
          </div>
        </section>
      </div>
    </div>
  );
}
