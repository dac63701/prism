import { SettingRow, SectionHeading } from "@/components/settings/SettingRow";
import { Input } from "@/components/ui/input";
import { useSettingsActions } from "@/hooks/useSettingsActions";
import { useSettingsStore } from "@/stores/settings";

export default function StorageSection() {
  const { settings, setField, debouncedSave } = useSettingsActions();
  const loaded = useSettingsStore((s) => s.loaded);
  const s = settings.storage;

  return (
    <section>
      <div className="mb-3 space-y-1">
        <span className="text-xs uppercase tracking-[0.28em] text-blue-300/70">STORAGE</span>
        <SectionHeading>Storage</SectionHeading>
      </div>
      <div className="mt-3 space-y-1 border-t border-border pt-3">
        <SettingRow label="Max clips (GB)" help="(0 = unlimited)">
          <Input
            type="number"
            min={0}
            key={loaded ? "max-gb-loaded" : "max-gb-initial"}
            defaultValue={s.max_clips_gb}
            onChange={(e) =>
              setField("storage", "max_clips_gb", (parseInt(e.target.value, 10) || 0) as never)
            }
            className="w-24"
          />
        </SettingRow>

        <SettingRow label="Auto-prune (days)" help="(empty = disabled)">
          <Input
            type="number"
            min={0}
            key={loaded ? "prune-loaded" : "prune-initial"}
            defaultValue={s.auto_prune_days ?? ""}
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
            className="w-24"
          />
        </SettingRow>
      </div>
    </section>
  );
}