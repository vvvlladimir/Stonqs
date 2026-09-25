import { useEffect } from "react";
import { api } from "./api";

/**
 * Every link that leaves the app, opened by the operating system.
 *
 * A webview opens no window of its own on any platform this ships to: `target="_blank"` is
 * silently dropped, so an address written as a plain `<a>` does nothing at all when clicked. One
 * delegated listener answers for all of them — a link in a panel, a source's site, an address
 * inside an answer the assistant wrote — so no component has to remember, and a link added later
 * works by being a link.
 *
 * Only `http(s)` is handed over. An in-app anchor (`#`) and anything else a document may carry is
 * left to the browser it already is.
 */
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
