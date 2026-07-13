"use client";

import { useState, useRef, useEffect } from "react";
import { useRouter } from "next/navigation";
import { Pencil, Check, X } from "lucide-react";
import { updateClipName } from "@/lib/api";

export function ClipRename({ clipId, initialTitle }: { clipId: string; initialTitle: string }) {
  const router = useRouter();
  const [editing, setEditing] = useState(false);
  const [title, setTitle] = useState(initialTitle);
  const [saving, setSaving] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (editing && inputRef.current) {
      inputRef.current.focus();
      inputRef.current.select();
    }
  }, [editing]);

  const handleSave = async () => {
    const trimmed = title.trim();
    if (!trimmed) {
      setTitle(initialTitle);
      setEditing(false);
      return;
    }
    if (trimmed === initialTitle) {
      setEditing(false);
      return;
    }

    setSaving(true);
    try {
      await updateClipName(clipId, trimmed);
      router.refresh();
      setEditing(false);
    } catch {
      setTitle(initialTitle);
      alert("Failed to rename clip");
    } finally {
      setSaving(false);
    }
  };

  const handleCancel = () => {
    setTitle(initialTitle);
    setEditing(false);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      handleSave();
    } else if (e.key === "Escape") {
      handleCancel();
    }
  };

  if (editing) {
    return (
      <div className="flex items-center gap-2">
        <input
          ref={inputRef}
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          onKeyDown={handleKeyDown}
          disabled={saving}
          className="min-w-0 flex-1 rounded-xl border border-blue-400/40 bg-[#09111f] px-3 py-2 text-2xl font-semibold tracking-tight text-white outline-none ring-2 ring-blue-500/20"
        />
        <button
          onClick={handleSave}
          disabled={saving}
          className="flex size-9 items-center justify-center rounded-lg text-zinc-400 transition hover:bg-white/5 hover:text-green-400"
        >
          <Check className="size-4 shrink-0" />
        </button>
        <button
          onClick={handleCancel}
          disabled={saving}
          className="flex size-9 items-center justify-center rounded-lg text-zinc-400 transition hover:bg-white/5 hover:text-red-400"
        >
          <X className="size-4 shrink-0" />
        </button>
      </div>
    );
  }

  return (
    <div className="group flex items-center gap-2">
      <h2 className="text-2xl font-semibold tracking-tight text-white">{title || "Untitled clip"}</h2>
      <button
        onClick={() => setEditing(true)}
        className="flex size-8 items-center justify-center rounded-lg text-zinc-500 opacity-0 transition hover:bg-white/5 hover:text-blue-300 group-hover:opacity-100"
        title="Rename clip"
      >
        <Pencil className="size-3.5 shrink-0" />
      </button>
    </div>
  );
}
