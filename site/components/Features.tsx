import {
  BellIcon,
  ChartPieSliceIcon,
  CoinsIcon,
  CurrencyCircleDollarIcon,
  FileCsvIcon,
  type Icon,
} from "@phosphor-icons/react";
import { motion } from "motion/react";
import type { ReactNode } from "react";
import { EASE, Reveal, Shot, Wrap } from "./ui";

function Cell({
  className = "",
  icon: I,
  title,
  children,
  visual,
  delay = 0,
}: {
  className?: string;
  icon?: Icon;
  title: string;
  children: ReactNode;
  visual?: ReactNode;
  delay?: number;
}) {
  return (
    <Reveal delay={delay} className={`flex flex-col overflow-hidden rounded-2xl border border-line ${className}`}>
      <div className="p-7 md:p-8">
        {I && <I size={26} weight="duotone" className="mb-5 text-accent" />}
        <h3 className="text-xl font-semibold tracking-tight">{title}</h3>
        <p className="mt-2 max-w-[46ch] leading-relaxed text-ink-2">{children}</p>
      </div>
      {visual}
    </Reveal>
  );
}

// The app's own palette slots, in the proportions of a plain three-fund portfolio.
const WEIGHTS = [
  { name: "Equities", share: 58, color: "var(--slot-1)" },
  { name: "Bonds", share: 22, color: "var(--slot-2)" },
  { name: "Real estate", share: 12, color: "var(--slot-3)" },
  { name: "Cash", share: 8, color: "var(--slot-4)" },
];

function Allocation() {
  return (
    <div className="mt-auto px-7 pb-8 md:px-8" role="img" aria-label="An example allocation by asset class">
      {/* The parent watches the viewport: a segment at scaleX 0 has no box for an observer to see. */}
      <motion.div
        className="flex h-2.5 gap-1 overflow-hidden rounded-full"
        initial="hidden"
        whileInView="shown"
        viewport={{ once: true, amount: 0.5 }}
        transition={{ staggerChildren: 0.12, delayChildren: 0.2 }}
      >
        {WEIGHTS.map((w) => (
          <motion.i
            key={w.name}
            className="block h-full origin-left rounded-full"
            style={{ flex: w.share, background: w.color }}
            variants={{ hidden: { scaleX: 0 }, shown: { scaleX: 1 } }}
            transition={{ duration: 0.9, ease: EASE }}
          />
        ))}
      </motion.div>
      <ul className="mt-5 grid grid-cols-2 gap-x-6 gap-y-2 text-sm">
        {WEIGHTS.map((w) => (
          <li key={w.name} className="flex items-center gap-2 text-ink-2">
            <b className="size-2 rounded-full" style={{ background: w.color }} />
            {w.name}
            <span className="ml-auto font-mono text-ink">{w.share}%</span>
          </li>
        ))}
      </ul>
      <p className="mt-4 text-xs text-ink-3">Example weights</p>
    </div>
  );
}

export function Features() {
  return (
    <section id="features" aria-labelledby="features-title" className="pb-24 md:pb-32">
      <Wrap>
        <Reveal>
          <h2
            id="features-title"
            className="max-w-[20ch] text-4xl font-semibold leading-[1.08] tracking-[-0.03em] md:text-5xl"
          >
            Everything a spreadsheet stopped doing well.
          </h2>
        </Reveal>

        <div className="mt-14 grid gap-4 md:grid-flow-dense md:grid-cols-2 lg:grid-cols-3">
          <Cell
            className="bg-surface-2 md:row-span-2"
            title="One ledger, every angle."
            visual={
              <div className="relative mt-auto h-[340px] overflow-hidden lg:h-auto lg:flex-1">
                <Shot
                  name="nav"
                  className="absolute left-8 top-0 w-[240px] max-w-none rounded-tl-2xl border-l border-t border-line bg-surface"
                  alt="The app's navigation: Overview, Positions, Transactions and Allocation as favourites, then the Portfolio section with Accounts, Instruments, Watchlist and Alerts, and the Analysis, Planning and Data sections."
                />
              </div>
            }
          >
            Positions, trades, risk, income and reports, all read from the same transactions you record once.
          </Cell>

          <Cell
            className="bg-accent-soft lg:col-span-2"
            icon={FileCsvIcon}
            title="Import without the busywork."
            delay={0.05}
          >
            Columns, dates and decimals are worked out for you, in a dozen languages. You see every row before
            it is saved, and importing the same file twice changes nothing.
          </Cell>

          <Cell
            className="bg-surface"
            icon={ChartPieSliceIcon}
            title="Allocation and rebalancing."
            visual={<Allocation />}
            delay={0.1}
          >
            Your own classification trees and target weights, with trades rounded to what you can buy.
          </Cell>

          <Cell className="bg-surface" icon={CoinsIcon} title="Income and costs." delay={0.15}>
            Dividends and interest by year, yield on cost, expected payments, and what fees and taxes took.
          </Cell>

          <Cell
            className="bg-surface md:col-span-2"
            icon={CurrencyCircleDollarIcon}
            title="Any currency, any account."
            visual={
              <div className="mt-auto border-t border-line bg-bg p-3 md:p-4">
                <Shot
                  name="kpis"
                  className="h-auto w-full"
                  alt="Four figures from the Overview: value 68,138.61 EUR, today +194.27 EUR, time-weighted return +53.37 %, earned over the period +17,795.20 EUR."
                />
              </div>
            }
          >
            Each trade keeps the exchange rate it was made at, and currency effects are shown on their own.
          </Cell>

          <Cell className="bg-surface" icon={BellIcon} title="Plans, goals and alerts." delay={0.05}>
            Savings plans that draft their own purchases, goals that track progress, and price levels that
            tell you when they are crossed.
          </Cell>
        </div>
      </Wrap>
    </section>
  );
}
