import { useCallback, useRef } from "react";
import { useSettingsStore, getDefaultHotkeys } from "@/stores/settings";
import type { AppSettings } from "@/types/settings";

export function useSettingsActions() {
  const settings = useSettingsStore((s) => s.settings);
  const updateSettings = useSettingsStore((s) => s.updateSettings);

  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const save = useCallback(
    (next: AppSettings) => updateSettings(next),
    [updateSettings]
  );

  const debouncedSave = useCallback(
    (next: AppSettings) => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
      debounceRef.current = setTimeout(() => {
        void updateSettings(next);
      }, 300);
    },
    [updateSettings]
  );

  const setField = useCallback(
    <S extends keyof AppSettings, K extends keyof AppSettings[S]>(
      section: S,
      key: K,
      value: AppSettings[S][K]
    ) => {
      save({
        ...settings,
        [section]: { ...settings[section], [key]: value },
      });
    },
    [save, settings]
  );

  const resetHotkeys = useCallback(() => {
    return updateSettings({
      ...settings,
      hotkeys: getDefaultHotkeys(),
    });
  }, [updateSettings, settings]);

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
    [save, settings]
  );

  return { settings, setField, save, debouncedSave, resetHotkeys, updateAutoClipGame };
}