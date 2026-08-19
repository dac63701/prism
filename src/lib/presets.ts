import type { RecordingSettings } from "@/types/settings";

export type QualityPresetKey = "fast" | "balanced" | "high";

export interface QualityPreset {
  key: QualityPresetKey;
  label: string;
  description: string;
  resolution: string;
  fps: number;
  fps_auto: boolean;
  bitrate_kbps: number;
}

export const QUALITY_PRESETS: QualityPreset[] = [
  {
    key: "fast",
    label: "Fast",
    description: "Lightweight clips, smaller files",
    resolution: "1080p",
    fps: 60,
    fps_auto: true,
    bitrate_kbps: 5000,
  },
  {
    key: "balanced",
    label: "Balanced",
    description: "Best default for sharing",
    resolution: "1080p",
    fps: 60,
    fps_auto: true,
    bitrate_kbps: 8000,
  },
  {
    key: "high",
    label: "High",
    description: "Sharper footage for high-refresh displays",
    resolution: "1440p",
    fps: 60,
    fps_auto: true,
    bitrate_kbps: 16000,
  },
];

export const DEFAULT_QUALITY_PRESET: QualityPresetKey = "balanced";

export function presetByKey(key: string): QualityPreset | undefined {
  return QUALITY_PRESETS.find((preset) => preset.key === key);
}

/**
 * Returns the preset key the current recording settings match, or "custom"
 * when any of resolution / fps / bitrate have been hand-tuned away from a
 * known preset.
 */
export function detectQualityPreset(recording: RecordingSettings): QualityPresetKey | "custom" {
  for (const preset of QUALITY_PRESETS) {
    const fpsMatches = preset.fps_auto
      ? recording.fps_auto
      : !recording.fps_auto && recording.fps === preset.fps;
    if (
      recording.resolution === preset.resolution &&
      recording.bitrate_kbps === preset.bitrate_kbps &&
      fpsMatches
    ) {
      return preset.key;
    }
  }
  return "custom";
}

/**
 * Applies a quality preset to a full set of recording settings, keeping
 * every other field (buffer, audio, output dir, ...) untouched.
 */
export function applyQualityPreset(
  recording: RecordingSettings,
  key: QualityPresetKey
): RecordingSettings {
  const preset = presetByKey(key);
  if (!preset) return { ...recording, quality_preset: "custom" };
  return {
    ...recording,
    resolution: preset.resolution,
    fps: preset.fps,
    fps_auto: preset.fps_auto,
    bitrate_kbps: preset.bitrate_kbps,
    quality_preset: preset.key,
  };
}