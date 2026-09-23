import { useLingui } from "@lingui/react";
import { Trans } from "@lingui/react/macro";
import { Suspense, lazy, useEffect, useState } from "react";

import { onDataChanged } from "./lib/api";
import {
  affects,
  useAlertsUnseen,
  useInvalidate,
  usePluginTheme,
  usePlugins,
  useProfiles,
  useStatus,
} from "./lib/queries";
import { needsPick } from "./lib/profiles";
import { ProfilePicker } from "./components/domain/ProfilePicker";
import { ProfileLock } from "./components/domain/ProfileLock";
import { useLanguage } from "./lib/i18n";
import { pluginTheme, useTheme } from "./lib/theme";
import { useUiState } from "./lib/uiState";
import { UpdatesProvider } from "./lib/updates";
import { AsOfProvider } from "./lib/asOf";
import { DockProvider, DockSlot } from "./lib/dock";
import { NavProvider, type ScreenId } from "./lib/nav";
import { noteChange } from "./lib/freshness";
import { Onboarding } from "./screens/Onboarding";
import { AiToggle } from "./components/domain/AiToggle";
import { AlertNotifier } from "./components/domain/AlertNotifier";
import { SyncChip } from "./components/domain/MarketRefresh";
import { ScopePicker } from "./components/domain/ScopePicker";
import { AsOfBanner, AsOfPicker } from "./components/domain/AsOfPicker";
import { Nav } from "./components/Nav";
import { SecurityCardProvider } from "./components/domain/SecurityCardProvider";
import { UpdateDialog } from "./components/domain/UpdateDialog";
import { Pending, ToastProvider, TooltipLayer } from "./components/ui";

// One screen is one chunk: the shell is what has to be on screen first, and nobody opens
// seventeen screens in a session. `Onboarding` stays eager — it is what an empty database shows
// before any screen exists — and so does the dock, which draws while a screen is still loading.
const Dashboard = lazy(() => import("./screens/dashboard/Dashboard").then((m) => ({ default: m.Dashboard })));
const Accounts = lazy(() => import("./screens/Accounts").then((m) => ({ default: m.Accounts })));
const Securities = lazy(() => import("./screens/Securities").then((m) => ({ default: m.Securities })));
const Settings = lazy(() => import("./screens/Settings").then((m) => ({ default: m.Settings })));
const Positions = lazy(() => import("./screens/Positions").then((m) => ({ default: m.Positions })));
const Transactions = lazy(() => import("./screens/Transactions").then((m) => ({ default: m.Transactions })));
const Performance = lazy(() => import("./screens/Performance").then((m) => ({ default: m.Performance })));
const Trades = lazy(() => import("./screens/Trades").then((m) => ({ default: m.Trades })));
const Risk = lazy(() => import("./screens/Risk").then((m) => ({ default: m.Risk })));
const Income = lazy(() => import("./screens/Income").then((m) => ({ default: m.Income })));
const Allocation = lazy(() => import("./screens/Allocation").then((m) => ({ default: m.Allocation })));
const Rebalance = lazy(() => import("./screens/Rebalance").then((m) => ({ default: m.Rebalance })));
const Plans = lazy(() => import("./screens/Plans").then((m) => ({ default: m.Plans })));
const Watchlist = lazy(() => import("./screens/Watchlist").then((m) => ({ default: m.Watchlist })));
const Alerts = lazy(() => import("./screens/Alerts").then((m) => ({ default: m.Alerts })));
const Reports = lazy(() => import("./screens/Reports").then((m) => ({ default: m.Reports })));
const Import = lazy(() => import("./screens/Import").then((m) => ({ default: m.Import })));

// The panel carries the markdown renderer, and it is mounted only once it is opened.
const AiChatPanel = lazy(() =>
  import("./components/domain/AiChatPanel").then((m) => ({ default: m.AiChatPanel })),
);

export function App() {
  const invalidate = useInvalidate();
  const [screen, setScreen] = useState<ScreenId>("dashboard");
  const [focus, setFocus] = useState<string | null>(null);
  const [aiOpen, setAiOpen] = useState(false);
  const status = useStatus();
  const profiles = useProfiles();
  const plugins = usePlugins();
  // Asked once per window, before anything else: with one profile there is nobody to ask about.
  const [picked, setPicked] = useState(false);
  // A crossing nobody looked at yet marks the Alerts item, the way a tree with gaps marks its tab.
  const unseen = useAlertsUnseen();

  // Re-renders the shell when the saved preference or the OS language changes the catalog.
  useLingui();
  useLanguage();
  // A theme installed as a plugin is a stylesheet on this machine plus the built-in scheme it
  // varies; both arrive a moment after the shell, which is why `useTheme` takes them separately.
  const theme = useUiState().ui.theme;
  const themeKey = pluginTheme(theme);
  const installed = usePluginTheme(themeKey);
  const base = plugins.data?.themes.find((t) => t.key === themeKey)?.base;
  useTheme(theme, installed.data, base);

  // A host-side write refreshes the screens that depend on what it touched.
  useEffect(() => {
    // An unknown kind from a newer host must not throw; the screens stay as they are.
    const unlisten = onDataChanged((kind) => {
      invalidate(...(affects[kind] ?? []));
      // A generated brief cannot be invalidated — regenerating it spends the user's money — so
      // the change is recorded and the tile says it is out of date instead.
      noteChange(kind);
    });
    return () => {
      unlisten.then((stop) => stop());
    };
  }, [invalidate]);

  // A locked profile shows nothing but its lock; every other query answers `locked` meanwhile.
  if (profiles.data?.locked)
    return (
      <ToastProvider>
        <div className="shell">
          <main className="app">
            <ProfileLock profiles={profiles.data} />
          </main>
        </div>
      </ToastProvider>
    );

  if (profiles.data && !picked && needsPick(profiles.data.profiles.length))
    return (
      <ToastProvider>
        <div className="shell">
          <main className="app">
            <ProfilePicker profiles={profiles.data} onPicked={() => setPicked(true)} />
          </main>
        </div>
      </ToastProvider>
    );

  if (status.isPending)
    return (
      <div className="app">
        <Trans>Opening the database…</Trans>
      </div>
    );
  if (status.isError) return <div className="app err">{status.error.message}</div>;

  if (status.data.account_count === 0) {
    return (
      <ToastProvider>
        <div className="shell">
          <main className="app">
            <Onboarding status={status.data} />
          </main>
        </div>
      </ToastProvider>
    );
  }

  // A screen reached with a navigation hint is keyed by it: arriving at one already open, with a
  // different instrument to look at, starts the screen over instead of leaving it to notice.
  const go = (id: ScreenId, withFocus?: string) => {
    setScreen(id);
    setFocus(withFocus ?? null);
  };

  return (
    <ToastProvider>
      <NavProvider value={{ screen, go }}>
        <AsOfProvider>
          <UpdatesProvider>
            <SecurityCardProvider>
              <DockProvider>
                <div className="shell">
                  <Nav screen={screen} go={go} alertsDot={(unseen.data ?? 0) > 0} />

                  <div className="main">
                    <AsOfBanner />
                    <main className="app">
                      <Suspense fallback={<Pending />}>
                        {screen === "dashboard" && <Dashboard />}
                        {screen === "positions" && <Positions />}
                        {screen === "transactions" && <Transactions key={focus} focus={focus} />}
                        {screen === "performance" && <Performance />}
                        {screen === "trades" && <Trades />}
                        {screen === "risk" && <Risk />}
                        {screen === "income" && <Income />}
                        {screen === "allocation" && <Allocation />}
                        {screen === "rebalance" && <Rebalance />}
                        {screen === "plans" && <Plans />}
                        {screen === "alerts" && <Alerts />}
                        {screen === "watchlist" && <Watchlist />}
                        {screen === "reports" && <Reports />}
                        {screen === "import" && <Import />}
                        {screen === "accounts" && <Accounts />}
                        {screen === "securities" && <Securities key={focus} focus={focus} />}
                        {screen === "settings" && <Settings status={status.data} />}
                      </Suspense>
                    </main>
                  </div>

                  <div className="dock">
                    {/* Screens portal dock controls here without changing page layout. */}
                    <DockSlot />
                    <AsOfPicker variant="dock" />
                    <ScopePicker variant="dock" />
                    <SyncChip />
                    <AiToggle open={aiOpen} onToggle={() => setAiOpen((v) => !v)} />
                  </div>

                  {aiOpen && (
                    <Suspense fallback={null}>
                      <AiChatPanel onClose={() => setAiOpen(false)} />
                    </Suspense>
                  )}

                  <TooltipLayer />
                  <AlertNotifier />
                  <UpdateDialog />
                </div>
              </DockProvider>
            </SecurityCardProvider>
          </UpdatesProvider>
        </AsOfProvider>
      </NavProvider>
    </ToastProvider>
  );
}
