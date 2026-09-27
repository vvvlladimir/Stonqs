import { useEffect } from "react";
import { api } from "./api";

/** One delegated listener hands every `http(s)` link to the OS; webviews drop `target="_blank"`. */
export function useExternalLinks(): void {
  useEffect(() => {
    const onClick = (e: MouseEvent) => {
      if (e.defaultPrevented || e.button !== 0) return;
      const anchor = (e.target as Element | null)?.closest?.("a[href]");
      const href = anchor?.getAttribute("href") ?? "";
      if (!/^https?:\/\//i.test(href)) return;
      e.preventDefault();
      void api.openUrl(href);
    };
    window.addEventListener("click", onClick);
    return () => window.removeEventListener("click", onClick);
  }, []);
}
