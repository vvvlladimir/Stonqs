import { defineConfig } from "@lingui/cli";
import { formatter } from "@lingui/format-po";

/** English is the source language: every msgid in the code is English, other locales are catalogs. */
export default defineConfig({
  sourceLocale: "en",
  locales: ["en", "ru"],
  // Line numbers would churn the catalogs on every unrelated edit.
  format: formatter({ lineNumbers: false }),
  catalogs: [
    {
      path: "<rootDir>/src/locales/{locale}/messages",
      include: ["<rootDir>/src"],
    },
  ],
});
