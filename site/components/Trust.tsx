import { BookOpenTextIcon, HandPalmIcon, LockSimpleIcon, PowerIcon } from "@phosphor-icons/react";
import { Reveal, Wrap } from "./ui";

const RULES = [
  {
    icon: PowerIcon,
    title: "Off until you turn it on",
    body: "Use your own key for OpenAI, Anthropic or Gemini, or a model running on your own machine.",
  },
  {
    icon: HandPalmIcon,
    title: "Asks before it reads",
    body: "Reading your portfolio needs your permission, once or for the whole chat.",
  },
  {
    icon: LockSimpleIcon,
    title: "Asks every time before it writes",
    body: "No standing permission covers a change. You approve each one.",
  },
  {
    icon: BookOpenTextIcon,
    title: "Explains from the app's own guide",
    body: "When the guide has no answer, it says so instead of guessing.",
  },
];

export function Assistant() {
  return (
    <section aria-labelledby="ai-title" className="bg-surface py-24 md:py-32">
      <Wrap className="grid gap-14 lg:grid-cols-[minmax(0,5fr)_minmax(0,6fr)] lg:gap-20">
        <Reveal className="lg:sticky lg:top-28 lg:self-start">
          <h2
            id="ai-title"
            className="text-4xl font-semibold leading-[1.08] tracking-[-0.03em] md:text-5xl"
          >
            An assistant that asks first.
          </h2>
          <p className="mt-5 max-w-[42ch] text-lg leading-relaxed text-ink-2">
            Ask why a return moved or what fees cost you this year. It answers from your own figures.
          </p>
        </Reveal>
        <div className="grid gap-10">
          {RULES.map(({ icon: I, title, body }, i) => (
            <Reveal key={title} delay={i * 0.06}>
              <div className="grid grid-cols-[auto_1fr] gap-5">
                <span className="flex size-11 items-center justify-center rounded-full bg-accent-soft text-accent">
                  <I size={20} weight="bold" />
                </span>
                <div>
                  <h3 className="text-lg font-semibold tracking-tight">{title}</h3>
                  <p className="mt-1.5 max-w-[48ch] leading-relaxed text-ink-2">{body}</p>
                </div>
              </div>
            </Reveal>
          ))}
        </div>
      </Wrap>
    </section>
  );
}

const CALLS = [
  {
    when: "When quotes are refreshed",
    rows: [
      ["Prices", "Yahoo Finance, or Twelve Data, EODHD and Kraken if you set them up"],
      ["Exchange rates", "European Central Bank, Frankfurter, Yahoo"],
      ["Inflation", "Eurostat and IMF, only if you pick a region"],
      ["Instrument lookup", "Yahoo and OpenFIGI, when you search"],
    ],
  },
  {
    when: "Only when you ask",
    rows: [
      ["AI assistant", "The provider you chose, with your own key"],
      ["Update check", "GitHub, once a day, and it can be turned off"],
    ],
  },
];

export function Privacy() {
  return (
    <section id="privacy" aria-labelledby="privacy-title" className="py-24 md:py-32">
      <Wrap>
        <Reveal className="max-w-[680px]">
          <h2
            id="privacy-title"
            className="text-4xl font-semibold leading-[1.08] tracking-[-0.03em] md:text-5xl"
          >
            Your data is a file you own.
          </h2>
          <p className="mt-5 text-lg leading-relaxed text-ink-2">
            No account, no server, no telemetry. The portfolio is a database on your disk, encrypted when
            the profile has a password. Below is every connection the app makes.
          </p>
        </Reveal>

        <div className="mt-14 grid gap-12 md:grid-cols-[3fr_2fr] md:gap-16">
          {CALLS.map((group, i) => (
            <Reveal key={group.when} delay={i * 0.08}>
              <h3 className="border-b border-line-strong pb-3 text-sm font-medium text-ink-3">{group.when}</h3>
              <dl className="mt-5 grid gap-5">
                {group.rows.map(([what, where]) => (
                  <div key={what} className="grid gap-1 sm:grid-cols-[150px_1fr] sm:gap-6">
                    <dt className="font-medium">{what}</dt>
                    <dd className="text-ink-2">{where}</dd>
                  </div>
                ))}
              </dl>
            </Reveal>
          ))}
        </div>
      </Wrap>
    </section>
  );
}
