"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { X, Link, Check, Globe, Lock, EyeOff } from "lucide-react";
import { Button } from "@/components/ui";
import { updateClipVisibility } from "@/lib/api";

const visibilityOptions = [
  { value: "public" as const, label: "Public", icon: Globe, description: "Anyone can see this clip" },
  { value: "unlisted" as const, label: "Unlisted", icon: EyeOff, description: "Only people with the link can see it" },
  { value: "private" as const, label: "Private", icon: Lock, description: "Only you can see this clip" },
];

export function ShareModal({
  clipId,
  shareUrl,
  currentVisibility,
  onClose,
}: {
  clipId: string;
  shareUrl: string;
  currentVisibility: string;
  onClose: () => void;
}) {
  const router = useRouter();
  const [visibility, setVisibility] = useState(currentVisibility);
  const [copied, setCopied] = useState(false);
  const [saving, setSaving] = useState(false);

  const fullShareUrl = `${window.location.origin}${shareUrl}`;

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(fullShareUrl);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      const input = document.getElementById("share-link-input") as HTMLInputElement;
      if (input) {
        input.select();
        document.execCommand("copy");
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
      }
    }
  };

  const handleVisibilityChange = async (newVisibility: string) => {
    if (newVisibility === visibility) return;
    setSaving(true);
    try {
      await updateClipVisibility(clipId, newVisibility as "public" | "private" | "unlisted");
      setVisibility(newVisibility);
      router.refresh();
    } catch {
      alert("Failed to update visibility");
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" onClick={onClose}>
      <div className="w-full max-w-md rounded-3xl border border-border bg-[linear-gradient(180deg,rgba(16,25,46,0.98),rgba(8,13,26,0.98))] p-6 shadow-xl shadow-black/30" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center justify-between">
          <h3 className="text-lg font-semibold text-white">Share clip</h3>
          <button onClick={onClose} className="flex size-8 items-center justify-center rounded-lg text-zinc-400 transition hover:bg-white/5 hover:text-white">
            <X className="size-4 shrink-0" />
          </button>
        </div>

        <div className="mt-5 space-y-1">
          <label className="text-sm text-zinc-400">Share link</label>
          <div className="flex gap-2">
            <input
              id="share-link-input"
              readOnly
              value={fullShareUrl}
              className="min-w-0 flex-1 rounded-xl border border-border bg-[#09111f] px-4 py-3 text-sm text-white"
            />
            <Button onClick={handleCopy} variant="secondary" className="shrink-0">
              {copied ? <Check className="size-4 shrink-0 text-green-400" /> : <Link className="size-4 shrink-0" />}
              {copied ? "Copied" : "Copy"}
            </Button>
          </div>
        </div>

        <div className="mt-6 space-y-1">
          <label className="text-sm text-zinc-400">Visibility</label>
          <div className="space-y-2">
            {visibilityOptions.map((opt) => {
              const Icon = opt.icon;
              const selected = visibility === opt.value;
              return (
                <button
                  key={opt.value}
                  onClick={() => handleVisibilityChange(opt.value)}
                  disabled={saving}
                  className={`flex w-full items-center gap-3 rounded-2xl border p-4 text-left transition ${
                    selected
                      ? "border-blue-400/40 bg-blue-500/10"
                      : "border-border bg-white/[0.03] hover:bg-white/[0.06]"
                  }`}
                >
                  <Icon className={`size-5 shrink-0 ${selected ? "text-blue-300" : "text-zinc-400"}`} />
                  <div className="min-w-0 flex-1">
                    <div className={`text-sm font-medium ${selected ? "text-white" : "text-zinc-300"}`}>{opt.label}</div>
                    <div className="text-xs text-zinc-500">{opt.description}</div>
                  </div>
                  {selected && <div className="size-2 rounded-full bg-blue-400" />}
                </button>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}
