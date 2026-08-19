import { useState } from "react";
import {
  UserCircle2,
  UploadCloud,
  Settings2,
  ChevronDown,
  AlertTriangle,
  LogOut,
  KeyRound,
  Loader2,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { Input } from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import AccountAvatar from "@/components/common/AccountAvatar";
import { SettingRow, SectionHeading, SettingCard, GroupTitle } from "@/components/settings/SettingRow";
import { useSettingsActions } from "@/hooks/useSettingsActions";
import { useCloudStore } from "@/stores/cloud";
import { useSettingsStore } from "@/stores/settings";

const VISIBILITY_OPTIONS = [
  { value: "public", label: "Public" },
  { value: "unlisted", label: "Unlisted" },
  { value: "private", label: "Private" },
] as const;

const RETRY_OPTIONS = [
  { value: 0, label: "0 (never retry)" },
  { value: 1, label: "1 retry" },
  { value: 2, label: "2 retries" },
  { value: 3, label: "3 retries" },
  { value: 5, label: "5 retries" },
] as const;

export default function CloudSection() {
  const { settings, setField, debouncedSave } = useSettingsActions();
  const loaded = useSettingsStore((s) => s.loaded);
  const s = settings.cloud;
  const [showManualCode, setShowManualCode] = useState(false);
  const [authCode, setAuthCode] = useState("");
  const [advancedOpen, setAdvancedOpen] = useState(false);

  const cloudAuthenticated = useCloudStore((st) => st.authenticated);
  const handleAuthCode = useCloudStore((st) => st.handleAuthCode);
  const uploadError = useCloudStore((st) => st.uploadError);
  const cloudLoading = useCloudStore((st) => st.loading);

  return (
    <section>
      <div className="mb-4 space-y-1">
        <span className="text-xs uppercase tracking-[0.28em] text-blue-300/70">CLOUD</span>
        <SectionHeading>Cloud Upload</SectionHeading>
      </div>

      <div className="space-y-4">
        {/* Account */}
        <SettingCard className="p-4">
          <GroupTitle
            icon={UserCircle2}
            title="Account"
            description="Sign in to upload clips to the cloud and get share links."
          />
          <div className="mt-3 border-t border-border pt-3">
            {cloudAuthenticated ? (
              <div className="flex items-center justify-between gap-3 rounded-xl border border-border bg-surface px-4 py-3">
                <div className="flex min-w-0 items-center gap-2.5">
                  <AccountAvatar
                    src={s.avatar_url}
                    name={s.account_display_name || s.account_email || "Connected"}
                  />
                  <div className="min-w-0">
                    <p className="truncate text-sm text-zinc-100">
                      {s.account_display_name || "Connected"}
                    </p>
                    {s.account_email ? (
                      <p className="truncate text-xs text-zinc-500">{s.account_email}</p>
                    ) : null}
                  </div>
                </div>
                <Button
                  variant="ghost"
                  size="xs"
                  type="button"
                  onClick={() => useCloudStore.getState().logout()}
                  disabled={cloudLoading}
                  className="shrink-0 text-zinc-500 hover:text-red-400"
                >
                  {cloudLoading ? (
                    <Loader2 className="size-3.5 animate-spin" />
                  ) : (
                    <LogOut className="size-3.5" />
                  )}
                  Sign out
                </Button>
              </div>
            ) : (
              <div className="rounded-xl border border-border bg-surface px-4 py-3">
                <div className="flex flex-wrap items-center justify-between gap-3">
                  <div>
                    <p className="text-sm text-zinc-300">Not signed in</p>
                    <p className="mt-0.5 text-xs text-zinc-500">
                      Uploads stay local until you connect a Prism account.
                    </p>
                  </div>
                  <Button
                    variant="brand"
                    size="sm"
                    type="button"
                    onClick={() => useCloudStore.getState().login()}
                    disabled={cloudLoading}
                    className="shrink-0"
                  >
                    {cloudLoading ? (
                      <>
                        <Loader2 className="size-3.5 animate-spin" />
                        Opening browser...
                      </>
                    ) : (
                      "Sign in with Google"
                    )}
                  </Button>
                </div>
                <button
                  type="button"
                  onClick={() => setShowManualCode((open) => !open)}
                  className="mt-3 inline-flex items-center gap-1.5 text-xs text-zinc-500 transition hover:text-zinc-300 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/20"
                >
                  <KeyRound className="size-3" />
                  Trouble signing in? Paste auth code manually
                </button>
                {showManualCode && (
                  <div className="mt-2 flex flex-col gap-2">
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
          </div>
        </SettingCard>

        {/* Uploads */}
        <SettingCard className="p-4">
          <GroupTitle
            icon={UploadCloud}
            title="Uploads"
            description="How clips are pushed to your cloud."
          />
          {uploadError && (
            <div className="mt-3 flex items-start gap-2 rounded-lg border border-red-900/60 bg-red-950/60 px-3 py-2">
              <AlertTriangle className="mt-0.5 size-3.5 shrink-0 text-red-400" />
              <p className="text-xs text-red-300">{uploadError}</p>
            </div>
          )}
          <div className="mt-3 space-y-1 border-t border-border pt-3">
            <SettingRow label="Auto-upload" help="Upload saved clips to the cloud automatically.">
              <Switch
                ariaLabel="Auto-upload clips to cloud"
                checked={s.auto_upload}
                onChange={(checked) => setField("cloud", "auto_upload", checked as never)}
              />
            </SettingRow>

            <SettingRow
              label="Concurrent uploads"
              help="How many clips can upload at the same time."
              className="flex-wrap"
            >
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

            <SettingRow
              label="Default visibility"
              help="Privacy for new uploads. Change per clip later from the dashboard."
              className="flex-wrap"
            >
              <Select
                value={s.default_visibility}
                onChange={(e) => setField("cloud", "default_visibility", e.target.value as never)}
                aria-label="Default visibility"
              >
                {VISIBILITY_OPTIONS.map((opt) => (
                  <option key={opt.value} value={opt.value}>
                    {opt.label}
                  </option>
                ))}
              </Select>
            </SettingRow>

            <SettingRow
              label="Copy link after upload"
              help="Automatically copy each clip's share link to your clipboard."
            >
              <Switch
                ariaLabel="Copy share link after upload"
                checked={s.copy_share_link_after_upload}
                onChange={(checked) =>
                  setField("cloud", "copy_share_link_after_upload", checked as never)
                }
              />
            </SettingRow>

            <SettingRow
              label="Upload retries"
              help="How many times a failed upload is retried before giving up."
              className="flex-wrap"
            >
              <Select
                value={s.max_upload_retries}
                onChange={(e) =>
                  setField(
                    "cloud",
                    "max_upload_retries",
                    (parseInt(e.target.value, 10) || 0) as never,
                  )
                }
                aria-label="Upload retries"
              >
                {RETRY_OPTIONS.map((opt) => (
                  <option key={opt.value} value={opt.value}>
                    {opt.label}
                  </option>
                ))}
              </Select>
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
              description="Rarely changed cloud details."
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
              <SettingRow
                label="Server URL"
                help="The Prism server clips are uploaded to."
                className="flex-wrap"
              >
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
                  className="w-full md:w-72"
                />
              </SettingRow>
              <p className="pt-1 text-xs text-zinc-500">
                Applies to new uploads. Clips already in the queue keep the URL they were queued
                with.
              </p>
            </div>
          )}
        </SettingCard>
      </div>
    </section>
  );
}