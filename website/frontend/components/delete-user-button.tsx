"use client";

import { useRouter } from "next/navigation";
import { Trash2 } from "lucide-react";
import { deleteAdminUser } from "@/lib/api";
import { useState } from "react";

export function DeleteUserButton({
  userId,
  userName,
  onDeleted,
}: {
  userId: string;
  userName: string;
  onDeleted?: () => void;
}) {
  const router = useRouter();
  const [deleting, setDeleting] = useState(false);

  const handleDelete = async () => {
    if (!window.confirm(`Delete user "${userName}"? All clips and data will be permanently removed. This cannot be undone.`)) return;

    setDeleting(true);
    try {
      await deleteAdminUser(userId);
      router.refresh();
      onDeleted?.();
    } catch {
      setDeleting(false);
      alert("Failed to delete user.");
    }
  };

  return (
    <button
      onClick={(e) => { e.preventDefault(); e.stopPropagation(); handleDelete(); }}
      disabled={deleting}
      className="text-xs text-zinc-500 transition hover:text-red-400 disabled:opacity-50"
      title={`Delete ${userName}`}
    >
      {deleting ? "..." : <Trash2 className="size-3.5 shrink-0" />}
    </button>
  );
}
