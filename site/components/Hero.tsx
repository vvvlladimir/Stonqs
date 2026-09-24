import { DownloadSimpleIcon, GithubLogoIcon } from "@phosphor-icons/react";
import { motion } from "motion/react";
import { btn, EASE, PRIMARY, REPO, Shot, useOs, Wrap } from "./ui";

export function Header() {
  return (
    <header className="sticky top-0 z-30 border-b border-line/70 bg-bg/80 backdrop-blur-xl">
      <Wrap className="flex h-16 items-center gap-8">
        <a href="/" className="flex items-center gap-2.5 text-[17px] font-semibold tracking-tight">
          <img src="/assets/icon-256.png" alt="" width={28} height={28} />
          Stonqs
        </a>
        <nav aria-label="Main" className="hidden items-center gap-7 text-sm text-ink-2 md:flex">
          <a className="transition-colors hover:text-ink" href="#features">
            Features
          </a>
          <a className="transition-colors hover:text-ink" href="#privacy">
            Privacy
          </a>
          <a className="transition-colors hover:text-ink" href="#faq">
            FAQ
          </a>
          <a className="transition-colors hover:text-ink" href="#download">
            Download
          </a>
        </nav>
        <a className={`${btn.ghost} ${btn.small} ml-auto`} href={REPO}>
          <GithubLogoIcon size={16} weight="bold" />
          GitHub
        </a>
      </Wrap>
    </header>
  );
}

export function Hero() {
  const os = useOs();
  const primary = os ? PRIMARY[os] : { href: "#download", label: "Download" };
  return (
    <section aria-labelledby="hero-title" className="relative overflow-hidden">
      {/* One soft accent wash behind the screenshot: depth, not a gradient show. */}
      <div
        aria-hidden
        className="pointer-events-none absolute -right-40 top-10 h-[640px] w-[900px] rounded-full bg-accent-soft blur-3xl"
      />
      <Wrap className="relative grid items-center gap-12 pb-20 pt-14 md:pt-20 lg:grid-cols-[minmax(0,540px)_minmax(0,1fr)] lg:gap-10 lg:pb-28 lg:pt-24">
        <motion.div
          initial={{ opacity: 0, y: 16 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.8, ease: EASE }}
        >
          <h1
            id="hero-title"
            className="text-[40px] font-semibold leading-[1.05] tracking-[-0.035em] md:text-5xl lg:text-[54px]"
          >
            Your portfolio, measured properly.
          </h1>
          <p className="mt-6 max-w-[44ch] text-lg leading-relaxed text-ink-2">
            True returns, realised gains and every currency, tracked on your own computer. Free, open
            source, no account.
          </p>
          <div className="mt-9 flex flex-wrap gap-3">
            <a className={btn.primary} href={primary.href}>
              <DownloadSimpleIcon size={18} weight="bold" />
              {primary.label}
            </a>
            <a className={btn.ghost} href={REPO}>
              <GithubLogoIcon size={18} weight="bold" />
              GitHub
            </a>
          </div>
        </motion.div>

        <motion.div
          className="lg:-mr-40 xl:-mr-56"
          initial={{ opacity: 0, y: 40, scale: 0.97 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          transition={{ duration: 1.1, delay: 0.15, ease: EASE }}
        >
          <Shot
            name="overview"
            priority
            className="h-auto w-full min-w-0"
            alt="The Overview screen of Stonqs: portfolio value, today's change, time-weighted return and money earned, above a chart of value and deposits over two years."
          />
        </motion.div>
      </Wrap>
    </section>
  );
}
