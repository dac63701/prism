import { SettingRow, SectionHeading } from "@/components/settings/SettingRow";
import { Switch } from "@/components/ui/switch";
import { Input } from "@/components/ui/input";
import { useSettingsActions } from "@/hooks/useSettingsActions";

export default function GeneralSection() {
  const { settings, setField } = useSettingsActions();
  const s = settings.general;

  return (
    <section>
      <div className="mb-3 space-y-1">
        <span className="text-xs uppercase tracking-[0.28em] text-blue-300/70">GENERAL</span>
        <SectionHeading>General</SectionHeading>
      </div>
      <div className="mt-3 space-y-1 border-t border-border pt-3">
        <SettingRow label="Launch at startup">
          <Switch
            ariaLabel="Launch at startup"
            checked={s.launch_at_startup}
            onChange={(checked) => setField("general", "launch_at_startup", checked as never)}
          />
        </SettingRow>

        <SettingRow label="Minimize to tray">
          <Switch
            ariaLabel="Minimize to tray"
            checked={s.minimize_to_tray}
            onChange={(checked) => setField("general", "minimize_to_tray", checked as never)}
          />
        </SettingRow>

        <SettingRow
          label="Show clip notification"
          help="Shows a notification when a clip is saved, even while other apps are in focus."
        >
          <Switch
            ariaLabel="Show clip notification"
            checked={s.show_clip_notification}
            onChange={(checked) => setField("general", "show_clip_notification", checked as never)}
          />
        </SettingRow>

        <SettingRow label="Game detection">
          <Switch
            ariaLabel="Game detection"
            checked={s.game_detection_enabled}
            onChange={(checked) => setField("general", "game_detection_enabled", checked as never)}
          />
        </SettingRow>

        <SettingRow label="CS2 GSI port" help="Restart Prism after changing the CS2 GSI port.">
          <Input
            type="number"
            min={1024}
            max={65535}
            value={s.cs2_gsi_port}
            onChange={(e) =>
              setField(
                "general",
                "cs2_gsi_port",
                (parseInt(e.target.value, 10) || 4000) as never
              )
            }
            className="w-24"
          />
        </SettingRow>
      </div>
    </section>
  );
}