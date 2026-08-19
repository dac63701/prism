import { useLocation } from "react-router-dom";

export function useRouteTitle(): string {
  const { pathname } = useLocation();
  if (pathname.startsWith("/clip/")) return "Clip details";
  if (pathname.startsWith("/library")) return "Library";
  if (pathname.startsWith("/settings")) return "Settings";
  return "Home";
}