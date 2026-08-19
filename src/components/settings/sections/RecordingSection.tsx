import { SettingRow, SectionHeading } from "@/components/settings/SettingRow";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import { Input } from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import PresetSlider from "@/components/settings/PresetSlider";
import type { PresetOption } from "@/components/settings/PresetSlider";
import { useSettingsActions } from "@/hooks/useSettingsActions";
import { useSettingsStore } from "@/stores/settings";
import { useDisplayRefreshRate } from "@/hooks/useDisplayRefreshRate";

const RESOLUTION_OPTIONS = [
  { value: "native", label: "Native" },
  { value: "720p", label: "720p" },
  { value: "1080p", label: "1080p" },
  { value: "1440p", label: "1440p" },
  { value: "2160p", label: "4K" },
] as const;

const FPS_OPTIONS = [24, 30, 60, 120, 144] as const;

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

export default function RecordingSection() {
  const { settings, setField, save } = useSettingsActions();
  const loaded = useSettingsStore((s) => s.loaded);
  const s = settings.recording;

  const refreshRate = useDisplayRefreshRate();

  // `0` is reserved for the Auto (match display) preset.
  const fpsOptions: PresetOption[] = [
    { value: 0, label: refreshRate > 0 ? `Auto · ${refreshRate} Hz` : "Auto" },
    ...FPS_OPTIONS.map((fps) => ({ value: fps, label: `${fps}` })),
  ];

  return (
    <section>
      <div className="mb-3 space-y-1">
        <span className="text-xs uppercase tracking-[0.28em] text-blue-300/70">RECORDING</span>
        <SectionHeading>Recording</SectionHeading>
      </div>
      <div className="mt-3 space-y-1 border-t border-border pt-3">
        <SettingRow label="Clip length">
          <Slider
            ariaLabel="Clip length in seconds"
            value={s.buffer_duration_secs}
            min={10}
            max={1800}
            step={5}
            onChange={(value) => setField("recording", "buffer_duration_secs", value as never)}
          />
          <span className="shrink-0 whitespace-nowrap text-right text-sm tabular-nums text-zinc-100">
            {s.buffer_duration_secs}s
          </span>
        </SettingRow>

        <SettingRow
          label="FPS"
          help="Auto matches your display's refresh rate. Higher FPS = smoother clips, larger files."
          className="flex-wrap"
        >
          <PresetSlider
            value={s.fps_auto ? 0 : s.fps}
            options={fpsOptions}
            className="w-full max-w-[20rem]"
            onChange={(value) => {
              if (value === 0) {
                setField("recording", "fps_auto", true);
              } else {
                setField("recording", "fps", value as never);
                setField("recording", "fps_auto", false);
              }
            }}
          />
        </SettingRow>

        <SettingRow label="Resolution">
          <Select
            value={s.resolution}
            onChange={(e) => setField("recording", "resolution", e.target.value as never)}
            aria-label="Resolution"
          >
            {RESOLUTION_OPTIONS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </Select>
        </SettingRow>

        <SettingRow label="Bitrate">
          <PresetSlider
            value={s.bitrate_kbps}
            options={BITRATE_OPTIONS}
            onChange={(value) => setField("recording", "bitrate_kbps", value as never)}
          />
        </SettingRow>

        <SettingRow label="Output directory">
          <Input
            key={loaded ? "output-loaded" : "output-initial"}
            defaultValue={s.output_directory}
            placeholder="~/Videos/Prism"
            onChange={(e) =>
              save({
                ...settings,
                recording: { ...settings.recording, output_directory: e.target.value },
              })
            }
            className="w-64"
          />
        </SettingRow>

        <SettingRow label="Always-on recording">
          <Switch
            ariaLabel="Always-on recording"
            checked={s.always_on_recording}
            onChange={(checked) => setField("recording", "always_on_recording", checked as never)}
          />
        </SettingRow>

        <SettingRow
          label="System audio"
          help="Record system sounds in clips (Windows). Turn off to save clips without an audio track."
        >
          <Switch
            ariaLabel="System audio"
            checked={s.capture_audio}
            onChange={(checked) => setField("recording", "capture_audio", checked as never)}
          />
        </SettingRow>
      </div>
    </section>
  );
}