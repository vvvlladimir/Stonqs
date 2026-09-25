/* eslint-disable lingui/no-unlocalized-strings -- dev-only recorder: log lines */
/**
 * The component half of the screenshot tour (`lib/ipcTour.ts`): once the demo profile is open
 * and seeded, it walks every screen and waits for each to go quiet, so `src/lib/ipcRecord.ts`
 * records one complete set of answers per screen.
 */
import { useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import { SCREENS, quiet, tourFinished, tourStage } from "./ipcTour";
import type { ScreenId } from "./nav";

/** Walks the screens once the demo profile is open. Rendered by `App` in tour mode only. */
export function RecordTour({ go }: { go: (screen: ScreenId) => void }) {
  const client = useQueryClient();
  useEffect(() => {
    if (tourStage() !== "visit") return;
    let cancelled = false;
    void (async () => {
      await quiet(client);
      for (const screen of SCREENS) {
        if (cancelled) return;
        go(screen);
        await quiet(client);
        console.info(`tour: recorded ${screen}`);
      }
      tourFinished();
      await fetch("/__ipc-record/done", { method: "POST" });
      document.title = "Recording finished";
    })();
    return () => {
      cancelled = true;
    };
    // `go` is a fresh closure every render; the tour runs once.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  return null;
}
