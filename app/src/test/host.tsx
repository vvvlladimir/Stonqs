/** A host that answers over IPC, the way the real one does. `mockIPC` intercepts at the door
 *  `lib/api` knocks on, so every layer above it — `call`, the query hooks, the invalidation —
 *  runs exactly as it does in the app. A command nobody wrote an answer for fails by name, so a
 *  screen that starts asking for something new says so instead of rendering an empty table. */
import type { ReactNode } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { i18n } from "@lingui/core";
import { I18nProvider } from "@lingui/react";
import { mockIPC } from "@tauri-apps/api/mocks";
import { render } from "@testing-library/react";
import { ShortcutsProvider } from "../lib/commands";
import { DockProvider } from "../lib/dock";
import { NavProvider } from "../lib/nav";
import { AsOfProvider } from "../lib/asOf";
import { ToastProvider } from "../components/ui";

export type Answers = Record<string, (args: Record<string, unknown>) => unknown>;

/** Every command the host is asked for during the test, in order — what a screen actually did. */
export type Asked = Array<{ command: string; args: Record<string, unknown> }>;

export function mockHost(answers: Answers): Asked {
  const asked: Asked = [];
  mockIPC((command, args) => {
    const payload = (args ?? {}) as Record<string, unknown>;
    asked.push({ command, args: payload });
    const answer = answers[command];
    if (!answer) {
      throw {
        code: "internal",
        message: `the test host was asked for ${command}, which it has no answer for`,
      };
    }
    return answer(payload);
  });
  return asked;
}

/** The providers a screen is mounted under, and no more of the shell than that. */
export function renderScreen(ui: ReactNode) {
  // The macros carry their English source as the default message, so an empty catalogue renders
  // the interface in English without a compile step in the test.
  i18n.load("en", {});
  i18n.activate("en");
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false, gcTime: 0 }, mutations: { retry: false } },
  });
  return render(
    <I18nProvider i18n={i18n}>
      <QueryClientProvider client={client}>
        <ToastProvider>
          <NavProvider value={{ screen: "import", go: () => {} }}>
            <AsOfProvider>
              <ShortcutsProvider>
                <DockProvider>{ui}</DockProvider>
              </ShortcutsProvider>
            </AsOfProvider>
          </NavProvider>
        </ToastProvider>
      </QueryClientProvider>
    </I18nProvider>,
  );
}
