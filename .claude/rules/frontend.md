# Frontend layers (`app/src`)

Dependencies point one way: `screens/ → components/domain/ → components/ui/ → styles/`. What
crosses to Rust is `.claude/rules/ui-boundary.md`; the assistant's panel is
`.claude/rules/ai-assistant.md`.

## Data

- `lib/queries.ts` is the data layer: every query key is spelled there once (`keys.*`) and every query is a hook (`usePositions`, `useIncome`, …). A screen does not call `useQuery` and never builds a key inline — a shared key is what makes two screens reuse one calculation. A hook with a nullable id or an absent period stays disabled itself, so callers carry no `enabled` and no `!`.
  A reporting hook takes `source?: Source` last and puts it last in its key, so a call without one
  keys identically to before and `invalidate(keys.positions())` still covers every source.
- Invalidation goes through `useInvalidate()` with a named group: `invalidate(...affects.accounts)`. The groups are keyed by the `scope` string the host puts on `data:changed` (`events.rs`), so a mutation's `onSuccess` and the host event invalidate the same list; `App` just forwards the event's kind. Never invalidate everything.
- The wire types are one module with many files: `lib/types/` is a barrel, so a screen imports from
  `lib/types` and never from a file inside it. A new type joins the file of its subject and is
  re-exported by `index.ts` — a name that would collide with one already there is the sign that two
  different wire shapes are being given one name, not an invitation to merge them.
- The date every reading screen answers for is `lib/asOf.tsx` (`useAsOf`), a **session** context,
  not `UiState`: a date restored from disk would open the app in the past without anybody asking.
  `AsOfPicker` wears the `.scope` look beside `ScopePicker` — two lenses, one control shape — and
  `AsOfBanner` sits above every screen while it is not today, because a past portfolio otherwise
  reads as one whose prices stopped updating. A screen reads `useAsOf().date` where it used to call
  `today()`; a **form default and anything that writes keeps real `today()`**, so viewing the past
  cannot backdate what is entered. The panel sends that date to `ai_send` (`as_of`), where the host
  only *states* it: the tools still answer for today, and an alert created mid-turn must not be
  dated 2023.
- A period is resolved by the core (`period_ranges`), never date arithmetic in the UI: `PeriodControl` picks an id, the screen hands the resulting `PeriodRange` to a query hook. `lib/periods.ts` owns only the labels of the shipped seven. See ADR-0018. A quote window (the 90-day sparkline) is not a period and does not go through it: `useQuoteWindow` in `lib/queries.ts` owns that arithmetic, so no screen subtracts days of its own.
- The axis is open at one end (ADR-0026): the shipped `PeriodPreset`s plus the user's own `PeriodSpec`s (`AppSettings::periods`), one flat list keyed by an opaque `PeriodId`. A screen never branches on which kind it picked, and an id that no longer resolves falls back through `pickRange` rather than leaving the screen periodless. Removing a shipped preset only records it in `hidden_presets`; `period_delete` refuses to empty the strip.

## Where a piece of UI lives

- `components/ui/` holds primitives that know nothing about the domain. Such a component never imports `lib/api`, never calls `useQuery`, and never names a domain type.
- `components/domain/` holds every reusable piece that names a domain type (`Instrument`, `KindTag`, `ScopePicker`, `PeriodControl`, `AllocationBar`, `ListingPicker`, `MarketRefresh`, `SecurityPopup`). What stays in `components/` itself is infrastructure only — `Page`, `ErrorBoundary`. Value rendering is not domain-aware — `Money` and friends live in `components/ui/`, over `lib/format` alone.
- An archetype (`components/Page.tsx`) says which slots a screen may fill. A `registry` may carry `controls` only because a column of it is computed over a period rather than read off a date (the positions table showing TWR); its total still belongs in the summary line, never in `metrics`.
- `screens/<Screen>/index.tsx` is composition only — queries, state, `Page` slots. A panel longer than ~80 lines moves to its own file; a screen file over ~250 lines gets split.
- One screen is one chunk: `App` holds every screen behind `lazy()` and one `Suspense`, so a new
  screen costs the first paint nothing. `Onboarding` is eager (it is what an empty database shows),
  and so is the dock — it draws while a screen is still arriving. The AI panel and the markdown
  renderer load the same way, on first use. `manualChunks` is deliberately unset: a hand-written
  chunk list is the screen map copied into a second file, and the copies drift.
- A screen puts controls of its own in the dock through `lib/dock.tsx` (`DockSlot`, `useDockSlot`),
  never by looking the element up by id: a lookup can only run after the commit, so the screen
  renders once into nothing and then fixes itself up a render later.
- A theme may come from a plugin (ADR-0070). `UiState::theme` then holds
  `plugin:<plugin id>/<theme id>`; `lib/theme.ts` owns that spelling in one constant, applies the
  theme's **base** scheme through the same `data-theme` attribute and injects the plugin's
  stylesheet as the one `#plugin-theme` style element. Injected rather than linked, so a theme
  file needs no origin and its author writes plain `:root` rules. A preference naming a plugin
  that is gone is **kept**, not dropped — the stylesheet simply never arrives and the base scheme
  is what shows.
- The updater is the frontend's, not the host's (ADR-0063): `lib/updates.tsx` owns the check (once
  a calendar day, `UiState::updates`), `UpdateDialog` is the only place a version is offered, and
  `api.ts` keeps the plugin's handle so nothing else holds an installer. An automatic check that
  fails is silent; one the user pressed answers. Desktop only — `useUpdates()` is null elsewhere.
- The navigation is `components/Nav`: Favorites (Overview always first, never unpinned), sections
  folded to one open at a time, Settings and the two lenses at the foot. One element at every
  width — the side rail from 1000px, below that a sheet behind the tab bar's `Menu`, whose tabs are
  Overview and the first three favourites. Its arrangement is `UiState::nav`, stored as bare ids;
  `Nav/model.ts` owns which screens and sections exist and reads a stored order over them (unknown
  ids dropped, missing ones appended), so a new screen only needs a place in `SECTIONS`. Rows are
  reordered and pinned by dragging (`Nav/useReorder`, a touch must be held first); a screen moves
  only within its own section.
- `Settings` is a strip of categories, not one long page: `screens/Settings/model.ts` lists them, one
  panel file each, `index.tsx` only switches. `Tabs` (`components/ui`) owns that layout — one shape at
  every width, the strip scrolling sideways rather than becoming a side rail.
- CSS lives one file per primitive under `styles/ui/`; `tokens.css` holds variables only. A screen does not declare its own classes: if a look is missing, the primitive gains a prop, never a copy.
- Enum labels live in `lib/kinds.ts` alone — a screen never declares its own `KIND_LABELS`; counted nouns go through Lingui's `<Plural>`/`plural()`, never a per-screen form table.

## Keyboard

- One `keydown` listener for the window, in `lib/shortcuts.tsx` (ADR-0072). A shortcut is a row in
  `COMMANDS` answered with `useCommand` / `<Command>`, never a `keydown` listener of a screen's own.
  Palette, `?` list, tooltips, `aria-keyshortcuts` and the macOS menu bar read the catalogue.
- An overlay takes the keyboard with `useLayer` (`Modal` does it for you): `Escape` closes the
  newest layer only, and commands are off under a layer unless it lets them `pass`. A modal also
  gets `useDialogFocus` (focus in, `Tab` trapped, focus returned); under a coarse pointer it focuses
  the dialog, not a field, so no on-screen keyboard appears unasked.
- A bare key (`n`, `g p`, `[`, `?`) never fires in a field, a menu or a list box, and
  `UiState::shortcuts.single_keys` turns them all off (WCAG 2.1.4). Letters match by what they type
  on a Latin layout, by position on any other, so a Cyrillic layout still has ⌘K.
- A screen's primary "create" answers `new` with its own label. A search box is found by
  `data-search` (`SearchBox` sets it), and a period strip answers `[` / `]`. `mod+Enter` saves a
  `FormDialog`, and ↑/↓ move between rows of a `DataTable` in the same column.

## The primitives own their shape

- A screen does not hand-write `<table>`, `<form>`, a loading string, or `className="err"` — those are `DataTable`, `FormDialog`, `Async`.
- Values are rendered by a component (`Money`, `Percent`, `Num`, `Quantity`, `Rate`, `Stat`), not by calling `format*` in markup: the `num` class and the sign colour belong to the value.
- `ListRow` owns every list line: a `pick`, a `lead` (logo, date, dot, icon), `title`/`sub`, `value`/`meta`, `end`, `foot`. Slots that are absent are not rendered, so there are no per-shape column variants — only looks the row wears: `box` (a card), `onClick` (tappable), `on`, `big`, `wrap`, `top`. `DataTable`'s card mode is a `ListRow`.
- `Bar` owns every horizontal track: a `fill`, `segments`, or a weight with its `tick`, `over` and `gap`. Colour is always a palette slot via `slotVar` (`lib/plot`), never a class on the bar — a slot class would paint the whole row.
- `Form` owns the only `<form>` and `FormDialog` is `Modal` + `Form`: a screen's form is a list of `Field`/`CheckField`/`FieldSet` and nothing else. Button wording lives in `components/ui/formLabels.ts` (`SUBMIT`, `SUBMITTING`, `CANCEL`) — a screen passes `ready` (what blocks saving), `busy`, `error`, and at most a `submitLabel`/`busyLabel` when the verb is not "Save". `FormDialog`'s submit sits in the modal footer but belongs to the form through `form=<id>`, so Enter saves; `lead` is for a button of its own meaning (delete, disable). `Field` renders the `<select>` itself when given `options` (with `placeholder` for the empty choice) — a screen never writes `<option>`.
- `DataTable` owns the only `<table>`. A screen describes columns (`cell`, `align`, `width`, `only: "wide"`, `card`); column widths live there, never as a per-screen CSS rule. Below 700px the same columns render as cards — derived by default, or via a `card` presenter where a screen's card is its own shape; `card={false}` keeps a dense preview as a scrolling table. Not fitting is the table's own business, never a screen's: `DataTable` measures its wrapper and scrolls sideways only once the columns really do not fit, because a scroll container is also a sticky container and while it fits the sticky header must keep sticking to the page. `sizing="content"` opts out — there the caller owns the `Scrolly x`.
  Ordering is the table's too: a column declares `sort` (the value its rows compare by, never the
  rendered cell) and its heading becomes a button cycling descending, ascending, then back to the
  order the screen handed over — text columns open ascending, figures at the largest, and
  `sortFirst` overrides that. Rows are ordered inside each `Section`, so a grouped table keeps its
  groups; an absent value sorts last both ways, because "no price" is not "the lowest price".
  A table whose order must outlive the screen passes `sort`/`onSortChange` and stores it itself;
  without them the table keeps the order in its own state.
  A column without `sort` is simply not sortable — a sparkline, a checkbox, a row-actions cell —
  and `ariaLabel` explains a column whose heading does not speak for itself — it is both the
  heading's accessible name and its `data-tip` tooltip, so the explanation is one prop, not two.
- A tooltip is a hint, not a paragraph. `data-tip` (and `Metric`'s `tip`) carries **one short
  sentence** saying what the figure is — no second sentence, no caveat, no comparison with the
  figure on the next screen. The full explanation of what a number means belongs to
  `docs/user-guide/` and `docs/ai-reference/`, where the assistant can be asked for it
  (`.claude/rules/assistant-docs.md`); a bubble long enough to need reading twice is skipped
  entirely, so it explains less than a short one. Where the bubble goes is never a screen's
  business: `TooltipLayer` measures it, keeps it inside the window and flips it under the anchor
  when there is no room above, and `styles/ui/overlay.css` caps its width — a tip that needs a
  position of its own is a tip that is too long. The one exception is the sync chip in error:
  its tip lists each refresh failure as what to fix (`FailureCause` → `remedy`), one paragraph
  each, and a tip containing `\n` renders as `tip--long`.
- A heading does not carry a line of small print explaining it: the explanation goes behind an
  info mark (`Panel`'s `info`, `InfoHeading`, `InfoTip`), under the same one-sentence rule.
  `Panel`'s `note` stays for a figure beside the title — a count, a window, a file name.

## The dashboard

- Every plot a widget draws fills its tile: the dashboard passes `height="fill"` and `Chart`
  measures `.chart__plot` instead of taking a pixel count, so a tile dragged taller is drawn into
  rather than padded out. `.w__body` is a flex column for exactly that reason — a filling chart
  takes what the notes above it leave. A screen's panel keeps a fixed height, because there the
  page is what scrolls. A list-shaped visual (`Contributions`, `DriftBars`) fills by letting its
  rows breathe up to a comfortable height, not by stretching them without end.
- A widget configures itself: `WidgetDef::fields` names what its dialog offers, and every widget
  that reads data offers `source` (its own data scope) on top of whatever is specific to it —
  `period`, `taxonomy`, `count`, `benchmark`, `target`. `sourceOf` turns the stored `cfg.source`
  into the `DataScope` the hooks pass to the host; absent means "follow the picker".
- The board is a flow of tiles on a 12-column grid (ADR-0029). A widget stores one width in
  twelfths plus a height in rows; `screens/dashboard/grid.ts` scales that onto the 12/6/2 columns
  the screen affords, and the catalog owns `size` (what a new widget gets) and `min` (where a
  drag stops). Both gestures are pointer events listened for on `window` (`usePointerDrag`), never
  HTML5 drag and drop, which does not exist under a finger, and never pointer capture, which a
  reorder loses: any edge or corner resizes the tile and its header moves it, the moved tile
  following the pointer over a placeholder that reorders under it (`dropOrder` + `useReflow`), with
  selection suppressed and widget bodies not hit-tested while the gesture runs. No second control,
  and no per-breakpoint layout to keep in step. A board leaves the app as a file through
  `boardToFile`/`boardFromFile`, which is also the format of `lib/defaultDashboard.json`: the
  shipped dashboard is authored by exporting one, never by editing code.
- The widget catalog is `screens/dashboard/widgets/`: `model.ts` (types and shared helpers), one file per catalog group (`chart`, `list`, `value` + `metrics`, `tiles`), and `index.tsx` as the registry alone. A registry file declares no components and a group file exports nothing but components — that is what keeps fast refresh working.

## i18n

- Lingui with English as the source language: `lib/i18n.ts` is the only module that knows locales
  exist, `format.ts` asks it which one is active instead of pinning a locale, and month names come
  from `Intl`. A literal is wrapped in `<Trans>`, `` t`…` `` or `` msg`…` `` — a label table holds
  `MessageDescriptor`s and resolves through `i18n._` at render, never a plain string. Plurals are
  ICU (`<Plural>`, `plural()`), never hand-written forms. `pnpm i18n:extract` refreshes
  `src/locales/{en,ru}/messages.po` and must report no missing translations.
- `lingui/no-unlocalized-strings` is an **error**: its ignore list names what is not text at all
  (wire enums, class names, CSS lengths, selectors, event names). A genuine exception is an inline
  `eslint-disable-next-line` with the reason, not a widening of the list.

`pnpm lint` (eslint) and `pnpm lint:css` (stylelint, `no-duplicate-selectors`) guard this; `pnpm format` is prettier at the same 110-column width as rustfmt.
