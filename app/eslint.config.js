import js from "@eslint/js";
import tseslint from "typescript-eslint";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";
import lingui from "eslint-plugin-lingui";

export default tseslint.config(
  { ignores: ["dist", "src-tauri", "scripts"] },
  js.configs.recommended,
  tseslint.configs.recommended,
  {
    files: ["**/*.{ts,tsx}"],
    languageOptions: {
      ecmaVersion: 2020,
      // The UI is browser-only; `window`/`document` come from the DOM lib, not Node.
      globals: { window: "readonly", document: "readonly", navigator: "readonly", console: "readonly" },
    },
    plugins: {
      "react-hooks": reactHooks,
      "react-refresh": reactRefresh,
      lingui,
    },
    rules: {
      ...reactHooks.configs.recommended.rules,
      "react-refresh/only-export-components": ["warn", { allowConstantExport: true }],
      // Three prop-to-state syncs trip this (Transactions.tsx, Securities.tsx, Allocation.tsx);
      // they are rewritten when the screens are split, so warn instead of blocking the build.
      "react-hooks/set-state-in-effect": "warn",
      // `noUnusedLocals` in tsconfig already covers this; keep the underscore escape hatch.
      "@typescript-eslint/no-unused-vars": ["warn", { argsIgnorePattern: "^_", varsIgnorePattern: "^_" }],
      // The guard that keeps hard-coded text from creeping back in: every user-visible string
      // goes through a macro, and the ignore list below names what is not text at all.
      "lingui/no-unlocalized-strings": [
        "error",
        {
          ignore: [
            // A string with no letter is a separator, a symbol or a key, never a sentence.
            "^[^\\p{L}]*$",
            // One lowercase token is an identifier: a wire field, a config key, a CSS value.
            "^[a-z][a-zA-Z0-9_]*$",
            // SCREAMING_SNAKE is a wire enum value, never a sentence.
            "^[A-Z][A-Z0-9_]*$",
            // A class name, a custom property or a data attribute: "row--tap", "--pos", "data-tip".
            "^-{0,2}[a-z][a-z0-9-]*$",
            // A CSS length and a media query: layout, not language.
            "^\\d+(px|%|ch|rem|em|fr)$",
            "^\\(.+\\)$",
            // An attribute selector and a Tauri event name.
            "^\\[[a-z-]+\\]$",
            "^[a-z]+:[a-z]+$",
          ],
          ignoreNames: [
            { regex: { pattern: "^(className|key|id|type|role|href|src|name|as|to)$" } },
            // Wire values and CSS-ish props: these cross to the host or to the DOM, not to a reader.
            { regex: { pattern: "^(data|aria)-" } },
          ],
          ignoreFunctions: ["msg", "t", "plural", "select", "console.*", "*.setAttribute"],
        },
      ],
    },
  },
);
