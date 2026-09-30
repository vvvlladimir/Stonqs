---
paths:
  - "app/src/**"
  - "app/mockups/**"
---

# Frontend (`app/src`)

Layers point one way: `screens/ → components/domain/ → components/ui/ → styles/`. IPC rules: `ui-boundary.md`. Dashboard: `dashboard.md`.

## Data
- `lib/queries/`: keys spelled once in `keys.ts`, invalidation groups in `invalidation.ts`, every query a hook. Screens never call `useQuery` or build keys. Hooks with a null id/absent period disable themselves. Reporting hooks take `source?: Source` last, last in the key.
- Invalidate via `useInvalidate()` + named group (`...affects.accounts`), keyed by the host's `data:changed` scope. Never invalidate everything.
- `lib/types/` is a barrel; import from `lib/types`. New type → its subject file + `index.ts`. A name collision means two shapes, not a merge.
- `UiState` is saved whole: write as a **patch** `save((ui) => ({ ...ui, … }))` against the freshest copy; never spread the render's `ui`. Patches before settings load are dropped.
- As-of date is `lib/asOf.tsx` (`useAsOf`), session only, not persisted. `AsOfPicker` beside `ScopePicker`; `AsOfBanner` while not today. Readers use `useAsOf().date`; form defaults and writes keep real `today()`. `ai_send` gets `as_of` but tools answer for today.
- Periods resolved by the core (`period_ranges`, ADR-0018): `PeriodControl` picks an id, hook gets `PeriodRange`. `lib/periods.ts` = labels only. Quote windows are `useQuoteWindow`. Period ids are opaque (presets + `AppSettings::periods`, ADR-0026); unresolved id falls back via `pickRange`; hiding a preset → `hidden_presets`; `period_delete` won't empty the strip.

## Where UI lives
- `components/ui/`: domain-free primitives (no `lib/api`, no `useQuery`, no domain types). `components/domain/`: reusable pieces naming domain types. `components/` root: infrastructure only (`Page`, `ErrorBoundary`). `Money` & co. are `ui/` over `lib/format`.
- `components/Page.tsx` archetypes define slots; a `registry` may carry `controls` only for a period-computed column.
- `screens/<Screen>/index.tsx` = composition only. Panel > ~80 lines → own file; screen file > ~250 → split; state machine → `use<Thing>.ts`; big component → folder named after it.
- Each screen is `lazy()` behind one `Suspense`; `Onboarding` and the dock eager; AI panel and markdown lazy. No `manualChunks`.
- Dock controls via `lib/dock.tsx` (`DockSlot`, `useDockSlot`), never by id lookup.
- Tour `lib/tour/` (ADR-0077): `steps.ts` = content (screen, optional `data-tour` anchor, title, body), names no figure/instrument; missing anchor skipped; writes only `UiState::tour`; once per profile. The sources question is the onboarding gate's second step (ADR-0091), not the tour's.
- External links: `lib/links.ts` (`useExternalLinks`, once in `App`) sends `http(s)` clicks to `api.openUrl`. Components write plain `<a href>`.
- Updater (ADR-0063): `lib/updates.tsx` checks once a day (`UiState::updates`); `UpdateDialog` offers; `lib/api/updates.ts` holds the handle. Auto-check fails silently. Desktop only.
- Nav `components/Nav`: Favorites (Overview first, fixed), one section open, Settings + lenses at the foot; rail ≥1000px, sheet below (tab bar = Overview + first three favourites). `UiState::nav` = bare ids; `Nav/model.ts` owns `SECTIONS` (unknown dropped, missing appended). Drag via `Nav/useReorder` (touch hold), within section only.
- Settings = category strip (`screens/Settings/model.ts`, one panel file each) via `Tabs`.
- Enum labels only in `lib/kinds.ts`; counts via `<Plural>`/`plural()`.

## Styles
- One CSS file per primitive in `styles/ui/`; `tokens.css` = variables. Screens declare no classes — add a primitive prop.
- Top-level surface = `.box` (`styles/ui/surface.css`, ADR-0075): border, large radius, `--box-pad`, `container-type: inline-size`. Tighter box sets `--box-pad`, never a padding shorthand, never from a query on itself. Inside: `--r`/`--r-input`.
- Width is a container, not the window (ADR-0073): shape-changing blocks declare `container-type` (`.wfall`, `.drift`, `.calbox`); thresholds are content box. Never `window.innerWidth`.

## Primitives own their shape
- No hand-written `<table>`, `<form>`, loading strings or `className="err"` → `DataTable`, `FormDialog`, `Async`.
- Values via `Money`, `Percent`, `Num`, `Quantity`, `Rate`, `Stat` — not `format*` in markup. `Figure` = number + note; change = `Delta` chip; figure over a track = `Progress` (headline optional).
- `useOverflow` (`lib/overflow.ts`) marks scroll fades with a class, not state.
- `ListRow` owns list lines (slots `pick`, `lead`, `title`/`sub`, `value`/`meta`, `end`, `foot`; looks `box`, `onClick`, `on`, `big`, `wrap`, `top`). `DataTable` card mode is a `ListRow`.
- `Bar` owns horizontal tracks (`fill`, `segments`, weight with `tick`/`over`/`gap`); colour = palette slot via `slotVar`; caller passes a 0–1 `share`, `lib/plot.trackWidth` clamps. `ShareBar` from `ShareSlice`s for "what is this made of" (`legend={false}`, `tip`).
- `Form` owns `<form>`; `FormDialog` = `Modal` + `Form`; content = `Field`/`CheckField`/`FieldSet`. Wording from `components/ui/formLabels.ts`; screen passes `ready`, `busy`, `error`, optional `submitLabel`/`busyLabel`; submit via `form=<id>`; `lead` for a distinct button. `Field` renders `<select>` from `options` (+`placeholder`).
- `DataTable` owns `<table>`: columns (`cell`, `align`, `width`, `only: "wide"`, `card`); cards below 700px (`card={false}` keeps a scrolling table); scrolls sideways only when columns don't fit (`sizing="content"` opts out). Sorting: column `sort` value; cycles desc/asc/original (text starts asc; `sortFirst`); within each `Section`; absent last both ways; persist via `sort`/`onSortChange`. `ariaLabel` = heading name + tooltip.
- Tooltips (`data-tip`, `Metric` `tip`): **one short sentence**; longer explanations go to the user guide. Placement is `TooltipLayer`'s. Only exception: sync chip error (`FailureCause` → `remedy`, `\n` → `tip--long`).
- Headings carry no small print: use `Panel` `info`, `InfoHeading`, `InfoTip`. `Panel` `note` = count/window/file name.

## Keyboard, palette, menu bar (ADR-0072)
- `lib/commands/` (via barrel): commands are rows in `catalog.ts`, answered by `useCommand`/`<Command>` — no screen `keydown` listeners. A screen-owned command names its `screen`.
- Exclusive options: `useChoice`, published by the owner (`PeriodControl`; shell for scope, scheme, profile).
- macOS menu: `components/domain/menuBar/model.ts`. Bare keys never accelerators. Everything also reachable from the palette.
- Overlays take the keyboard with `useLayer` (`Modal` does); `Escape` closes newest only. Modals use `useDialogFocus` (no auto-focus on a field under coarse pointer).
- Bare keys never fire in fields/menus/listboxes; `UiState::shortcuts.single_keys` disables them. Letters by Latin value, else by position.
- Primary create answers `new`; search = `data-search` (`SearchBox`); period strip `[`/`]`; `mod+Enter` saves `FormDialog`; ↑/↓ in `DataTable`.

## i18n
- Lingui, English source; `lib/i18n.ts` is the only locale-aware module; `format.ts` asks it; months from `Intl`. Literals in `<Trans>`, `` t`…` ``, `` msg`…` ``; label tables hold `MessageDescriptor`s resolved via `i18n._`. ICU plurals.
- `lingui/no-unlocalized-strings` is an error; its ignore list is for non-text only. Exceptions: inline disable with reason.

## Tests
- `pure`: `src/**/*.test.ts`, node, no DOM or `lib/api`. `dom`: `src/**/*.dom.test.tsx`, jsdom via `src/test/host.tsx`, host mocked at IPC (`mockIPC`), never by stubbing `lib/api`/hooks; unanswered commands fail by name. `src/test/setup.ts` only fills jsdom gaps.
- DOM tests only for user sequences; answers generated from the core (wizard fixture). Assert what the user sees; `findBy*`/`userEvent`, no manual `act`. Tests beside code; i18n rule off for tests and `src/test/`.
- Browser host: `VITE_BROWSER_HOST` makes `lib/api/browserHost.ts` forward every IPC call over HTTP to `src-tauri/examples/browser_host`, which answers with the **real command functions** over a fresh demo profile — no fixture to drift. `pnpm dev:browser` is the playground (`?screen=<id>` opens a screen); commands taking `AppHandle`, async ones, paths, keychain and network are `DESKTOP_ONLY` and answer an error. A new command must be sorted into `routes.rs` or `DESKTOP_ONLY` (`tests/browser_host.rs`).
- `e2e/` (`pnpm e2e`, Playwright over the browser host, served as a `development`-mode build so React's warnings survive): every screen in WebKit (en, ru) and Chromium at 380/700/1280px fails on an uncaught error, a `console.error`, a host error answer, a routed-less command, sideways scroll, or — in the `webkit` project only, it is the slow check — a serious WCAG A/AA axe violation not in `KNOWN` (a debt list by rule + element; remove an entry once fixed). One test per screen, one page load. A new screen joins `SCREENS` there.
- The host answers a repeated `READS` command from memory until any other command runs; `a_read_asked_twice_answers_the_same` is what makes that safe. A read of state a write changes belongs in `others`.
