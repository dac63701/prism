"use client";

import { useRouter } from "next/navigation";
import { Trash2 } from "lucide-react";
import { deleteClip } from "@/lib/api";
import { Button } from "@/components/ui";
import { useState } from "react";

export function DeleteClipButton({
  clipId,
  clipTitle,
  redirectTo,
  compact,
  onDeleted,
}: {
  clipId: string;
  clipTitle: string;
  redirectTo?: string;
  compact?: boolean;
  onDeleted?: () => void;
}) {
  const router = useRouter();
  const [deleting, setDeleting] = useState(false);

  const handleDelete = async () => {
    if (!window.confirm(`Delete "${clipTitle}"? This cannot be undone.`)) return;

    setDeleting(true);
    try {
      await deleteClip(clipId);
      if (redirectTo) {
        router.push(redirectTo);
      } else {
        router.refresh();
      }
      onDeleted?.();
    } catch {
      setDeleting(false);
      alert("Failed to delete clip.");
    }
  };

  if (compact) {
    return (
      <button
        onClick={(e) => { e.stopPropagation(); handleDelete(); }}
        disabled={deleting}
        className="absolute right-2 top-2 flex size-8 items-center justify-center rounded-lg bg-black/60 text-zinc-400 opacity-0 transition hover:bg-red-500/80 hover:text-white group-hover:opacity-100"
        title={`Delete ${clipTitle}`}
      >
        <Trash2 className="size-4 shrink-0" />
      </button>
    );
  }

  return (
    <Button variant="secondary" onClick={handleDelete} disabled={deleting} className="text-red-400 hover:border-red-500/40 hover:bg-red-500/10 hover:text-red-300">
      <Trash2 className="size-4 shrink-0" />
      {deleting ? "Deleting..." : "Delete clip"}
    </Button>
  );
}
