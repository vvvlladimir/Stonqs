import { msg } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { Fragment, useEffect, useState } from "react";

import {
  BankIcon,
  BellIcon,
  CertificateIcon,
  ChartLineUpIcon,
  ChartPieSliceIcon,
  ArrowsLeftRightIcon,
  CalendarDotsIcon,
  CoinsIcon,
  DotsThreeCircleIcon,
  EyeIcon,
  FileArrowUpIcon,
  FileTextIcon,
  GearIcon,
  ListBulletsIcon,
  ReceiptIcon,
  ScalesIcon,
  SquaresFourIcon,
  WaveSineIcon,
  type Icon,
} from "@phosphor-icons/react";
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
import type { MessageDescriptor } from "@lingui/core";
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
import { SecurityCardProvider } from "./components/domain/SecurityCardProvider";
import { UpdateDialog } from "./components/domain/UpdateDialog";
import { ListRow, Modal, ToastProvider, TooltipLayer } from "./components/ui";

interface ScreenDef {
  id: string;
  title: MessageDescriptor;
  icon: Icon;
  group: "portfolio" | "analysis" | "data";
  /** Screens shown in the mobile bottom navigation. */
  primary?: boolean;
}

const SCREENS = [
  { id: "dashboard", title: msg`Overview`, icon: SquaresFourIcon, group: "portfolio", primary: true },

  { id: "positions", title: msg`Positions`, icon: ListBulletsIcon, group: "portfolio", primary: true },
  { id: "transactions", title: msg`Transactions`, icon: ReceiptIcon, group: "portfolio", primary: true },
  { id: "accounts", title: msg`Accounts`, icon: BankIcon, group: "portfolio" },
  { id: "securities", title: msg`Instruments`, icon: CertificateIcon, group: "portfolio" },
  { id: "watchlist", title: msg`Watchlist`, icon: EyeIcon, group: "portfolio" },
  { id: "plans", title: msg`Plans`, icon: CalendarDotsIcon, group: "portfolio" },
  { id: "alerts", title: msg`Alerts`, icon: BellIcon, group: "portfolio" },

  { id: "performance", title: msg`Performance`, icon: ChartLineUpIcon, group: "analysis" },
  { id: "trades", title: msg`Trades`, icon: ArrowsLeftRightIcon, group: "analysis" },
  { id: "risk", title: msg`Risk`, icon: WaveSineIcon, group: "analysis" },
  { id: "allocation", title: msg`Allocation`, icon: ChartPieSliceIcon, group: "analysis", primary: true },
  { id: "rebalance", title: msg`Rebalance`, icon: ScalesIcon, group: "analysis" },
  { id: "income", title: msg`Income`, icon: CoinsIcon, group: "analysis" },

  { id: "import", title: msg`Import`, icon: FileArrowUpIcon, group: "data" },
  { id: "reports", title: msg`Reports`, icon: FileTextIcon, group: "data" },
  { id: "settings", title: msg`Settings`, icon: GearIcon, group: "data" },
] as const satisfies readonly {
  id: ScreenId;
  title: MessageDescriptor;
  icon: Icon;
  group: string;
  primary?: boolean;
}[];

const NAV: readonly ScreenDef[] = SCREENS;

const GROUPS: { id: ScreenDef["group"]; label: MessageDescriptor }[] = [
  { id: "portfolio", label: msg`Portfolio` },
  { id: "analysis", label: msg`Analysis` },
  { id: "data", label: msg`Data` },
];

export function App() {
  const invalidate = useInvalidate();
  const [screen, setScreen] = useState<ScreenId>("dashboard");
  const [focus, setFocus] = useState<string | null>(null);
  const [moreOpen, setMoreOpen] = useState(false);
  const [aiOpen, setAiOpen] = useState(false);
  const status = useStatus();
  const profiles = useProfiles();
  // Asked once per window, before anything else: with one profile there is nobody to ask about.
  const [picked, setPicked] = useState(false);
  // A crossing nobody looked at yet marks the Alerts item, the way a tree with gaps marks its tab.
  const unseen = useAlertsUnseen();

  // Re-activates the catalog when the saved preference or the OS language changes.
  const { t, i18n } = useLingui();
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
    setMoreOpen(false);
  };

  const navButton = (item: ScreenDef, extra = "") => {
    const Icon = item.icon;
    return (
      <button
        key={item.id}
        type="button"
        className={`nav__item${item.id === screen ? " nav__item--active" : ""}${extra}`}
        onClick={() => go(item.id as ScreenId)}
      >
        <Icon weight={item.id === screen ? "fill" : "regular"} />
        {i18n._(item.title)}
        {item.id === "alerts" && (unseen.data ?? 0) > 0 && (
          <i className="tab-dot" aria-label={t`new crossings to look at`} />
        )}
      </button>
    );
  };

  return (
    <ToastProvider>
      <NavProvider value={{ screen, go }}>
        <AsOfProvider>
          <UpdatesProvider>
            <SecurityCardProvider>
              <div className="shell">
                <nav className="nav">
                  {navButton(NAV[0])}
                  {GROUPS.map((group) => (
                    <Fragment key={group.id}>
                      <div className="nav__group-label">{i18n._(group.label)}</div>
                      {NAV.filter((s) => s.group === group.id && s.id !== "dashboard").map((s) =>
                        navButton(s, s.primary ? "" : " nav__more"),
                      )}
                    </Fragment>
                  ))}
                  <button type="button" className="nav__item nav__mobile" onClick={() => setMoreOpen(true)}>
                    <DotsThreeCircleIcon />
                    <Trans>More</Trans>
                  </button>
                  <AsOfPicker variant="nav" />
                  <ScopePicker variant="nav" />
                </nav>

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

                {moreOpen && (
                  <Modal title={t`All screens`} onClose={() => setMoreOpen(false)}>
                    <div className="smenu">
                      {GROUPS.map((group) => (
                        <div key={group.id}>
                          <div className="group-label">{i18n._(group.label)}</div>
                          {NAV.filter((s) => s.group === group.id).map((item) => {
                            const Icon = item.icon;
                            return (
                              <ListRow
                                key={item.id}
                                lead={<Icon />}
                                title={i18n._(item.title)}
                                on={item.id === screen}
                                onClick={() => go(item.id as ScreenId)}
                              />
                            );
                          })}
                        </div>
                      ))}
                    </div>
                  </Modal>
                )}

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
