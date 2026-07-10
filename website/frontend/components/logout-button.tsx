"use client";

import { LogOut } from "lucide-react";
import { Button } from "@/components/ui";

export function LogoutButton() {
  return (
    <Button
      variant="secondary"
      className="w-full justify-start"
      onClick={async () => {
        await fetch("/api/auth/logout", { method: "POST", credentials: "include" });
        window.location.href = "/login";
      }}
    >
      <LogOut className="h-4 w-4" />
      Logout
    </Button>
  );
}
