import React from "react";
import ReactDOM from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { i18n } from "@lingui/core";
import { I18nProvider } from "@lingui/react";
import { App } from "./App";
import { ShortcutsProvider } from "./lib/commands";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { applyTheme } from "./lib/theme";
import { activateLocale, resolveLocale } from "./lib/i18n";

// Fonts ship bundled, not from a CDN — the app works offline.
import "@fontsource-variable/geist";
import "@fontsource-variable/geist-mono";
import "./styles/index.css";

// All app state is a server cache over Tauri commands — no separate store,
// data lives in SQLite. refetchOnWindowFocus is off: the source is local.
const queryClient = new QueryClient({
  defaultOptions: { queries: { refetchOnWindowFocus: false, retry: false } },
});

// Must run before first paint — tokens.css variables are keyed off [data-theme].
applyTheme("system");

// The saved preference arrives with the settings query; until then the OS language is the
// best guess. Rendering waits for the catalog so no screen ever paints untranslated.
// `pnpm record:tour` sets up a demo profile first and reloads; nothing renders until it is open.
const ready = import.meta.env.VITE_RECORD_TOUR
  ? import("./lib/ipcTour").then((m) => m.prepareTour())
  : Promise.resolve(true);

void Promise.all([ready, activateLocale(resolveLocale("system", null))]).then(([go]) => {
  if (!go) return;
  ReactDOM.createRoot(document.getElementById("root")!).render(
    <React.StrictMode>
      <I18nProvider i18n={i18n}>
        <QueryClientProvider client={queryClient}>
          <ErrorBoundary>
            <ShortcutsProvider>
              <App />
            </ShortcutsProvider>
          </ErrorBoundary>
        </QueryClientProvider>
      </I18nProvider>
    </React.StrictMode>,
  );
});
