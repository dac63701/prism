import { useState, useEffect } from "react";
import { useParams, useNavigate } from "react-router-dom";
import {
  ArrowLeft,
  Play,
  Pause,
  Film,
  Copy,
  Check,
  Trash2,
} from "lucide-react";
import { api } from "@/lib/api";

interface ClipDetail {
  id: string;
  title: string;
  game: string;
  tags: string[];
  duration_secs: number;
  size_bytes: number;
  width: number;
  height: number;
  codec: string;
  visibility: string;
  share_url: string;
  original_filename: string;
  download_count: number;
  created_at: string;
  updated_at: string;
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
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return iso;
  }
}

export default function ClipDetailPage() {
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [clip, setClip] = useState<ClipDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [copied, setCopied] = useState(false);
  const [title, setTitle] = useState("");
  const [game, setGame] = useState("");
  const [visibility, setVisibility] = useState("unlisted");
  const [saving, setSaving] = useState(false);
  const [tags, setTags] = useState<string[]>([]);
  const [tagInput, setTagInput] = useState("");

  useEffect(() => {
    if (!id) return;
    setLoading(true);
    api
      .get<ClipDetail>(`/api/clips/${id}`)
      .then((data) => {
        setClip(data);
        setTitle(data.title);
        setGame(data.game);
        setVisibility(data.visibility);
        setTags(data.tags || []);
      })
      .catch((err) => setError(err.message))
      .finally(() => setLoading(false));
  }, [id]);

  const handleDelete = async () => {
    if (!id || !confirm("Delete this clip permanently?")) return;
    try {
      await api.delete(`/api/clips/${id}`);
      navigate("/library");
    } catch (err: any) {
      alert(err.message);
    }
  };

  const copyShareUrl = () => {
    if (!clip) return;
    const url = `${window.location.origin}${clip.share_url}`;
    navigator.clipboard.writeText(url).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    });
  };

  if (loading) {
    return (
      <div className="h-full flex items-center justify-center text-zinc-500 text-sm">
        Loading clip...
      </div>
    );
  }

  if (error || !clip) {
    return (
      <div className="h-full flex flex-col items-center justify-center text-zinc-500 gap-3">
        <Film className="size-10 text-zinc-700" />
        <p className="text-sm">{error || "Clip not found"}</p>
        <button
          onClick={() => navigate("/library")}
          className="text-xs text-zinc-400 hover:text-zinc-200"
        >
          Back to library
        </button>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col">
      <header className="flex items-center gap-3 px-6 pt-5 pb-3">
        <button
          onClick={() => navigate("/library")}
          className="p-1.5 rounded-lg text-zinc-500 hover:text-zinc-200 hover:bg-zinc-800 transition-colors"
        >
          <ArrowLeft className="size-5" />
        </button>
        <div className="flex-1 min-w-0">
          <h1 className="text-sm font-medium text-zinc-100 truncate">
            {clip.title || clip.original_filename}
          </h1>
        </div>
        <button
          onClick={handleDelete}
          className="p-1.5 rounded-lg text-zinc-500 hover:text-red-300 hover:bg-red-950/60 transition-colors"
          title="Delete clip"
        >
          <Trash2 className="size-4" />
        </button>
      </header>

      <div className="flex-1 flex gap-6 px-6 pb-6 overflow-y-auto">
        {/* Video Player */}
        <div className="flex-1">
          <div className="aspect-video bg-black rounded-xl overflow-hidden flex items-center justify-center">
            <p className="text-zinc-600 text-sm">
              Video playback requires server-side streaming setup
            </p>
          </div>

          {/* Metadata */}
          <div className="mt-4 flex items-center gap-4 text-xs text-zinc-500">
            <span>{formatDate(clip.created_at)}</span>
            <span>·</span>
            <span>{formatDuration(clip.duration_secs)}</span>
            <span>·</span>
            <span>{formatSize(clip.size_bytes)}</span>
            <span>·</span>
            <span>{clip.width}×{clip.height}</span>
            <span>·</span>
            <span>{clip.codec?.toUpperCase()}</span>
            <span>·</span>
            <span>{clip.download_count} download{clip.download_count !== 1 ? "s" : ""}</span>
          </div>
        </div>

        {/* Edit Panel */}
        <div className="w-72 shrink-0 space-y-4">
          <div className="rounded-xl bg-zinc-900 border border-zinc-800 p-4 space-y-3">
            <h3 className="text-xs font-medium text-zinc-500 uppercase tracking-wider">
              Details
            </h3>

            <div>
              <label className="block text-xs text-zinc-500 mb-1">Title</label>
              <input
                type="text"
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-2.5 py-1.5 text-sm text-zinc-100 focus:outline-none focus:ring-1 focus:ring-zinc-600"
              />
            </div>

            <div>
              <label className="block text-xs text-zinc-500 mb-1">Game</label>
              <input
                type="text"
                value={game}
                onChange={(e) => setGame(e.target.value)}
                className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-2.5 py-1.5 text-sm text-zinc-100 focus:outline-none focus:ring-1 focus:ring-zinc-600"
              />
            </div>

            <div>
              <label className="block text-xs text-zinc-500 mb-1">Visibility</label>
              <select
                value={visibility}
                onChange={(e) => setVisibility(e.target.value)}
                className="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-2.5 py-1.5 text-sm text-zinc-100 focus:outline-none focus:ring-1 focus:ring-zinc-600"
              >
                <option value="public">Public</option>
                <option value="unlisted">Unlisted (anyone with link)</option>
                <option value="private">Private</option>
              </select>
            </div>

            <div>
              <label className="block text-xs text-zinc-500 mb-1">Tags</label>
              <div className="flex flex-wrap gap-1.5 mb-2">
                {tags.map((tag) => (
                  <span key={tag} className="inline-flex items-center gap-1 px-2 py-0.5 rounded bg-zinc-800 text-xs text-zinc-300">
                    {tag}
                    <button
                      onClick={() => setTags(tags.filter((t) => t !== tag))}
                      className="text-zinc-600 hover:text-zinc-300"
                    >
                      ×
                    </button>
                  </span>
                ))}
              </div>
              <form
                onSubmit={(e) => {
                  e.preventDefault();
                  const trimmed = tagInput.trim().toLowerCase();
                  if (trimmed && !tags.includes(trimmed)) {
                    setTags([...tags, trimmed]);
                  }
                  setTagInput("");
                }}
                className="flex gap-2"
              >
                <input
                  type="text"
                  value={tagInput}
                  onChange={(e) => setTagInput(e.target.value)}
                  placeholder="Add tag..."
                  className="flex-1 bg-zinc-950 border border-zinc-800 rounded-lg px-2.5 py-1 text-xs text-zinc-100 placeholder-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-600"
                />
                <button
                  type="submit"
                  className="px-2 py-1 rounded bg-zinc-800 text-xs text-zinc-300 hover:bg-zinc-700"
                >
                  +
                </button>
              </form>
            </div>

            <button
              onClick={async () => {
                setSaving(true);
                try {
                  await Promise.all([
                    api.patch(`/api/clips/${id}`, { title, game, visibility }),
                    api.put(`/api/clips/${id}/tags`, { tags }),
                  ]);
                  setClip((prev) =>
                    prev ? { ...prev, title, game, visibility, tags } : prev
                  );
                } catch (err: any) {
                  alert(err.message);
                } finally {
                  setSaving(false);
                }
              }}
              disabled={saving}
              className="w-full bg-zinc-100 text-zinc-950 rounded-lg px-3 py-1.5 text-sm font-medium hover:bg-zinc-200 transition-colors disabled:opacity-50"
            >
              {saving ? "Saving..." : "Save changes"}
            </button>
          </div>

          {/* Share URL */}
          <div className="rounded-xl bg-zinc-900 border border-zinc-800 p-4 space-y-2">
            <h3 className="text-xs font-medium text-zinc-500 uppercase tracking-wider">
              Share
            </h3>
            <div className="flex items-center gap-2">
              <input
                type="text"
                readOnly
                value={`${window.location.origin}${clip.share_url}`}
                className="flex-1 bg-zinc-950 border border-zinc-800 rounded-lg px-2.5 py-1.5 text-xs text-zinc-400 font-mono"
              />
              <button
                onClick={copyShareUrl}
                className="p-1.5 rounded-lg text-zinc-500 hover:text-zinc-200 hover:bg-zinc-800 transition-colors"
              >
                {copied ? <Check className="size-4 text-green-400" /> : <Copy className="size-4" />}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
