import { useCallback, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Wand2,
  Clapperboard,
  Timer,
  MonitorPlay,
  FolderOpen,
  Settings2,
  ChevronDown,
  AlertTriangle,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { SettingRow, SectionHeading, SettingCard, GroupTitle } from "@/components/settings/SettingRow";
import SegmentedControl, { type SegmentedOption } from "@/components/settings/SegmentedControl";
import PresetSlider, { type PresetOption } from "@/components/settings/PresetSlider";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Dialog } from "@/components/ui/dialog";
import { useSettingsActions } from "@/hooks/useSettingsActions";
import { useSettingsStore, getDefaultSettings } from "@/stores/settings";
import { useDisplayRefreshRate } from "@/hooks/useDisplayRefreshRate";
import { formatClock, estimateMBperMin } from "@/lib/utils";
import {
  QUALITY_PRESETS,
  detectQualityPreset,
  applyQualityPreset,
  presetByKey,
} from "@/lib/presets";
import type { RecordingSettings } from "@/types/settings";

const RESOLUTION_OPTIONS: SegmentedOption[] = [
  { value: "native", label: "Native" },
  { value: "720p", label: "720p" },
  { value: "1080p", label: "1080p" },
  { value: "1440p", label: "1440p" },
  { value: "2160p", label: "4K" },
];

const ASPECT_OPTIONS: SegmentedOption[] = [
  { value: "match", label: "Match" },
  { value: "16:9", label: "16:9" },
  { value: "21:9", label: "21:9" },
  { value: "32:9", label: "32:9" },
];

const ASPECT_LABEL: Record<string, string> = {
  match: "Match",
  "16:9": "16:9",
  "21:9": "21:9",
  "32:9": "32:9",
};

const FPS_OPTIONS = [24, 30, 60, 120, 144] as const;

const BITRATE_OPTIONS: PresetOption[] = [
  { value: 1000, label: "1 Mbps" },
  { value: 2500, label: "2.5 Mbps" },
  { value: 5000, label: "5 Mbps" },
  { value: 8000, label: "8 Mbps" },
  { value: 12000, label: "12 Mbps" },
  { value: 16000, label: "16 Mbps" },
  { value: 25000, label: "25 Mbps" },
  { value: 40000, label: "40 Mbps" },
  { value: 60000, label: "60 Mbps" },
];

export default function RecordingSection() {
  const { settings, save, debouncedSave } = useSettingsActions();
  const loaded = useSettingsStore((s) => s.loaded);
  const s = settings.recording;

  const refreshRate = useDisplayRefreshRate();
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [confirmResetOpen, setConfirmResetOpen] = useState(false);

  const updateRecording = useCallback(
    (patch: Partial<RecordingSettings>) => {
      save({ ...settings, recording: { ...settings.recording, ...patch } });
    },
    [save, settings]
  );

  const effectiveFps = s.fps_auto ? (refreshRate > 0 ? refreshRate : s.fps) : s.fps;
  const bitrateMbps = (s.bitrate_kbps / 1000).toFixed(1).replace(/\.0$/, "");
  const mbPerMin = estimateMBperMin(s.bitrate_kbps);

  // `0` is reserved for the Auto (match display) preset.
  const fpsOptions: PresetOption[] = [
    { value: 0, label: refreshRate > 0 ? `Auto · ${refreshRate} Hz` : "Auto" },
    ...FPS_OPTIONS.map((fps) => ({ value: fps, label: `${fps}` })),
  ];

  const presetOptions: SegmentedOption[] = QUALITY_PRESETS.map((preset) => ({
    value: preset.key,
    label: preset.label,
  }));

  const presetKey = detectQualityPreset(s);
  const activePreset = presetKey === "custom" ? undefined : presetByKey(presetKey);
  const presetDescription =
    activePreset?.description ?? "Custom — hand-tuned resolution, FPS, and bitrate";

  const applyPreset = (value: string) => {
    if (value === "fast" || value === "balanced" || value === "high") {
      updateRecording(applyQualityPreset(settings.recording, value));
    }
  };

  const setResolution = (value: string) =>
    updateRecording({ resolution: value, quality_preset: "custom" });

  const setAspect = (value: string) => updateRecording({ aspect_ratio: value });

  const setFps = (value: number) =>
    updateRecording(
      value === 0
        ? { fps_auto: true, quality_preset: "custom" }
        : { fps: value, fps_auto: false, quality_preset: "custom" }
    );

  const setBitrate = (value: number) =>
    updateRecording({ bitrate_kbps: value, quality_preset: "custom" });

  const openOutputFolder = useCallback(async () => {
    try {
      await invoke("open_clip_location");
    } catch (err) {
      console.error("Failed to open output folder:", err);
    }
  }, []);

  const resetRecording = () => {
    updateRecording(getDefaultSettings().recording);
    setConfirmResetOpen(false);
  };

  const targetLabel = useMemo(() => {
    if (!s.capture_target.trim()) return "Main display";
    try {
      const parsed = JSON.parse(s.capture_target);
      if (typeof parsed === "string" && parsed === "display") return "Main display";
      if (typeof parsed === "object" && parsed !== null) {
        if ("display_id" in parsed) return `Display ${parsed.display_id}`;
        if ("application" in parsed) {
          const bundleId = parsed.application as string;
          const parts = bundleId.split(".");
          return parts.length > 2
            ? parts.slice(0, -1).pop() ?? bundleId
            : parts.pop() ?? bundleId;
        }
      }
      return null;
    } catch {
      return null;
    }
  }, [s.capture_target]);

  return (
    <section>
      <div className="mb-4 space-y-1">
        <span className="text-xs uppercase tracking-[0.28em] text-blue-300/70">RECORDING</span>
        <SectionHeading>Recording</SectionHeading>
      </div>

      <div className="space-y-4">
        {/* Quality preset */}
        <SettingCard className="p-4">
          <GroupTitle
            icon={Wand2}
            title="Quality preset"
            description="Set resolution, FPS, and bitrate together. Tuning any of them below switches to a custom setup."
          />
          <div className="mt-3 border-t border-border pt-3">
            <SegmentedControl
              value={presetKey}
              options={presetOptions}
              onChange={applyPreset}
              ariaLabel="Quality preset"
            />
            <p className="mt-2 text-xs text-zinc-500">{presetDescription}</p>
          </div>
        </SettingCard>

        {/* Video quality */}
        <SettingCard className="p-4">
          <GroupTitle
            icon={Clapperboard}
            title="Video quality"
            description="Fine-tune how clips are captured and encoded."
          />
          <div className="mt-3 space-y-1 border-t border-border pt-3">
            <SettingRow label="Resolution" className="flex-wrap">
              <SegmentedControl
                value={s.resolution}
                options={RESOLUTION_OPTIONS}
                onChange={setResolution}
                ariaLabel="Resolution"
              />
            </SettingRow>

            <SettingRow
              label="Aspect ratio"
              help="Match keeps your display's ratio — an ultrawide stays ultrawide instead of being stretched. Forcing a ratio fills that shape (may stretch if your display differs)."
              className="flex-wrap"
            >
              <SegmentedControl
                value={s.aspect_ratio}
                options={ASPECT_OPTIONS}
                onChange={setAspect}
                ariaLabel="Aspect ratio"
              />
            </SettingRow>

            <SettingRow
              label="FPS"
              help="Auto matches your display's refresh rate. Higher FPS = smoother clips, larger files."
              className="flex-wrap"
            >
              <PresetSlider
                value={s.fps_auto ? 0 : s.fps}
                options={fpsOptions}
                ariaLabel="FPS"
                className="w-full max-w-[20rem]"
                onChange={setFps}
              />
            </SettingRow>

            <SettingRow
              label="Bitrate"
              help="Higher bitrate = sharper clips, larger files. 8 Mbps is a good default for 1080p."
              className="flex-wrap"
            >
              <PresetSlider
                value={s.bitrate_kbps}
                options={BITRATE_OPTIONS}
                ariaLabel="Bitrate"
                className="w-full max-w-[20rem]"
                onChange={setBitrate}
              />
            </SettingRow>
          </div>

          <div className="mt-3 flex items-center justify-between gap-2 rounded-xl border border-border bg-surface px-3 py-2 text-xs">
            <span className="text-zinc-500">Output</span>
            <span className="tabular-nums text-zinc-100">
              {s.resolution === "native"
                ? "Native"
                : `${s.resolution} · ${ASPECT_LABEL[s.aspect_ratio] ?? "Match"}`}{" "}
              · {effectiveFps} FPS · {bitrateMbps} Mbps · ≈{mbPerMin} MB/min
            </span>
          </div>
        </SettingCard>

        {/* Clip length */}
        <SettingCard className="p-4">
          <GroupTitle
            icon={Timer}
            title="Clip length"
            description="How much recent gameplay the shadow buffer keeps before overwriting."
          />
          <div className="mt-3 space-y-1 border-t border-border pt-3">
            <SettingRow label="Buffer duration">
              <Slider
                ariaLabel="Clip length in seconds"
                value={s.buffer_duration_secs}
                min={10}
                max={1800}
                step={5}
                onChange={(value) => updateRecording({ buffer_duration_secs: value })}
              />
              <span className="w-12 shrink-0 whitespace-nowrap text-right text-sm tabular-nums text-zinc-100">
                {formatClock(s.buffer_duration_secs)}
              </span>
            </SettingRow>
          </div>
          <p className="mt-2 text-xs text-zinc-500">
            Prism continuously buffers the last {formatClock(s.buffer_duration_secs)} of gameplay in
            memory. Press your save-clip hotkey to export it as an MP4.
          </p>
        </SettingCard>

        {/* Capture */}
        <SettingCard className="p-4">
          <GroupTitle
            icon={MonitorPlay}
            title="Capture"
            description="Recording behavior and audio."
          />
          <div className="mt-3 space-y-1 border-t border-border pt-3">
            <SettingRow label="Always-on recording" help="Automatically start the buffer when Prism launches. Turn this off to start and stop the buffer manually with the Home-screen button or the toggle-recording hotkey.">
              <Switch
                ariaLabel="Always-on recording"
                checked={s.always_on_recording}
                onChange={(checked) => updateRecording({ always_on_recording: checked })}
              />
            </SettingRow>

            <SettingRow
              label="System audio"
              help="Record system sounds in clips (Windows). Turn off to save clips without an audio track."
            >
              <Switch
                ariaLabel="System audio"
                checked={s.capture_audio}
                onChange={(checked) => updateRecording({ capture_audio: checked })}
              />
            </SettingRow>
          </div>
        </SettingCard>

        {/* Storage */}
        <SettingCard className="p-4">
          <GroupTitle icon={FolderOpen} title="Storage" description="Where saved clips are written." />
          <div className="mt-3 space-y-1 border-t border-border pt-3">
            <SettingRow label="Output directory" className="flex-wrap">
              <Input
                key={loaded ? "output-loaded" : "output-initial"}
                defaultValue={s.output_directory}
                placeholder="~/Videos/Prism"
                onChange={(e) =>
                  debouncedSave({
                    ...settings,
                    recording: { ...settings.recording, output_directory: e.target.value },
                  })
                }
                className="w-full md:w-64"
              />
              <Button
                variant="outline"
                size="sm"
                type="button"
                onClick={openOutputFolder}
                className="shrink-0"
              >
                <FolderOpen className="size-3.5" />
                Open folder
              </Button>
            </SettingRow>
          </div>
        </SettingCard>

        {/* Advanced */}
        <SettingCard className="p-4">
          <button
            type="button"
            onClick={() => setAdvancedOpen((open) => !open)}
            className="flex w-full items-center justify-between gap-3 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20"
          >
            <GroupTitle
              icon={Settings2}
              title="Advanced"
              description="Rarely changed capture details."
            />
            <ChevronDown
              className={cn(
                "size-4 shrink-0 text-zinc-500 transition-transform",
                advancedOpen && "rotate-180"
              )}
            />
          </button>
          {advancedOpen && (
            <div className="mt-3 space-y-1 border-t border-border pt-3">
              <SettingRow label="Capture source">
                <span className="truncate text-sm text-zinc-300">
                  {targetLabel || "Main display"}
                </span>
              </SettingRow>
              <p className="pt-1 text-xs text-zinc-500">
                Video and audio settings apply the next time recording starts.
              </p>
            </div>
          )}
        </SettingCard>

        {/* Danger zone */}
        <SettingCard className="border-red-900/40 bg-red-950/10 p-4">
          <div className="flex items-start justify-between gap-3">
            <GroupTitle
              icon={AlertTriangle}
              title="Reset recording settings"
              description="Restore every recording option to its default."
            />
            <Button
              variant="destructive"
              size="sm"
              type="button"
              onClick={() => setConfirmResetOpen(true)}
              className="shrink-0"
            >
              Reset
            </Button>
          </div>
        </SettingCard>
      </div>

      <Dialog
        open={confirmResetOpen}
        onClose={() => setConfirmResetOpen(false)}
        title="Reset recording settings?"
        description="Clip length, quality, capture, and storage options will return to their defaults. This can't be undone."
        footer={
          <>
            <Button
              variant="ghost"
              size="sm"
              type="button"
              onClick={() => setConfirmResetOpen(false)}
            >
              Cancel
            </Button>
            <Button variant="destructive" size="sm" type="button" onClick={resetRecording}>
              Reset settings
            </Button>
          </>
        }
      />
    </section>
  );
}