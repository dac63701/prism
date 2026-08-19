import { useCallback, useEffect } from "react";
import { Loader2, Upload, RefreshCw, Globe } from "lucide-react";
import { Dialog } from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import PrismLogo from "@/components/common/PrismLogo";
import { useCloudStore } from "@/stores/cloud";
import { useSettingsStore } from "@/stores/settings";

const FEATURES = [
  { icon: Upload, label: "Upload & share clips with a single link" },
  { icon: RefreshCw, label: "Auto-upload highlights as you play" },
  { icon: Globe, label: "Access your clips from anywhere" },
];

interface SignInPromptProps {
  open: boolean;
  onClose: () => void;
}

export default function SignInPrompt({ open, onClose }: SignInPromptProps) {
  const authenticated = useCloudStore((s) => s.authenticated);
  const loading = useCloudStore((s) => s.loading);
  const uploadError = useCloudStore((s) => s.uploadError);
  const clearUploadError = useCloudStore((s) => s.clearUploadError);
  const login = useCloudStore((s) => s.login);

  // Close automatically once the OAuth flow completes.
  useEffect(() => {
    if (open && authenticated) onClose();
  }, [open, authenticated, onClose]);

  // Reset transient error when the dialog (re)opens.
  useEffect(() => {
    if (open) clearUploadError();
  }, [open, clearUploadError]);

  const dismiss = useCallback(() => {
    const { settings, updateSettings } = useSettingsStore.getState();
    if (!authenticated) {
      void updateSettings({
        ...settings,
        general: {
          ...settings.general,
          sign_in_prompt_dismissed_at: Math.floor(Date.now() / 1000),
        },
      });
    }
    onClose();
  }, [authenticated, onClose]);

  if (!open) return null;

  return (
    <Dialog open={open} onClose={dismiss} className="max-w-sm">
      <div className="flex flex-col items-center gap-5 text-center">
        <div className="mt-1 flex flex-col items-center gap-3">
          <PrismLogo className="h-14 w-14" />
          <div>
            <h2 className="text-xl font-semibold tracking-tight text-white">
              Welcome to Prism
            </h2>
            <p className="mt-1 text-sm leading-6 text-zinc-400">
              Want to take your clips to the cloud? Sign in with your Google
              account to upload, share, and sync.
            </p>
          </div>
        </div>

        <ul className="w-full space-y-2">
          {FEATURES.map(({ icon: Icon, label }) => (
            <li
              key={label}
              className="flex items-center gap-2.5 rounded-lg border border-border bg-white/[0.03] px-3 py-2 text-left"
            >
              <Icon className="size-4 shrink-0 text-blue-300" />
              <span className="text-sm text-zinc-300">{label}</span>
            </li>
          ))}
        </ul>

        {loading ? (
          <div className="flex w-full flex-col items-center gap-3 py-1">
            <div className="flex items-center gap-2.5 text-sm text-zinc-300">
              <Loader2 className="size-4 animate-spin text-blue-300" />
              Waiting for sign-in…
            </div>
            <p className="text-xs leading-5 text-zinc-500">
              If a browser window didn't open, check your browser — you may
              need to allow Prism to open external links.
            </p>
            <Button variant="ghost" size="sm" type="button" onClick={dismiss}>
              Cancel
            </Button>
          </div>
        ) : (
          <div className="flex w-full flex-col gap-2">
            {uploadError && (
              <div className="rounded-lg border border-red-900/60 bg-red-950/60 px-3 py-2">
                <p className="text-xs text-red-300">{uploadError}</p>
              </div>
            )}
            <Button
              variant="brand"
              size="lg"
              type="button"
              className="w-full"
              onClick={() => {
                void login();
              }}
            >
              Sign in with Google
            </Button>
            <Button
              variant="ghost"
              size="sm"
              type="button"
              className="w-full text-zinc-500 hover:text-zinc-300"
              onClick={dismiss}
            >
              Not now
            </Button>
          </div>
        )}
      </div>
    </Dialog>
  );
}