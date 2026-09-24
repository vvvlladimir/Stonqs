import {
  CalendarDotsIcon,
  ChartLineUpIcon,
  ChartPieSliceIcon,
  CoinsIcon,
  ScalesIcon,
  type Icon,
} from "@phosphor-icons/react";
import { AnimatePresence, motion } from "motion/react";
import { useId, useState, type KeyboardEvent } from "react";
import { EASE, Reveal, Shot, Wrap } from "./ui";
import shots from "./shots.json";

// One screen per tab, each a real panel of the demo portfolio (scripts/screenshots.mjs).
const STOPS: { key: keyof typeof shots; label: string; icon: Icon; text: string; alt: string }[] = [
  {
    key: "performance",
    label: "Returns",
    icon: ChartLineUpIcon,
    text: "Every month's time-weighted return and each year's total, so a good year and a lucky month are told apart.",
    alt: "Return by month: a grid of monthly returns from September 2023 to September 2026, coloured green and red, with a total for each year.",
  },
  {
    key: "allocation",
    label: "Allocation",
    icon: ChartPieSliceIcon,
    text: "Your own classification trees, with cash counted like any holding and splits across categories.",
    alt: "Allocation by asset class as a map: equities 74.5 %, bonds 12.0 %, cash 10.4 %, with targets of 70 % and 20 % beside equities and bonds.",
  },
  {
    key: "rebalance",
    label: "Rebalance",
    icon: ScalesIcon,
    text: "How far each part is from its target, and the trades that close the gap, rounded to what you can buy.",
    alt: "Deviation from target: cash 10.4 % of 5 %, bonds 12.0 % of 20 %, crypto 3.0 % of 5 %, equities 74.5 % of 70 %, with the amount to buy or sell for each.",
  },
  {
    key: "income",
    label: "Income",
    icon: CoinsIcon,
    text: "Dividends and interest month by month, with what was withheld at the source kept apart.",
    alt: "When the money arrives: a grid of income received per month from October 2023 to September 2026, darker where more arrived.",
  },
  {
    key: "goals",
    label: "Goals",
    icon: CalendarDotsIcon,
    text: "Goals that track their own accounts and say whether the current pace gets there in time.",
    alt: "Two goals: an apartment deposit at 56.78 % of 120,000 EUR, marked behind, needing 702.83 EUR a month; an emergency fund at 13.30 % of 15,000 EUR.",
  },
];

// The tallest panel sets the frame, so switching tabs does not move the page.
const RATIO = Math.min(...STOPS.map((s) => shots[s.key].width / shots[s.key].height));

export function Tour() {
  const [at, setAt] = useState(0);
  const base = useId();
  const stop = STOPS[at];

  // Arrow keys move along the strip, as a tablist is expected to (WAI-ARIA tabs pattern).
  function onKey(e: KeyboardEvent) {
    const step = e.key === "ArrowRight" ? 1 : e.key === "ArrowLeft" ? -1 : 0;
    if (!step) return;
    e.preventDefault();
    const next = (at + step + STOPS.length) % STOPS.length;
    setAt(next);
    document.getElementById(`${base}-tab-${next}`)?.focus();
  }

  return (
    <section aria-labelledby="tour-title" className="pb-24 md:pb-32">
      <Wrap>
        <Reveal className="max-w-[640px]">
          <h2
            id="tour-title"
            className="text-4xl font-semibold leading-[1.08] tracking-[-0.03em] md:text-5xl"
          >
            The detail behind every figure.
          </h2>
        </Reveal>

        <Reveal delay={0.05} className="mt-10">
          <div
            role="tablist"
            aria-label="Screens"
            onKeyDown={onKey}
            className="-mx-5 flex gap-2 overflow-x-auto px-5 pb-1 md:mx-0 md:px-0"
          >
            {STOPS.map((s, i) => (
              <button
                key={s.key}
                id={`${base}-tab-${i}`}
                role="tab"
                type="button"
                aria-selected={i === at}
                aria-controls={`${base}-panel`}
                tabIndex={i === at ? 0 : -1}
                onClick={() => setAt(i)}
                className={`inline-flex shrink-0 items-center gap-2 rounded-full border px-4 py-2 text-sm font-medium transition-colors ${
                  i === at
                    ? "border-ink bg-ink text-bg"
                    : "border-line-strong bg-surface text-ink-2 hover:text-ink"
                }`}
              >
                <s.icon size={16} weight={i === at ? "fill" : "regular"} />
                {s.label}
              </button>
            ))}
          </div>
        </Reveal>

        <Reveal delay={0.1} className="mt-6">
          <div
            id={`${base}-panel`}
            role="tabpanel"
            aria-labelledby={`${base}-tab-${at}`}
            tabIndex={0}
            className="shadow-frame overflow-x-auto rounded-2xl border border-line bg-bg"
          >
            {/* Below ~960px a panel would be too small to read, so the frame scrolls instead. */}
            <div className="min-w-[960px] p-3 md:p-5">
              <div className="relative" style={{ aspectRatio: RATIO }}>
                <AnimatePresence initial={false} mode="popLayout">
                  <motion.div
                    key={stop.key}
                    className="absolute inset-0 flex items-center"
                    initial={{ opacity: 0, y: 12 }}
                    animate={{ opacity: 1, y: 0 }}
                    exit={{ opacity: 0 }}
                    transition={{ duration: 0.5, ease: EASE }}
                  >
                    <Shot name={stop.key} alt={stop.alt} className="h-auto w-full" />
                  </motion.div>
                </AnimatePresence>
              </div>
            </div>
          </div>
          <p aria-live="polite" className="mt-5 max-w-[60ch] leading-relaxed text-ink-2">
            {stop.text}
          </p>
        </Reveal>
      </Wrap>
    </section>
  );
}
