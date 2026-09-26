import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { Trans, useLingui } from "@lingui/react/macro";
import { useTour } from "../../../lib/tour";
import { useLayer } from "../../../lib/commands";
import { Modal } from "../../ui";
import { SourcesSetup } from "../sources";
import { useAnchor } from "./useAnchor";

/** Distance the card keeps from its anchor and from the window's edges. */
const GAP = 12;
const EDGE = 12;
/** Below this the card is a sheet at the foot of the window and is not placed at all. */
const NARROW = 700;

/**
 * Draws the tour: the offer, the card of the current step, and the hole cut around what it
 * points at. Everything under it is inert — a tour that can be clicked through is a tour that
 * ends on a screen nobody meant to open.
 */
export function TourLayer() {
  const tour = useTour();
  if (!tour) return null;
  return (
    <>
      {tour.offered && <TourOffer />}
      {tour.step && <TourCard key={tour.step.id} />}
      {tour.chooseSources && <SourcesSetup onClose={tour.closeSources} />}
    </>
  );
}

/** Asked once per profile. Both answers are answers: neither is asked again by itself. */
function TourOffer() {
  const tour = useTour();
  if (!tour) return null;
  return (
    <Modal
      title={<Trans>New here?</Trans>}
      onClose={tour.decline}
      foot={
        <>
          <button type="button" className="btn btn--quiet" onClick={tour.decline}>
            <Trans>Not now</Trans>
          </button>
          <span className="spacer" />
          <button type="button" className="btn" onClick={tour.start}>
            <Trans>Show me around</Trans>
          </button>
        </>
      }
    >
      <p>
        <Trans>
          A short tour of what each screen is for. It changes nothing and can be left at any time; you can
          start it again from Settings.
        </Trans>
      </p>
    </Modal>
  );
}

function TourCard() {
  const { i18n } = useLingui();
  const tour = useTour();
  const card = useRef<HTMLDivElement>(null);
  const [placed, setPlaced] = useState<{ left: number; top: number } | null>(null);
  const step = tour?.step ?? null;
  const anchor = useAnchor(step?.anchor, step?.id ?? "");
  const narrow = useNarrow();

  // Escape leaves the tour, and the keyboard belongs to it while it runs.
  useLayer(() => tour?.stop());

  // A step whose element this screen does not draw is not shown at all.
  const missing = anchor.missing;
  useEffect(() => {
    if (missing) tour?.next();
    // `tour` is a fresh object every render; the skip belongs to this step alone.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [missing, step?.id]);

  useLayoutEffect(() => {
    const el = card.current;
    if (!el || !anchor.rect || narrow) {
      setPlaced(null);
      return;
    }
    const { width, height } = el.getBoundingClientRect();
    const r = anchor.rect;
    const below = r.top - GAP - height < EDGE;
    const top = below
      ? Math.min(r.bottom + GAP, window.innerHeight - EDGE - height)
      : Math.max(r.top - GAP - height, EDGE);
    const centred = r.left + r.width / 2 - width / 2;
    setPlaced({
      left: Math.min(Math.max(centred, EDGE), Math.max(EDGE, window.innerWidth - EDGE - width)),
      top,
    });
  }, [anchor.rect, narrow, step?.id]);

  if (!tour || !step) return null;
  const last = tour.index + 1 === tour.count;

  return createPortal(
    <div className="tour" role="dialog" aria-modal="true" aria-label={i18n._(step.title)}>
      <div className="tour__mask" style={holeOf(anchor.rect)} />
      <div
        ref={card}
        className={`tour__card${anchor.rect && !narrow ? "" : " tour__card--centred"}`}
        style={placed ? { left: placed.left, top: placed.top } : undefined}
      >
        <div>
          <h2>{i18n._(step.title)}</h2>
          <p>{i18n._(step.body)}</p>
        </div>
        <div className="tour__foot">
          <span className="tour__dots" aria-hidden>
            {Array.from({ length: tour.count }, (_, i) => (
              <i key={i} className={i === tour.index ? "on" : undefined} />
            ))}
          </span>
          <span className="tour__acts">
            {/* Leaving is the quietest thing here, going back the second: one primary button,
                so the way on is never one of three equal boxes. */}
            <button type="button" className="btn btn--sm btn--quiet" onClick={tour.stop}>
              <Trans>Skip</Trans>
            </button>
            {tour.index > 0 && (
              <button type="button" className="btn btn--sm btn--ghost" onClick={tour.back}>
                <Trans>Back</Trans>
              </button>
            )}
            <button type="button" className="btn btn--sm" autoFocus onClick={tour.next}>
              {last ? <Trans>Finish</Trans> : <Trans>Next</Trans>}
            </button>
          </span>
        </div>
      </div>
    </div>,
    document.body,
  );
}

/**
 * The hole is the mask's own shadow rather than a second element: one box, so nothing can drift
 * out of step with what it is cut around.
 */
function holeOf(rect: DOMRect | null) {
  if (!rect) return undefined;
  return {
    left: rect.left - 4,
    top: rect.top - 4,
    width: rect.width + 8,
    height: rect.height + 8,
    opacity: 1,
  };
}

/** The card is a sheet on a narrow window: there is no room beside anything. */
function useNarrow(): boolean {
  const [narrow, setNarrow] = useState(() => window.innerWidth < NARROW);
  useEffect(() => {
    const onResize = () => setNarrow(window.innerWidth < NARROW);
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);
  return narrow;
}
