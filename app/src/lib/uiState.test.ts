/** What a stored layout means when it was written by an older build, or by nobody at all. */

import { describe, expect, it } from "vitest";
import { boardFromFile, boardToFile, parseUiState, type Dashboard } from "./uiState";

const board = (widgets: Dashboard["widgets"]): Dashboard => ({ id: "d-1", name: "Board", widgets });

describe("parseUiState", () => {
  it("answers a blob it cannot read with the shipped defaults", () => {
    for (const raw of [null, undefined, 7, "{}", []]) {
      expect(parseUiState(raw).dashboards.length).toBeGreaterThan(0);
    }
  });

  it("counts a layout written before the tour existed as already offered", () => {
    // Whoever has a stored board is using the app already; offering them the introduction is
    // the one case this flag exists to prevent.
    expect(parseUiState({ dashboards: [board([])] }).tour.done).toBe(true);
    expect(parseUiState({ dashboards: [board([])], tour: { done: false } }).tour.done).toBe(false);
  });

  it("reads a height written before the row shrank as three of today's rows", () => {
    const widget = { id: "w-1", type: "value", w: 6, h: 2, cfg: {} };
    const before = parseUiState({ version: 2, dashboards: [board([widget])] });
    const after = parseUiState({ version: 3, dashboards: [board([widget])] });
    expect(before.dashboards[0].widgets[0].h).toBe(6);
    expect(after.dashboards[0].widgets[0].h).toBe(2);
  });

  it("merges the three progress tiles into one, keeping which track each showed", () => {
    const widgets = [
      { id: "w-1", type: "goal", w: 6, h: 6, cfg: {} },
      { id: "w-2", type: "limit", w: 6, h: 6, cfg: {} },
      { id: "w-3", type: "fire", w: 6, h: 6, cfg: {} },
    ];
    const migrated = parseUiState({ dashboards: [board(widgets)] }).dashboards[0].widgets;
    expect(migrated.map((w) => w.type)).toEqual(["progress", "progress", "progress"]);
    expect(migrated.map((w) => w.cfg.track)).toEqual(["goal", "limit", "fire"]);
  });

  it("re-mints ids a board repeats, so two tiles are never one", () => {
    const twice = [
      { id: "w-1", type: "value", w: 6, h: 6, cfg: {} },
      { id: "w-1", type: "value", w: 6, h: 6, cfg: {} },
    ];
    const ids = parseUiState({ dashboards: [board(twice)] }).dashboards[0].widgets.map((w) => w.id);
    expect(new Set(ids).size).toBe(2);
  });

  it("drops a widget with nothing to identify it rather than failing the whole board", () => {
    const widgets = [{ id: "w-1", type: "value", w: 6, h: 6, cfg: {} }, { nonsense: true }];
    const kept = parseUiState({ dashboards: [board(widgets as Dashboard["widgets"])] });
    expect(kept.dashboards[0].widgets).toHaveLength(1);
  });
});

describe("boardFromFile", () => {
  it("is null for anything that is not a board", () => {
    expect(boardFromFile("not json")).toBeNull();
    expect(boardFromFile("[]")).toBeNull();
    expect(boardFromFile(JSON.stringify({ kind: "stonqs.dashboard", version: 3 }))).toBeNull();
  });

  it("round-trips a board and gives the copy ids of its own", () => {
    const original = board([{ id: "w-1", type: "value", w: 6, h: 4, cfg: { source: null } }]);
    const read = boardFromFile(boardToFile(original));

    expect(read?.name).toBe("Board");
    expect(read?.widgets[0]).toMatchObject({ type: "value", w: 6, h: 4 });
    // A second import of one file must not shadow the first.
    expect(read?.id).not.toBe(original.id);
    expect(read?.widgets[0].id).not.toBe("w-1");
  });

  it("accepts a bare board, which is the shape the shipped dashboard ships in", () => {
    const bare = JSON.stringify(board([{ id: "w-1", type: "value", w: 6, h: 4, cfg: {} }]));
    expect(boardFromFile(bare)?.widgets).toHaveLength(1);
  });
});
