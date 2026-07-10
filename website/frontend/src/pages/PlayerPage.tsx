import { useState, useEffect } from "react";
import { useParams } from "react-router-dom";
import { Film, Copy, Check, Download } from "lucide-react";
import { api } from "@/lib/api";

interface ClipMeta {
  id: string;
  title: string;
  game: string;
  duration_secs: number;
  width: number;
  height: number;
  size_bytes: number;
  codec: string;
  visibility: string;
  created_at: string;
  thumbnail_url?: string;
}

function formatSize(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return (bytes / Math.pow(1024, i)).toFixed(i > 0 ? 1 : 0) + " " + units[i];
}

function formatDuration(secs: number): string {
  const m = Math.floor(secs / 60);
  const s = Math.floor(secs % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}

function formatDate(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleDateString(undefined, {
      month: "short",
      day: "numeric",
      year: "numeric",
    });
  } catch {
    return iso;
  }
}

export default function PlayerPage() {
  const { shareId } = useParams<{ shareId: string }>();
  const [meta, setMeta] = useState<ClipMeta | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!shareId) return;
    setLoading(true);
    api
      .get<ClipMeta>(`/api/s/${shareId}/meta`)
      .then((data) => setMeta(data))
      .catch((err) => setError(err.message))
      .finally(() => setLoading(false));
  }, [shareId]);

  const copyLink = () => {
    navigator.clipboard.writeText(window.location.href).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  };

  if (loading) {
    return (
      <div className="h-screen flex items-center justify-center bg-zinc-950 text-zinc-500 text-sm">
        Loading clip...
      </div>
    );
  }

  if (error || !meta) {
    return (
      <div className="h-screen flex flex-col items-center justify-center bg-zinc-950 text-zinc-500 gap-3">
        <Film className="size-12 text-zinc-700" />
        <p className="text-sm">{error || "Clip not found"}</p>
        <p className="text-xs text-zinc-700">
          This clip may be private or doesn't exist.
        </p>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-zinc-950 flex flex-col items-center py-8 px-4">
      <div className="w-full max-w-4xl">
        {/* Header */}
        <div className="text-center mb-6">
          <h1 className="text-lg font-semibold text-zinc-100">
            {meta.title || "Untitled Clip"}
          </h1>
          {meta.game && (
            <p className="text-sm text-zinc-500 mt-1">{meta.game}</p>
          )}
        </div>

        {/* Video Player */}
        <div className="aspect-video bg-black rounded-xl overflow-hidden flex items-center justify-center border border-zinc-800">
          <div className="text-center text-zinc-600">
            <Film className="size-12 mx-auto mb-3 text-zinc-700" />
            <p className="text-sm">Video playback requires server-side streaming</p>
            <p className="text-xs text-zinc-700 mt-1">
              Download the clip to view it locally
            </p>
          </div>
        </div>

        {/* Info Bar */}
        <div className="mt-4 flex flex-wrap items-center gap-4 text-xs text-zinc-500 justify-center">
          <span>{formatDate(meta.created_at)}</span>
          <span className="text-zinc-700">·</span>
          <span>{formatDuration(meta.duration_secs)}</span>
          <span className="text-zinc-700">·</span>
          <span>{formatSize(meta.size_bytes)}</span>
          {meta.width > 0 && (
            <>
              <span className="text-zinc-700">·</span>
              <span>
                {meta.width}×{meta.height}
              </span>
            </>
          )}
          {meta.codec && (
            <>
              <span className="text-zinc-700">·</span>
              <span>{meta.codec.toUpperCase()}</span>
            </>
          )}
        </div>

        {/* Actions */}
        <div className="flex items-center justify-center gap-3 mt-6">
          <button
            onClick={copyLink}
            className="flex items-center gap-2 px-4 py-2 bg-zinc-900 border border-zinc-800 rounded-lg text-sm text-zinc-400 hover:text-zinc-200 hover:bg-zinc-800 transition-colors"
          >
            {copied ? (
              <>
                <Check className="size-4 text-green-400" />
                Copied!
              </>
            ) : (
              <>
                <Copy className="size-4" />
                Copy Link
              </>
            )}
          </button>
        </div>

        {/* Footer */}
        <div className="text-center mt-12">
          <p className="text-xs text-zinc-700">
            Shared via <span className="text-zinc-500">Prism</span>
          </p>
        </div>
      </div>
    </div>
  );
}
