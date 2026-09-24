import { motion } from "motion/react";
import { useEffect, useState, type ReactNode } from "react";
import shots from "./shots.json";

export const REPO = "https://github.com/vvvlladimir/stonqs";
const LATEST = `${REPO}/releases/latest/download/`;

export type Os = "mac" | "windows" | "linux";

// Stable file names: release.yml uploads a copy of each build under these.
export const FILES = {
  macArm: `${LATEST}Stonqs_macos_aarch64.dmg`,
  macIntel: `${LATEST}Stonqs_macos_x64.dmg`,
  windows: `${LATEST}Stonqs_windows_x64-setup.exe`,
  appImage: `${LATEST}Stonqs_linux_x86_64.AppImage`,
  deb: `${LATEST}Stonqs_linux_amd64.deb`,
  rpm: `${LATEST}Stonqs_linux_x86_64.rpm`,
};

export const PRIMARY: Record<Os, { href: string; label: string }> = {
  mac: { href: FILES.macArm, label: "Download for macOS" },
  windows: { href: FILES.windows, label: "Download for Windows" },
  linux: { href: FILES.appImage, label: "Download for Linux" },
};

/** The visitor's desktop platform, read from the browser alone; null before hydration and on phones. */
export function useOs(): Os | null {
  const [os, setOs] = useState<Os | null>(null);
  useEffect(() => {
    const ua = navigator.userAgent;
    if (/iPhone|iPad|Android/.test(ua)) return;
    setOs(/Mac/.test(ua) ? "mac" : /Windows/.test(ua) ? "windows" : /Linux/.test(ua) ? "linux" : null);
  }, []);
  return os;
}

export const btn = {
  primary:
    "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-full bg-accent px-5 py-3 text-[15px] font-medium text-accent-ink transition-[transform,filter] duration-300 ease-[cubic-bezier(0.16,1,0.3,1)] hover:brightness-110 active:scale-[0.98]",
  ghost:
    "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-full border border-line-strong bg-surface px-5 py-3 text-[15px] font-medium text-ink transition-[transform,border-color] duration-300 ease-[cubic-bezier(0.16,1,0.3,1)] hover:border-ink-3 active:scale-[0.98]",
  small: "!px-4 !py-2 !text-sm",
};

export const EASE = [0.16, 1, 0.3, 1] as const;

/** Fades a block up as it enters the viewport; MotionConfig in _app turns it off for reduced motion. */
export function Reveal({
  children,
  delay = 0,
  className,
}: {
  children: ReactNode;
  delay?: number;
  className?: string;
}) {
  return (
    <motion.div
      className={className}
      initial={{ opacity: 0, y: 24 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, amount: 0.25 }}
      transition={{ duration: 0.7, delay, ease: EASE }}
    >
      {children}
    </motion.div>
  );
}

/**
 * A real screenshot of the app, in the colour scheme the visitor's OS asks for. Its size comes from
 * shots.json, which scripts/screenshots.mjs rewrites with the images.
 */
export function Shot({
  name,
  alt,
  className,
  priority,
}: {
  name: keyof typeof shots;
  alt: string;
  className?: string;
  priority?: boolean;
}) {
  const { width, height } = shots[name];
  return (
    <picture>
      <source srcSet={`/assets/${name}-dark.webp`} media="(prefers-color-scheme: dark)" />
      <img
        src={`/assets/${name}-light.webp`}
        alt={alt}
        width={width}
        height={height}
        className={className}
        loading={priority ? "eager" : "lazy"}
        fetchPriority={priority ? "high" : "auto"}
        decoding="async"
      />
    </picture>
  );
}

export function Wrap({ children, className = "" }: { children: ReactNode; className?: string }) {
  return <div className={`mx-auto w-full max-w-[1240px] px-5 md:px-8 ${className}`}>{children}</div>;
}
