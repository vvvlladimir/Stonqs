/** The only module that knows locales exist: everything else asks it which one is active. */

import { useEffect, useState } from "react";
import { i18n } from "@lingui/core";
import { useSettings, useStatus } from "./queries";

export type Locale = "en" | "ru";

/** Mirrors the theme preference: an explicit locale, or "follow the OS". */
export type LanguagePreference = "system" | Locale;

/** English is the source language and therefore the fallback for every missing message. */
export const FALLBACK: Locale = "en";

export const LOCALES: Locale[] = ["en", "ru"];

export const PREFERENCES: LanguagePreference[] = ["system", ...LOCALES];

/** Names are written in their own language, so the list reads the same whatever is active. */
export const LOCALE_NAMES: Record<Locale, string> = {
  en: "English",
  ru: "Русский",
};

export function isPreference(value: unknown): value is LanguagePreference {
  return typeof value === "string" && (PREFERENCES as string[]).includes(value);
}

/** Matches a BCP-47 tag by its primary subtag: `ru-RU`, `ru` and `ru-Cyrl-RU` are one locale. */
export function localeOf(tag: string | null | undefined): Locale | undefined {
  const primary = tag?.split(/[-_]/)[0].toLowerCase();
  return LOCALES.find((locale) => locale === primary);
}

export function resolveLocale(preference: LanguagePreference, systemLocale: string | null): Locale {
  if (preference !== "system") return preference;
  return localeOf(systemLocale) ?? localeOf(navigator.language) ?? FALLBACK;
}

/** Catalogs are dynamic imports so only the active language's messages are downloaded. */
async function catalog(locale: Locale) {
  switch (locale) {
    case "ru":
      return (await import("../locales/ru/messages.po")).messages;
    case "en":
      return (await import("../locales/en/messages.po")).messages;
  }
}

export async function activateLocale(locale: Locale): Promise<void> {
  i18n.loadAndActivate({ locale, messages: await catalog(locale) });
}

/**
 * The locale `Intl` formatting follows. Reading it from the i18n runtime keeps one source of
 * truth: a number and the sentence around it can never disagree about the language.
 */
export function currentLocale(): Locale {
  return (i18n.locale as Locale) || FALLBACK;
}

/**
 * Keeps the active locale in step with the saved preference and the OS language.
 * Activation is asynchronous, so the initial locale is activated in `main.tsx`
 * before the first render — this only handles later changes.
 */
export function useLanguage(): { preference: LanguagePreference; locale: Locale } {
  const settings = useSettings();
  const status = useStatus();

  const preference = isPreference(settings.data?.language) ? settings.data.language : "system";
  const locale = resolveLocale(preference, status.data?.system_locale ?? null);
  const [active, setActive] = useState(currentLocale());

  useEffect(() => {
    if (locale === currentLocale()) return;
    let cancelled = false;
    void activateLocale(locale).then(() => {
      if (!cancelled) setActive(locale);
    });
    return () => {
      cancelled = true;
    };
  }, [locale]);

  return { preference, locale: active };
}
