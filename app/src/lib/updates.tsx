import { createContext, useCallback, useContext, useEffect, useRef, useState, type ReactNode } from "react";

import { appVersion, checkForUpdate, installUpdate, restart, today, type AvailableUpdate } from "./api";
import { useUiState } from "./uiState";

/**
 * In-app updates. The plugin verifies the signature and replaces the bundle; everything the user
 * sees is decided here, so the check lives in one place and two parts of the app cannot offer the
 * same version at once.
 *
 * The check is throttled to once a calendar day and never runs while an answer is on screen. A
 * version the user skipped is dropped silently — asking again is how an update prompt becomes
 * something people learn to dismiss without reading.
 */

export type UpdateStage = "idle" | "checking" | "found" | "current" | "installing" | "ready" | "failed";

interface Updates {
  stage: UpdateStage;
  /** The running build's own version; null until the bundle has answered. */
  current: string | null;
  /** What is on offer, once a check found something. */
  update: AvailableUpdate | null;
  /** How much of the download has arrived; `null` while the size is unknown. */
  progress: number | null;
  /** Raw failure text from the plugin — English, a developer detail shown beside our own wording. */
  error: string | null;
  /** True while a check the user started is running, so only that button shows it working. */
  asked: boolean;
  check: () => void;
  install: () => void;
  relaunch: () => void;
  /** Not now: the same version is offered again at the next check. */
  later: () => void;
  /** Never this version again. */
  skip: () => void;
}

const UpdatesContext = createContext<Updates | null>(null);

export function UpdatesProvider({ children }: { children: ReactNode }) {
  const { ui, ready, save } = useUiState();
  const [stage, setStage] = useState<UpdateStage>("idle");
  const [update, setUpdate] = useState<AvailableUpdate | null>(null);
  const [progress, setProgress] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [asked, setAsked] = useState(false);
  const [current, setCurrent] = useState<string | null>(null);
  // The automatic check is a once-per-window event, not something a re-render may repeat.
  const startedRef = useRef(false);

  // Read once and shared: the version this build runs is what every answer here is measured
  // against, so two parts of the app must not ask the bundle separately and disagree.
  useEffect(() => {
    void appVersion().then(setCurrent);
  }, []);

  const run = useCallback(
    async (byHand: boolean) => {
      // The automatic check shows nothing while it runs: it starts during the first render, and
      // a state change there is a cascade for something the user never asked to watch.
      if (byHand) {
        setStage("checking");
        setAsked(true);
        setError(null);
      }
      try {
        const found = await checkForUpdate();
        // A version already declined is not news; by hand, "nothing new" is still an answer.
        if (found && (byHand || found.version !== ui.updates.skip)) {
          setUpdate(found);
          setStage("found");
        } else {
          setUpdate(null);
          // Silence is the right answer to a check nobody asked for; a pressed button needs one.
          setStage(byHand ? "current" : "idle");
        }
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
        // Being offline is not worth a modal; only a check the user asked for reports it.
        setStage(byHand ? "failed" : "idle");
      } finally {
        setAsked(false);
      }
    },
    [ui.updates.skip],
  );

  useEffect(() => {
    if (!ready || startedRef.current) return;
    if (!ui.updates.auto || ui.updates.checked === today()) return;
    startedRef.current = true;
    save({ ...ui, updates: { ...ui.updates, checked: today() } });
    // After the render that started it, not inside it: the check is a network call, and its
    // answer is a dialog, so nothing about it belongs to painting the app for the first time.
    void Promise.resolve().then(() => run(false));
  }, [ready, ui, save, run]);

  const install = useCallback(async () => {
    setStage("installing");
    setProgress(null);
    setError(null);
    try {
      // False means the handle went away — ask again rather than claim anything was installed.
      const installed = await installUpdate(setProgress);
      if (installed) setStage("ready");
      else {
        setUpdate(null);
        setStage("idle");
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setStage("failed");
    }
  }, []);

  const value: Updates = {
    stage,
    current,
    update,
    progress,
    error,
    asked,
    check: () => void run(true),
    install: () => void install(),
    relaunch: () => void restart(),
    later: () => {
      setUpdate(null);
      setStage("idle");
    },
    skip: () => {
      if (update) save({ ...ui, updates: { ...ui.updates, skip: update.version } });
      setUpdate(null);
      setStage("idle");
    },
  };

  return <UpdatesContext.Provider value={value}>{children}</UpdatesContext.Provider>;
}

/** Null outside the provider, which is how a build without the updater renders nothing. */
export function useUpdates(): Updates | null {
  return useContext(UpdatesContext);
}
