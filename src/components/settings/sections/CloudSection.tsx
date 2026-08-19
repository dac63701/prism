import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Input } from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import { SettingRow, SectionHeading } from "@/components/settings/SettingRow";
import { useSettingsActions } from "@/hooks/useSettingsActions";
import { useCloudStore } from "@/stores/cloud";
import { useSettingsStore } from "@/stores/settings";

export default function CloudSection() {
  const { settings, setField, debouncedSave } = useSettingsActions();
  const loaded = useSettingsStore((s) => s.loaded);
  const s = settings.cloud;
  const [showManualCode, setShowManualCode] = useState(false);
  const [authCode, setAuthCode] = useState("");

  const cloudAuthenticated = useCloudStore((st) => st.authenticated);
  const handleAuthCode = useCloudStore((st) => st.handleAuthCode);
  const uploadError = useCloudStore((st) => st.uploadError);
  const cloudLoading = useCloudStore((st) => st.loading);

  return (
    <section>
      <div className="mb-3 space-y-1">
        <span className="text-xs uppercase tracking-[0.28em] text-blue-300/70">CLOUD</span>
        <SectionHeading>Cloud Upload</SectionHeading>
      </div>
      <div className="mt-3 space-y-1 border-t border-border pt-3">
        <SettingRow label="Server URL">
          <Input
            key={loaded ? "server-url-loaded" : "server-url-initial"}
            defaultValue={s.server_url}
            placeholder="https://clips.example.com"
            onChange={(e) =>
              debouncedSave({
                ...settings,
                cloud: { ...settings.cloud, server_url: e.target.value },
              })
            }
            className="w-64"
          />
        </SettingRow>

        <SettingRow label="Account">
          {cloudAuthenticated ? (
            <div className="flex items-center gap-3">
              <div className="text-sm text-zinc-300">
                {s.account_display_name || "Connected"}
                {s.account_email ? (
                  <span className="ml-2 text-xs text-zinc-500">{s.account_email}</span>
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
                <div className="mt-1 flex w-full flex-col gap-2">
                  <Input
                    value={authCode}
                    onChange={(e) => setAuthCode(e.target.value)}
                    placeholder="Paste auth code here..."
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
        </SettingRow>

        {uploadError && (
          <div className="rounded-lg border border-red-900/60 bg-red-950/60 px-4 py-2">
            <p className="text-xs text-red-300">{uploadError}</p>
          </div>
        )}

        <SettingRow label="Auto-upload">
          <Switch
            ariaLabel="Auto-upload clips to cloud"
            checked={s.auto_upload}
            onChange={(checked) => setField("cloud", "auto_upload", checked as never)}
          />
        </SettingRow>

        <SettingRow label="Concurrent uploads">
          <Select
            value={s.max_concurrent_uploads}
            onChange={(e) =>
              setField(
                "cloud",
                "max_concurrent_uploads",
                (parseInt(e.target.value, 10) || 1) as never,
              )
            }
            aria-label="Concurrent uploads"
          >
            <option value={1}>1 (sequential)</option>
            <option value={2}>2</option>
            <option value={3}>3</option>
          </Select>
        </SettingRow>
      </div>
    </section>
  );
}