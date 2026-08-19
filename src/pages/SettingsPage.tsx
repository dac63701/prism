import { useEffect, useState } from "react";
import {
  Video,
  Keyboard,
  Settings2,
  Sparkles,
  Cloud,
  Database,
} from "lucide-react";
import { useSettingsStore } from "@/stores/settings";
import { Tabs, type TabItem } from "@/components/ui/tabs";
import RecordingSection from "@/components/settings/sections/RecordingSection";
import HotkeysSection from "@/components/settings/sections/HotkeysSection";
import GeneralSection from "@/components/settings/sections/GeneralSection";
import AutoClipSection from "@/components/settings/sections/AutoClipSection";
import CloudSection from "@/components/settings/sections/CloudSection";
import StorageSection from "@/components/settings/sections/StorageSection";

type SectionKey = "recording" | "hotkeys" | "general" | "autoclip" | "cloud" | "storage";

const SECTIONS: TabItem<SectionKey>[] = [
  { value: "recording", label: "Recording", icon: Video },
  { value: "hotkeys", label: "Hotkeys", icon: Keyboard },
  { value: "general", label: "General", icon: Settings2 },
  { value: "autoclip", label: "Auto-clip", icon: Sparkles },
  { value: "cloud", label: "Cloud", icon: Cloud },
  { value: "storage", label: "Storage", icon: Database },
];

export default function SettingsPage() {
  const loadSettings = useSettingsStore((s) => s.loadSettings);
  const loaded = useSettingsStore((s) => s.loaded);
  const [section, setSection] = useState<SectionKey>("recording");

  useEffect(() => {
    if (!loaded) loadSettings();
  }, [loaded, loadSettings]);

  return (
    <div className="h-full overflow-y-auto px-6 py-6">
      <div className="mb-6">
        <h1 className="text-xl font-semibold tracking-tight text-zinc-100">Settings</h1>
        <p className="mt-1 text-sm text-zinc-500">Changes are saved automatically.</p>
      </div>

      <div className="flex flex-col gap-6 md:flex-row md:gap-8">
        {/* Section rail — vertical on md+, horizontal chips below */}
        <div className="shrink-0 md:w-44">
          <Tabs
            value={section}
            items={SECTIONS}
            onChange={setSection}
            orientation="vertical"
            className="hidden md:flex"
          />
          <Tabs
            value={section}
            items={SECTIONS}
            onChange={setSection}
            orientation="horizontal"
            className="flex flex-wrap md:hidden"
          />
        </div>

        <div className="min-w-0 flex-1">
          <div key={section} className="animate-fade-up">
            {section === "recording" && <RecordingSection />}
            {section === "hotkeys" && <HotkeysSection />}
            {section === "general" && <GeneralSection />}
            {section === "autoclip" && <AutoClipSection />}
            {section === "cloud" && <CloudSection />}
            {section === "storage" && <StorageSection />}
          </div>
        </div>
      </div>
    </div>
  );
}