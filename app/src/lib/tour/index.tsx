import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from "react";
import { useNav } from "../nav";
import { useSettings } from "../queries";
import { useUiState } from "../uiState";
import { STEPS, type TourStep } from "./steps";

interface Tour {
  /** The step being shown, or `null` while the tour is not running. */
  step: TourStep | null;
  /** Which stop this is, for the progress dots. */
  index: number;
  count: number;
  /** Shown to somebody who has not answered the offer yet, once the sources question is out
   *  of the way — a tour offered over that dialog is a second question about a first one. */
  offered: boolean;
  start: () => void;
  next: () => void;
  back: () => void;
  /** Ends the tour; the offer is answered either way, and never made again by itself. */
  stop: () => void;
  /** Turns the offer down. The sources question has already been asked by then. */
  decline: () => void;
  /** The first thing a new profile is shown, while the sources are unchosen: the one question
   *  nobody else can answer, ahead of the offer to walk through screens it decides the content
   *  of. Answered or left, it is not asked again by itself. */
  chooseSources: boolean;
  closeSources: () => void;
}

const TourContext = createContext<Tour | null>(null);

/**
 * The guided tour's state: which stop is showing, and what happens at the end of it.
 *
 * It holds no DOM and draws nothing — `components/domain/tour` does, the way the updater's
 * dialog is separate from `lib/updates`. The tour writes no portfolio data and changes no
 * setting but the one that records it was offered (ADR-0077).
 */
export function TourProvider({ children }: { children: ReactNode }) {
  const nav = useNav();
  const { ui, ready, save } = useUiState();
  const settings = useSettings();
  const [index, setIndex] = useState<number | null>(null);
  // Set when the sources dialog is closed, however it was closed: the question is asked once,
  // and the offer behind it must not wait for an answer the user declined to give.
  const [sourcesAsked, setSourcesAsked] = useState(false);

  const answer = useCallback(() => {
    if (!ui.tour.done) save((current) => ({ ...current, tour: { done: true } }));
  }, [save, ui]);

  // A profile that has never answered the offer, with the app on screen and nothing running:
  // the one state in which anything here is shown without being asked for.
  const firstRun = ready && !ui.tour.done && index === null;
  const unchosen = !sourcesAsked && !(settings.data?.sources_configured ?? true);

  const value = useMemo<Tour>(() => {
    const show = (next: number) => {
      const step = STEPS[next];
      if (!step) return;
      setIndex(next);
      nav.go(step.screen);
    };
    return {
      step: index === null ? null : (STEPS[index] ?? null),
      index: index ?? 0,
      count: STEPS.length,
      // Offered once the app is on screen with settings loaded, and once the sources question
      // has been put: an offer drawn over a shell that is still opening has nothing to point
      // at yet, and one drawn over that dialog asks two things at once.
      offered: firstRun && !unchosen,
      start: () => {
        answer();
        show(0);
      },
      next: () => {
        if (index === null) return;
        if (index + 1 >= STEPS.length) {
          setIndex(null);
          answer();
          return;
        }
        show(index + 1);
      },
      back: () => index !== null && index > 0 && show(index - 1),
      stop: () => {
        setIndex(null);
        answer();
      },
      decline: answer,
      chooseSources: firstRun && unchosen,
      // Answered or left, the offer behind it is what comes next: declining the question is
      // still an answer to it, and the stops do not depend on what was chosen.
      closeSources: () => setSourcesAsked(true),
    };
  }, [answer, firstRun, index, nav, unchosen]);

  return <TourContext.Provider value={value}>{children}</TourContext.Provider>;
}

/** Null outside the shell — the onboarding gate and the profile picker have no tour. */
export function useTour(): Tour | null {
  return useContext(TourContext);
}
