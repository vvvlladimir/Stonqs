import { useEffect } from "react";
import { useLingui } from "@lingui/react/macro";
import { alertNotification } from "../../lib/alerts";
import { api, notify, onDataChanged, onMarketProgress } from "../../lib/api";

/** OS notifications for new crossings: on start, after each refresh and on `alerts` changes. Renders nothing. */
export function AlertNotifier() {
  const { i18n } = useLingui();

  useEffect(() => {
    const announce = async () => {
      try {
        for (const row of await api.alertsTakeNotifications()) {
          const { title, body } = alertNotification(i18n, row);
          await notify(title, body);
        }
      } catch {
        // A refused permission or a failed read must not break the app; the log still shows it.
      }
    };
    void announce();
    const stops = [
      onMarketProgress((progress) => {
        if (progress.event === "finished") void announce();
      }),
      onDataChanged((kind) => {
        if (kind === "alerts") void announce();
      }),
    ];
    return () => {
      for (const stop of stops) stop.then((unlisten) => unlisten());
    };
  }, [i18n]);

  return null;
}
