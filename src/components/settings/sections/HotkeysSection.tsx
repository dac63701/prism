import { Button } from "@/components/ui/button";
import { SettingRow, SectionHeading } from "@/components/settings/SettingRow";
import HotkeyCaptureInput from "@/components/settings/HotkeyCaptureInput";
import { useSettingsActions } from "@/hooks/useSettingsActions";

export default function HotkeysSection() {
  const { settings, setField, resetHotkeys } = useSettingsActions();
  const s = settings.hotkeys;

  return (
    <section>
      <div className="mb-3 space-y-1">
        <span className="text-xs uppercase tracking-[0.28em] text-blue-300/70">HOTKEYS</span>
        <SectionHeading>Hotkeys</SectionHeading>
      </div>
      <div className="mt-3 space-y-1 border-t border-border pt-3">
        <SettingRow label="Save clip">
          <HotkeyCaptureInput
            value={s.save_clip}
            onChange={async (value) => {
              await setField("hotkeys", "save_clip", value as never);
            }}
          />
        </SettingRow>

        <SettingRow label="Toggle recording">
          <HotkeyCaptureInput
            value={s.toggle_recording}
            onChange={async (value) => {
              await setField("hotkeys", "toggle_recording", value as never);
            }}
          />
        </SettingRow>

        <SettingRow label="Open library">
          <HotkeyCaptureInput
            value={s.open_library}
            onChange={async (value) => {
              await setField("hotkeys", "open_library", value as never);
            }}
          />
        </SettingRow>

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
  );
}