/**
 * Dev-only tour for the website's screenshots, started by `pnpm record:tour`
 * (scripts/record-tour.mjs). It records from a fresh demo profile rather than whatever the dev
 * profile holds, so every recording shows the same portfolio:
 *
 *   fresh -> delete the previous "Screenshots" profile, create a new one and open it
 *   seed  -> fill it with the demo portfolio and wait for the refresh it starts
 *   visit -> open every screen in turn, recording what each asks (src/lib/ipcRecord.ts)
 *
 * Each stage ends in a reload, as switching profile does in the app, so the stage is kept in
 * sessionStorage. Absent from a release build: imported only behind `VITE_RECORD_TOUR`.
 *
 * The stages live here and the component that walks them in `ipcTourRecorder.tsx`: a module
 * exporting both a component and the functions beside it loses fast refresh for the whole app.
 */
import type { QueryClient } from "@tanstack/react-query";
import { api } from "./api";
import type { ScreenId } from "./nav";
import { markChosen } from "./profiles";

const STAGE = "vs.tour.stage";
const PROFILE = "Screenshots";

/** Every screen the website may shoot, in the order the tour opens them; the dashboard last,
 *  so the fixture's final answers for the shell are the ones it shows on arrival. */
export const SCREENS: ScreenId[] = [
  "positions",
  "transactions",
  "accounts",
  "securities",
  "watchlist",
  "plans",
  "alerts",
  "performance",
  "trades",
  "risk",
  "allocation",
  "rebalance",
  "income",
  "import",
  "reports",
  "dashboard",
];

type Stage = "fresh" | "seed" | "visit" | "done";

export function tourStage(): Stage {
  return (sessionStorage.getItem(STAGE) as Stage | null) ?? "fresh";
}

/** The recorder's last act: the walk is over and the fixture is complete. */
export function tourFinished() {
  sessionStorage.setItem(STAGE, "done");
}

function next(stage: Stage) {
  sessionStorage.setItem(STAGE, stage);
  markChosen();
  window.location.reload();
}

const pause = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** Runs the stages that happen before the app renders. False while the window is about to reload. */
export async function prepareTour(): Promise<boolean> {
  const stage = tourStage();
  if (stage === "fresh") {
    const list = await api.profilesList();
    const open = list.profiles.find((p) => p.id === list.open);
    const old = list.profiles.find((p) => p.name === PROFILE);
    if (open?.name === PROFILE) {
      await api.profileDelete(null);
      next("fresh");
    } else if (old) {
      await api.profileOpen(old.id);
      next("fresh");
    } else {
      const created = await api.profileCreate(PROFILE);
      await api.profileOpen(created.id);
      next("seed");
    }
    return false;
  }
  if (stage === "seed") {
    await api.demoSeed();
    // Seeding queues a refresh; the tour must not record figures that are about to change.
    await pause(1000);
    while ((await api.refreshStatus()).running) await pause(500);
    next("visit");
    return false;
  }
  markChosen();
  return true;
}

/** Nothing in flight for `ms` in a row: queries arrive in waves, a chart after its data. */
export async function quiet(client: QueryClient, ms = 1500) {
  let since = Date.now();
  while (Date.now() - since < ms) {
    await pause(100);
    if (client.isFetching() + client.isMutating() > 0) since = Date.now();
  }
}
