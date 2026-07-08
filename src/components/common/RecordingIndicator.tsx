import { useRecordingStore } from "@/stores/recording";

export default function RecordingIndicator() {
  const isRecording = useRecordingStore((s) => s.isRecording);
  const frameCount = useRecordingStore((s) => s.frameCount);

  if (!isRecording) {
    return (
      <div className="flex items-center gap-2 text-xs text-zinc-600">
        <span className="size-1.5 rounded-full bg-zinc-600" />
        Idle
      </div>
    );
  }

  return (
    <div className="flex items-center gap-2 text-xs">
      <span className="size-2 rounded-full bg-red-500 animate-pulse shadow-[0_0_6px_rgba(239,68,68,0.5)]" />
      <span className="font-medium text-zinc-100">Recording</span>
      <span className="text-zinc-600">{frameCount}f</span>
    </div>
  );
}
