"use client";

import { useState } from "react";
import { Share2 } from "lucide-react";
import { Button } from "@/components/ui";
import { ShareModal } from "@/components/share-modal";

export function ShareButton({
  clipId,
  shareUrl,
  currentVisibility,
}: {
  clipId: string;
  shareUrl: string;
  currentVisibility: string;
}) {
  const [open, setOpen] = useState(false);

  return (
    <>
      <Button variant="secondary" onClick={() => setOpen(true)}>
        <Share2 className="size-4 shrink-0" />
        Share
      </Button>
      {open && (
        <ShareModal
          clipId={clipId}
          shareUrl={shareUrl}
          currentVisibility={currentVisibility}
          onClose={() => setOpen(false)}
        />
      )}
    </>
  );
}
