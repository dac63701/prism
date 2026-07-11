import { cn } from "@/lib/utils";

export default function PrismLogo({ className }: { className?: string }) {
  return (
    <span
      className={cn(
        "inline-flex h-10 w-10 items-center justify-center",
        className
      )}
      aria-hidden="true"
    >
      <img
        src="/logo.svg"
        alt="Prism"
        className="h-full w-full object-contain"
      />
    </span>
  );
}
