import { useState } from "react";
import { User } from "lucide-react";
import { cn } from "@/lib/utils";

function getInitials(name: string) {
  return name
    .split(/\s+/)
    .map((part) => part[0])
    .filter(Boolean)
    .slice(0, 2)
    .join("")
    .toUpperCase();
}

export default function AccountAvatar({
  src,
  name,
  size = "size-8",
}: {
  src: string;
  name: string;
  size?: string;
}) {
  const [failed, setFailed] = useState(false);

  if (src && !failed) {
    return (
      <img
        src={src}
        alt={name || "Account"}
        onError={() => setFailed(true)}
        className={cn("shrink-0 rounded-full object-cover", size)}
      />
    );
  }
  const initials = getInitials(name);
  return (
    <span
      className={cn(
        "flex shrink-0 items-center justify-center rounded-full bg-accent/15 text-xs font-semibold text-blue-300",
        size
      )}
    >
      {initials || <User className="size-4" />}
    </span>
  );
}