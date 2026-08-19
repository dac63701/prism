export default function AmbientBackground() {
  return (
    <div className="pointer-events-none fixed inset-0 overflow-hidden">
      <div className="absolute -left-40 -top-40 h-[500px] w-[500px] rounded-full bg-blue-500/[0.07] blur-[120px]" />
      <div className="absolute -bottom-40 -right-40 h-[500px] w-[500px] rounded-full bg-blue-600/5 blur-[120px]" />
    </div>
  );
}