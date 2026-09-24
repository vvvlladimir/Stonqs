import {
  AppleLogoIcon,
  ArrowRightIcon,
  DownloadSimpleIcon,
  LinuxLogoIcon,
  PlusIcon,
  WindowsLogoIcon,
  type Icon,
} from "@phosphor-icons/react";
import type { ReactNode } from "react";
import { btn, FILES, REPO, Reveal, useOs, Wrap, type Os } from "./ui";

const FAQ: { q: string; a: ReactNode }[] = [
  {
    q: "Is it really free?",
    a: "Yes. Stonqs is open source under the AGPL, with no paid tier, no account and no ads.",
  },
  {
    q: "Where is my data stored?",
    a: "In a database file inside your profile folder, on your own disk. Give the profile a password and the file is encrypted. Back it up like any other file.",
  },
  {
    q: "My broker is not on the list. Can I still import?",
    a: "Yes. The import reads any CSV: you map the columns once, see every row before it is saved, and can keep the mapping as a layout for the next export. Interactive Brokers Flex statements are read directly.",
  },
  {
    q: "Which returns does it show?",
    a: "Time-weighted return for how the investments did, money-weighted return for how you did, and realised gains from the actual lots each sale used up. Every figure works for any period and any group of accounts.",
  },
  {
    q: "Is there a phone app?",
    a: "Not yet. Stonqs runs on macOS, Windows and Linux today.",
  },
  {
    q: "Why does my system warn me on first launch?",
    a: (
      <>
        The alpha builds are not code-signed yet. On macOS, move the app to Applications, then run{" "}
        <code className="rounded-md bg-surface-2 px-1.5 py-0.5 font-mono text-[13px] text-ink">
          xattr -dr com.apple.quarantine /Applications/Stonqs.app
        </code>
        . On Windows, choose More info, then Run anyway.
      </>
    ),
  },
];

export function Faq() {
  return (
    <section id="faq" aria-labelledby="faq-title" className="border-t border-line py-24 md:py-32">
      <Wrap className="grid gap-12 lg:grid-cols-[minmax(0,4fr)_minmax(0,7fr)] lg:gap-20">
        <Reveal>
          <h2 id="faq-title" className="text-4xl font-semibold leading-[1.08] tracking-[-0.03em] md:text-5xl">
            Questions
          </h2>
        </Reveal>
        <Reveal delay={0.05}>
          <div className="border-t border-line-strong">
            {FAQ.map(({ q, a }) => (
              <details key={q} className="group border-b border-line">
                <summary className="flex cursor-pointer items-center gap-6 py-5 text-lg font-medium tracking-tight transition-colors hover:text-accent">
                  {q}
                  <PlusIcon
                    size={18}
                    weight="bold"
                    className="faq-plus ml-auto shrink-0 text-ink-3 transition-transform duration-300"
                  />
                </summary>
                <p className="max-w-[62ch] pb-6 leading-relaxed text-ink-2">{a}</p>
              </details>
            ))}
          </div>
        </Reveal>
      </Wrap>
    </section>
  );
}

const PLATFORMS: {
  os: Os;
  icon: Icon;
  name: string;
  note: string;
  /** The recommended file is last: it renders as the primary button, at the row's right edge. */
  files: { label: string; href: string }[];
}[] = [
  {
    os: "mac",
    icon: AppleLogoIcon,
    name: "macOS",
    note: "Apple Silicon is any Mac from late 2020 on. Older ones are Intel.",
    files: [
      { label: "Intel", href: FILES.macIntel },
      { label: "Apple Silicon", href: FILES.macArm },
    ],
  },
  {
    os: "windows",
    icon: WindowsLogoIcon,
    name: "Windows",
    note: "Windows 10 or 11, 64-bit.",
    files: [{ label: "Installer", href: FILES.windows }],
  },
  {
    os: "linux",
    icon: LinuxLogoIcon,
    name: "Linux",
    note: "x86-64. Ubuntu 22.04, Debian 12 or newer.",
    files: [
      { label: ".deb", href: FILES.deb },
      { label: ".rpm", href: FILES.rpm },
      { label: "AppImage", href: FILES.appImage },
    ],
  },
];

export function Download() {
  const mine = useOs();
  return (
    <section id="download" aria-labelledby="download-title" className="bg-surface py-24 md:py-32">
      <Wrap>
        <Reveal className="max-w-[640px]">
          <h2
            id="download-title"
            className="text-4xl font-semibold leading-[1.08] tracking-[-0.03em] md:text-5xl"
          >
            Free for macOS, Windows and Linux.
          </h2>
          <p className="mt-5 text-lg leading-relaxed text-ink-2">
            Start with the demo portfolio, then bring in your own broker export.
          </p>
        </Reveal>

        <div className="mt-14 grid gap-3">
          {PLATFORMS.map(({ os, icon: I, name, note, files }, i) => (
            <Reveal key={os} delay={i * 0.06}>
              <div
                className={`grid items-center gap-5 rounded-2xl border p-6 transition-colors md:grid-cols-[220px_1fr_auto] md:gap-8 md:px-8 ${
                  mine === os ? "border-accent bg-accent-soft" : "border-line bg-bg"
                }`}
              >
                <h3 className="flex items-center gap-3 text-xl font-semibold tracking-tight">
                  <I size={24} weight="fill" />
                  {name}
                </h3>
                <p className="text-sm text-ink-2">{note}</p>
                <div className="flex flex-wrap gap-2 md:justify-end">
                  {files.map((f, j) => {
                    const primary = j === files.length - 1;
                    return (
                      <a
                        key={f.label}
                        href={f.href}
                        className={`${primary ? btn.primary : btn.ghost} ${btn.small}`}
                      >
                        {primary && <DownloadSimpleIcon size={16} weight="bold" />}
                        {f.label}
                      </a>
                    );
                  })}
                </div>
              </div>
            </Reveal>
          ))}
        </div>

        <div className="mt-10 flex flex-wrap gap-x-8 gap-y-3 text-[15px] font-medium">
          <a className="group inline-flex items-center gap-1.5 text-accent" href={`${REPO}/releases`}>
            All releases and checksums
            <ArrowRightIcon
              size={16}
              weight="bold"
              className="transition-transform group-hover:translate-x-0.5"
            />
          </a>
          <a
            className="group inline-flex items-center gap-1.5 text-accent"
            href={`${REPO}/blob/main/CONTRIBUTING.md#getting-set-up`}
          >
            Build from source
            <ArrowRightIcon
              size={16}
              weight="bold"
              className="transition-transform group-hover:translate-x-0.5"
            />
          </a>
        </div>
      </Wrap>
    </section>
  );
}

export function Footer() {
  const links = [
    ["GitHub", REPO],
    ["Discussions", `${REPO}/discussions`],
    ["Privacy", "/privacy.html"],
    ["Impressum", "/impressum.html"],
    ["Datenschutz", "/datenschutz.html"],
  ];
  return (
    <footer className="border-t border-line py-12">
      <Wrap>
        <div className="flex flex-wrap items-center gap-x-7 gap-y-3 text-sm text-ink-2">
          <a href="/" className="mr-auto flex items-center gap-2.5 text-base font-semibold text-ink">
            <img src="/assets/icon-256.png" alt="" width={24} height={24} />
            Stonqs
          </a>
          {links.map(([label, href]) => (
            <a key={label} href={href} className="transition-colors hover:text-ink">
              {label}
            </a>
          ))}
        </div>
        <p className="mt-8 max-w-[90ch] text-[13px] leading-relaxed text-ink-3">
          Stonqs is a tool for recording and analysing your own investments. It is not financial advice, and
          neither are the answers of its assistant. Market data comes from free third-party sources with no
          guarantee of accuracy. This site sets no cookies and loads nothing from other servers.
        </p>
      </Wrap>
    </footer>
  );
}
