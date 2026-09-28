/** A tile's stored width is twelfths; what a narrower board does with it is arithmetic (ADR-0073). */

import { describe, expect, it } from "vitest";
import { GRID_COLS, MAX_ROWS, fitSize, limitsOf, shownSpan, spanAt } from "./grid";

describe("spanAt", () => {
  it("is the stored width itself on a full board", () => {
    expect(spanAt(4, GRID_COLS)).toBe(4);
    expect(spanAt(12, GRID_COLS)).toBe(12);
  });

  it("scales onto a narrower board and never disappears", () => {
    expect(spanAt(12, 6)).toBe(6);
    expect(spanAt(6, 6)).toBe(3);
    // A third of a two-column board rounds to nothing, and a tile of no width is not a tile.
    expect(spanAt(1, 2)).toBe(1);
  });
});

describe("shownSpan", () => {
  const box = { w: 3, h: 6 };

  it("hands a phone row to a tile that needs a third of the board", () => {
    // A 150px plot is a texture, so anything declaring 4/12 or more takes the whole row.
    expect(shownSpan(box, 2, 4)).toBe(2);
    expect(shownSpan(box, 2, 3)).toBe(1);
  });

  it("changes nothing where the board has room", () => {
    expect(shownSpan({ w: 6, h: 6 }, GRID_COLS, 4)).toBe(6);
  });
});

describe("limitsOf", () => {
  const def = { min: { w: 3, h: 2 } } as Parameters<typeof limitsOf>[1];

  it("takes the catalog's minimum when the tile names none", () => {
    expect(limitsOf({ cfg: {} }, def)).toEqual({ w: 3, h: 2 });
  });

  it("lets a tile name its own, and ignores one the board cannot honour", () => {
    expect(limitsOf({ cfg: { min_w: 6 } }, def).w).toBe(6);
    expect(limitsOf({ cfg: { min_w: 0 } }, def).w).toBe(3);
    expect(limitsOf({ cfg: { min_w: 99 } }, def).w).toBe(3);
    expect(limitsOf({ cfg: { min_w: "wide" } }, def).w).toBe(3);
  });
});

describe("fitSize", () => {
  const min = { w: 3, h: 2 };

  it("keeps a drag between the widget's minimum and the board", () => {
    expect(fitSize(min, 1, 1)).toEqual({ w: 3, h: 2 });
    expect(fitSize(min, 99, 999)).toEqual({ w: GRID_COLS, h: MAX_ROWS });
    expect(fitSize(min, 5.4, 4.6)).toEqual({ w: 5, h: 5 });
  });
});
