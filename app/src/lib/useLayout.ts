import { useEffect, useState } from "react";

// Width, not platform(): a landscape iPad is wide, a desktop window shrunk
// to 380px should behave like a phone. See CLAUDE.md "Mobile-first".
const WIDE_QUERY = "(min-width: 700px)";

export function useIsWide(): boolean {
  const [isWide, setIsWide] = useState(() => window.matchMedia(WIDE_QUERY).matches);

  useEffect(() => {
    const mql = window.matchMedia(WIDE_QUERY);
    const onChange = (e: MediaQueryListEvent) => setIsWide(e.matches);
    mql.addEventListener("change", onChange);
    return () => mql.removeEventListener("change", onChange);
  }, []);

  return isWide;
}
