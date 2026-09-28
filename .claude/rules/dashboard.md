---
paths:
  - "app/src/screens/dashboard/**"
  - "app/src/lib/defaultDashboard.json"
  - "app/src/lib/uiState.ts"
  - "app/src/components/charts/**"
  - "app/mockups/**"
---

# Dashboard (`screens/dashboard/`)

- A widget is a declaration: catalog row (label, icon, group, `size`, `min`, `fields`) + a `Render` from shared primitives. The tile draws icon, title, period chip; a body writes no header, footer or empty state.
- Catalog: `widgets/model.ts` (types/helpers), group files (`chart`, `list`/`periodLists`, `tiles`, `value`/`activity`/`ratio`/`progress` over `figure`, `metrics` catalog), entries in `catalog/` (`lists`, `numbers`, `charts`, `text`), `index.tsx` = registry only. Registry files declare no components; group files export only components (fast refresh).
- Always read the catalog through `useWidgetCatalog()` (never `WIDGETS`); `of(type)` keeps a tile whose plugin is gone. Plugin widgets = `InstalledWidget`, stored as `plugin:<plugin>/<widget>`, body `widgets/plugin.tsx` (see `plugins.md`).
- One shape = one widget: goal/limit/FI are one `progress` widget with `track` (`model.trackOf`, `TRACK_LABELS`); old types migrate in `parseUiState` by type.
- Data widgets offer `source` (own scope; `sourceOf` → `DataScope`; absent = follow picker) plus their own fields (`period`, `taxonomy`, `count`, `benchmark`, `target`).
- Grid (ADR-0029): 12 columns; width in twelfths + height in rows; `grid.ts` maps to 12/6/2 columns. Catalog `size` = default, `min` = drag stop; `grid.limitsOf` = `cfg.min_w` else `min.w` (ADR-0074), offered in every non-plain widget's dialog. On a two-column board a widget with min ≥ 4/12 takes the row (`grid.shownSpan`). No per-widget mobile flags.
- Gestures: pointer events on `window` (`usePointerDrag`), no HTML5 DnD, no pointer capture. Edges/corners resize, header moves; placeholder reorders (`dropOrder` + `useReflow`); selection suppressed, bodies not hit-tested during drag.
- Board width: `.wboard` + `useBoardColumns(gridRef)`; CSS `@container board`. Tile = `container-type: size` (`.w` over `.box`); figures scale with `clamp(…, min(11cqw, 26cqh), …)`.
- Charts fill tiles: `height="fill"`, `Chart` measures `.chart__plot`; `.w__body` is a flex column. Screen panels keep fixed heights. List visuals (`Contributions`, `DriftBars`) grow rows up to a comfortable height.
- Boards export/import via `boardToFile`/`boardFromFile`; `lib/defaultDashboard.json` is authored by exporting a board, never edited in code.
- Check new widgets in `app/mockups/widgets.html` (`pnpm mockup`) before running the app.
