import { Reveal, Shot, Wrap } from "./ui";

// Distinct brokers behind core/presets/brokers.json (29 layouts, some brokers have two).
const BROKERS = [
  "Trade Republic",
  "Interactive Brokers",
  "DEGIRO",
  "Trading 212",
  "Saxo Bank",
  "Swissquote",
  "Revolut",
  "Charles Schwab",
  "eToro",
  "XTB",
  "Avanza",
  "Freetrade",
  "InvestEngine",
  "Finpension",
  "Bitvavo",
  "Coinbase",
  "Crypto.com",
  "BUX",
  "Directa",
  "Disnat",
  "Rabobank",
  "Relai",
  "Parqet",
  "Delta",
  "CoinTracking",
  "Investimental",
];

export function Brokers() {
  return (
    <section aria-labelledby="brokers-title" className="border-y border-line bg-surface py-12">
      <Wrap>
        <h2 id="brokers-title" className="text-center text-[15px] text-ink-2">
          Reads the exports of {BROKERS.length} brokers out of the box, and any other CSV you map once.
        </h2>
      </Wrap>
      <div
        className="marquee-wrap relative mt-8 overflow-hidden [mask-image:linear-gradient(90deg,transparent,#000_12%,#000_88%,transparent)]"
        aria-label={`Built-in layouts: ${BROKERS.join(", ")}`}
        role="img"
      >
        <div className="marquee flex w-max gap-3" aria-hidden>
          {[...BROKERS, ...BROKERS].map((name, i) => (
            <span
              key={i}
              className="whitespace-nowrap rounded-full border border-line bg-bg px-4 py-2 text-sm font-medium text-ink-2"
            >
              {name}
            </span>
          ))}
        </div>
      </div>
      <Wrap>
        <p className="mt-6 text-center text-xs text-ink-3">
          Broker names are trademarks of their owners. Stonqs is not affiliated with or endorsed by any of them.
        </p>
      </Wrap>
    </section>
  );
}

const RETURNS = [
  {
    question: "How did my investments do?",
    name: "Time-weighted return",
    body: "Deposits and withdrawals are taken out, so a big top-up never passes for good performance. This is the figure to hold against a benchmark.",
  },
  {
    question: "How did I do?",
    name: "Money-weighted return",
    body: "Your timing counts. Buying before a rally helps, adding right before a fall hurts. Realised gains come from the actual lots a sale used up.",
  },
];

export function Returns() {
  return (
    <section aria-labelledby="returns-title" className="py-24 md:py-32">
      <Wrap>
        <Reveal>
          <h2
            id="returns-title"
            className="max-w-[18ch] text-4xl font-semibold leading-[1.08] tracking-[-0.03em] md:text-5xl"
          >
            Two returns, and it tells you which is which.
          </h2>
        </Reveal>
        <div className="mt-16 grid gap-12 md:grid-cols-2 md:gap-0">
          {RETURNS.map((r, i) => (
            <Reveal
              key={r.name}
              delay={i * 0.08}
              className={i === 1 ? "md:border-l md:border-line md:pl-12" : "md:pr-12"}
            >
              <p className="text-2xl font-medium leading-snug tracking-tight text-ink md:text-[28px]">
                &ldquo;{r.question}&rdquo;
              </p>
              <h3 className="mt-6 font-mono text-sm font-medium text-accent">{r.name}</h3>
              <p className="mt-3 max-w-[48ch] leading-relaxed text-ink-2">{r.body}</p>
            </Reveal>
          ))}
        </div>
      </Wrap>
    </section>
  );
}

export function Chart() {
  return (
    <section aria-labelledby="chart-title" className="pb-24 md:pb-32">
      <Wrap>
        <Reveal className="max-w-[640px]">
          <h2
            id="chart-title"
            className="text-4xl font-semibold leading-[1.08] tracking-[-0.03em] md:text-5xl"
          >
            Growth you paid for, apart from growth you earned.
          </h2>
          <p className="mt-5 text-lg leading-relaxed text-ink-2">
            Every deposit and withdrawal sits on the same axis as the value, for any period you pick.
          </p>
        </Reveal>
        <Reveal delay={0.1} className="mt-12">
          <div className="shadow-frame overflow-hidden rounded-2xl border border-line bg-bg p-3 md:p-5">
            <Shot
              name="chart"
              className="h-auto w-full"
              alt="The Value and flows chart: portfolio value from September 2023 to September 2026, rising to about 68,000 EUR, with the deposits marked along the bottom and the pointer on December 9, 2025 at 44,036.28 EUR."
            />
          </div>
        </Reveal>
      </Wrap>
    </section>
  );
}
