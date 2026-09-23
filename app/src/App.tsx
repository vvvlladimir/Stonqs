import { useLingui } from "@lingui/react";
import { Trans } from "@lingui/react/macro";
import { useEffect, useState } from "react";

import { onDataChanged } from "./lib/api";
import { affects, useAlertsUnseen, useInvalidate, useProfiles, useStatus } from "./lib/queries";
import { needsPick } from "./lib/profiles";
import { ProfilePicker } from "./components/domain/ProfilePicker";
import { ProfileLock } from "./components/domain/ProfileLock";
import { useLanguage } from "./lib/i18n";
import { useTheme } from "./lib/theme";
import { useUiState } from "./lib/uiState";
import { UpdatesProvider } from "./lib/updates";
import { AsOfProvider } from "./lib/asOf";
import { NavProvider, type ScreenId } from "./lib/nav";
import { noteChange } from "./lib/freshness";
import { Dashboard } from "./screens/dashboard/Dashboard";
import { Onboarding } from "./screens/Onboarding";
import { Accounts } from "./screens/Accounts";
import { Securities } from "./screens/Securities";
import { Settings } from "./screens/Settings";
import { Positions } from "./screens/Positions";
import { Transactions } from "./screens/Transactions";
import { Performance } from "./screens/Performance";
import { Trades } from "./screens/Trades";
import { Risk } from "./screens/Risk";
import { Income } from "./screens/Income";
import { Allocation } from "./screens/Allocation";
import { Rebalance } from "./screens/Rebalance";
import { Plans } from "./screens/Plans";
import { Watchlist } from "./screens/Watchlist";
import { Alerts } from "./screens/Alerts";
import { AiChatPanel, AiToggle } from "./components/domain/AiChatPanel";
import { AlertNotifier } from "./components/domain/AlertNotifier";
import { Reports } from "./screens/Reports";
import { Import } from "./screens/Import";
import { SyncChip } from "./components/domain/MarketRefresh";
import { ScopePicker } from "./components/domain/ScopePicker";
import { AsOfBanner, AsOfPicker } from "./components/domain/AsOfPicker";
import { Nav } from "./components/Nav";
import { SecurityCardProvider } from "./components/domain/SecurityCardProvider";
import { UpdateDialog } from "./components/domain/UpdateDialog";
import { ToastProvider, TooltipLayer } from "./components/ui";

export function App() {
  const invalidate = useInvalidate();
  const [screen, setScreen] = useState<ScreenId>("dashboard");
  const [focus, setFocus] = useState<string | null>(null);
  const [aiOpen, setAiOpen] = useState(false);
  const status = useStatus();
  const profiles = useProfiles();
  // Asked once per window, before anything else: with one profile there is nobody to ask about.
  const [picked, setPicked] = useState(false);
  // A crossing nobody looked at yet marks the Alerts item, the way a tree with gaps marks its tab.
  const unseen = useAlertsUnseen();

  // Re-renders the shell when the saved preference or the OS language changes the catalog.
  useLingui();
  useLanguage();
  useTheme(useUiState().ui.theme);

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
              <div className="shell">
                <Nav screen={screen} go={go} alertsDot={(unseen.data ?? 0) > 0} />

                <div className="main">
                  <AsOfBanner />
                  <main className="app">
                    {screen === "dashboard" && <Dashboard />}
                    {screen === "positions" && <Positions />}
                    {screen === "transactions" && <Transactions focus={focus} />}
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
                    {screen === "securities" && <Securities focus={focus} />}
                    {screen === "settings" && <Settings status={status.data} />}
                  </main>
                </div>

                <div className="dock">
                  {/* Screens portal dock controls here without changing page layout. */}
                  <div id="dock-slot" className="dock__slot" />
                  <AsOfPicker variant="dock" />
                  <ScopePicker variant="dock" />
                  <SyncChip />
                  <AiToggle open={aiOpen} onToggle={() => setAiOpen((v) => !v)} />
                </div>

                {aiOpen && <AiChatPanel onClose={() => setAiOpen(false)} />}

                <TooltipLayer />
                <AlertNotifier />
                <UpdateDialog />
              </div>
            </SecurityCardProvider>
          </UpdatesProvider>
        </AsOfProvider>
      </NavProvider>
    </ToastProvider>
  );
}
