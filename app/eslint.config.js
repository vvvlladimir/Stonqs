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
      "react-hooks/set-state-in-effect": "error",
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
  {
    // Size guards, so a file split once does not grow back. Blank lines and comments are free.
    // Never widen a limit to let a file through — split it. A genuine exception is an inline
    // `eslint-disable-next-line` with the reason, and it fails once it is no longer needed.
    files: ["**/*.{ts,tsx}"],
    linterOptions: { reportUnusedDisableDirectives: "error" },
    rules: {
      "max-lines": ["error", { max: 300, skipBlankLines: true, skipComments: true }],
      "max-lines-per-function": ["error", { max: 80, skipBlankLines: true, skipComments: true }],
    },
  },
  {
    // A component's body is mostly markup, so it gets more room than logic does.
    files: ["**/*.tsx"],
    rules: {
      "max-lines-per-function": ["error", { max: 150, skipBlankLines: true, skipComments: true }],
    },
  },
  {
    // A test's name is written for whoever reads the failure, and nothing here reaches a
    // window: the one place in `src` where an English string is not a missing translation.
    // `max-lines-per-function` goes with it — a `describe` is a list, not a function body.
    // `src/test/` is the harness the DOM tests mount through: same rule, same reason.
    files: ["**/*.test.ts", "**/*.test.tsx", "src/test/**"],
    rules: {
      "lingui/no-unlocalized-strings": "off",
      "max-lines-per-function": "off",
    },
  },
  {
    // A context and the hook that reads it are one file on purpose: the pair is the primitive,
    // and splitting it to keep fast refresh would cost every reader a hop for a dev-time gain.
    // Editing one of these files reloads the window; that is the trade, and it is why the list
    // is spelled out rather than turned into a pattern — anything else exporting a helper beside
    // a component is still a warning worth reading.
    files: [
      "src/lib/{asOf,dock,nav,updates}.tsx",
      "src/lib/commands/react.tsx",
      "src/lib/tour/index.tsx",
      "src/components/ui/{Async,Menu,Selection,Toast}.tsx",
      "src/components/domain/{MarketRefresh,SecurityCardProvider}.tsx",
    ],
    rules: { "react-refresh/only-export-components": "off" },
  },
);
