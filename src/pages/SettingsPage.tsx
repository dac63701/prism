import { useEffect, useRef } from "react";
import { useSettingsStore, getDefaultHotkeys } from "@/stores/settings";
import { cn } from "@/lib/utils";
import type { AppSettings } from "@/types/settings";
import PresetSlider from "@/components/settings/PresetSlider";
import HotkeyCaptureInput from "@/components/settings/HotkeyCaptureInput";

const RESOLUTION_OPTIONS = [
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
        checked ? "bg-zinc-100" : "bg-zinc-700"
      )}
    >
      <span
        className={cn(
          "inline-block h-3.5 w-3.5 rounded-full bg-zinc-950 transition-transform",
          checked ? "translate-x-[18px]" : "translate-x-[2px]"
        )}
      />
    </button>
  );
}

function SectionHeading({ children }: { children: React.ReactNode }) {
  return (
    <h2 className="text-sm font-semibold text-zinc-100 tracking-tight">
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

export default function SettingsPage() {
  const loadSettings = useSettingsStore((s) => s.loadSettings);
  const loaded = useSettingsStore((s) => s.loaded);
  const settings = useSettingsStore((s) => s.settings);
  const updateSettings = useSettingsStore((s) => s.updateSettings);

  useEffect(() => {
    if (!loaded) loadSettings();
  }, [loaded, loadSettings]);

  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const debouncedSave = (newSettings: AppSettings) => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => {
      updateSettings(newSettings);
    }, 300);
  };

  const save = (next: AppSettings) => updateSettings(next);

  const setField = <S extends keyof AppSettings, K extends keyof AppSettings[S]>(
    section: S,
    key: K,
    value: AppSettings[S][K]
  ) => {
    save({
      ...settings,
      [section]: { ...settings[section], [key]: value },
    });
  };

  const resetHotkeys = () => {
    return updateSettings({
      ...settings,
      hotkeys: getDefaultHotkeys(),
    });
  };

  const s = settings;

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
          <SectionHeading>Recording</SectionHeading>
          <div className="mt-3 border-t border-zinc-800/50 pt-3 space-y-1">
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
                className="w-40 h-1.5 bg-zinc-700 rounded-full appearance-none cursor-pointer accent-zinc-100"
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
                className="bg-zinc-900 border border-zinc-700 rounded-lg px-3 py-1.5 text-sm text-zinc-100 focus:outline-none focus:ring-1 focus:ring-zinc-600"
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
                className="bg-zinc-900 border border-zinc-700 rounded-lg px-3 py-1.5 text-sm text-zinc-100 focus:outline-none focus:ring-1 focus:ring-zinc-600"
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
                className="w-64 bg-zinc-900 border border-zinc-700 rounded-lg px-3 py-1.5 text-sm text-zinc-100 placeholder-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-600"
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
          <SectionHeading>Hotkeys</SectionHeading>
          <div className="mt-3 border-t border-zinc-800/50 pt-3 space-y-1">
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
              <button
                type="button"
                onClick={() => {
                  void resetHotkeys();
                }}
                className="text-xs text-zinc-500 hover:text-zinc-300 transition-colors"
              >
                Reset to defaults
              </button>
            </div>
          </div>
        </section>

        {/* General Section */}
        <section className="mb-8">
          <SectionHeading>General</SectionHeading>
          <div className="mt-3 border-t border-zinc-800/50 pt-3 space-y-1">
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
          </div>
        </section>

        {/* Storage Section */}
        <section className="mb-8">
          <SectionHeading>Storage</SectionHeading>
          <div className="mt-3 border-t border-zinc-800/50 pt-3 space-y-1">
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
                className="w-24 bg-zinc-900 border border-zinc-700 rounded-lg px-3 py-1.5 text-sm text-zinc-100 focus:outline-none focus:ring-1 focus:ring-zinc-600 [&::-webkit-inner-spin-button]:opacity-50"
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
                className="w-24 bg-zinc-900 border border-zinc-700 rounded-lg px-3 py-1.5 text-sm text-zinc-100 focus:outline-none focus:ring-1 focus:ring-zinc-600 [&::-webkit-inner-spin-button]:opacity-50"
              />
              <span className="text-xs text-zinc-500">(empty = disabled)</span>
            </FieldRow>
          </div>
        </section>
      </div>
    </div>
  );
}
