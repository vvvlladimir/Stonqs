import { describe, expect, it } from "vitest";
import type { ImportOptions, ImportProblem, ImportSummary, RowStatus } from "../../lib/types";
import type { PreviewRow } from "./labels";
import { distinctRows, filterRows, rowsToWrite, shapeOf } from "./model";

function draft(over: Partial<NonNullable<PreviewRow["draft"]>> = {}) {
  return {
    account_id: "acc",
    kind: "BUY" as const,
    date: "2024-01-15",
    symbol: "AAPL",
    isin: null,
    security_name: null,
    security_id: null,
    quantity: "10",
    price: "185.50",
    amount: "1855",
    fees: "0",
    taxes: "0",
    currency: "USD",
    fee_currency: null,
    tax_currency: null,
    fx_rate_to_base: null,
    link_id: null,
    external_id: null,
    replaces: null,
    note: null,
    ...over,
  };
}

function row(over: Partial<PreviewRow> = {}): PreviewRow {
  return {
    number: 1,
    part: 1,
    raw: {},
    draft: draft(),
    status: "READY" as RowStatus,
    problems: [],
    ...over,
  };
}

function warning(): ImportProblem {
  return { code: "DIRECTION_FROM_SIGN", severity: "WARNING", row: 1, column: null, message: "" };
}

function summary(over: Partial<ImportSummary> = {}): ImportSummary {
  return {
    total: 0,
    ready: 0,
    duplicates: 0,
    similar: 0,
    updated: 0,
    unknown_securities: 0,
    ignored: 0,
    invalid: 0,
    warnings: 0,
    ...over,
  };
}

function options(over: Partial<ImportOptions> = {}): ImportOptions {
  return {
    create_missing_securities: true,
    new_security_kind: "OTHER",
    import_duplicates: false,
    import_similar: false,
    ...over,
  };
}

describe("filterRows", () => {
  const rows = [
    row({ number: 1, status: "READY" }),
    row({ number: 2, status: "INVALID" }),
    row({ number: 3, status: "READY", problems: [warning()] }),
    row({ number: 4, status: "DUPLICATE" }),
  ];

  it("shows everything under ALL", () => {
    expect(filterRows(rows, "ALL", new Set())).toHaveLength(4);
  });

  it("finds a warned row whatever its status", () => {
    expect(filterRows(rows, "WARNING", new Set()).map((r) => r.number)).toEqual([3]);
  });

  it("finds the rows the user edited by their file row, not their position", () => {
    expect(filterRows(rows, "FIXED", new Set([2, 4])).map((r) => r.number)).toEqual([2, 4]);
  });

  it("filters by status", () => {
    expect(filterRows(rows, "INVALID", new Set()).map((r) => r.number)).toEqual([2]);
  });

  it("keeps every row of a status, never only the first", () => {
    const many = [row({ number: 1 }), row({ number: 2 }), row({ number: 3 })];
    expect(filterRows(many, "READY", new Set())).toHaveLength(3);
  });
});

describe("shapeOf", () => {
  it("separates two operations that differ in what they carry", () => {
    const withFee = row({ draft: draft({ fees: "4.95" }) });
    expect(shapeOf(withFee)).not.toEqual(shapeOf(row()));
  });

  it("separates a negative amount from a positive one", () => {
    const out = row({ draft: draft({ amount: "-1855" }) });
    expect(shapeOf(out)).not.toEqual(shapeOf(row()));
  });

  it("separates two currencies", () => {
    const euros = row({ draft: draft({ currency: "EUR" }) });
    expect(shapeOf(euros)).not.toEqual(shapeOf(row()));
  });

  it("gives a row with nothing to write a shape of its own", () => {
    expect(shapeOf(row({ draft: null, status: "INVALID" }))).toBe("—|INVALID");
  });
});

describe("distinctRows", () => {
  it("keeps one example per shape, in file order", () => {
    const rows = [
      row({ number: 1 }),
      row({ number: 2 }),
      row({ number: 3, draft: draft({ kind: "SELL" }) }),
      row({ number: 4, draft: draft({ fees: "1" }) }),
    ];
    expect(distinctRows(rows).map((r) => r.number)).toEqual([1, 3, 4]);
  });

  it("keeps the first of a shape, so the example is the one the user scrolls to", () => {
    const rows = [row({ number: 7 }), row({ number: 8 })];
    expect(distinctRows(rows).map((r) => r.number)).toEqual([7]);
  });
});

describe("rowsToWrite", () => {
  const s = summary({
    total: 20,
    ready: 10,
    updated: 2,
    unknown_securities: 3,
    duplicates: 4,
    similar: 1,
  });

  it("counts the rows that are ready and the ones that replace a stored operation", () => {
    expect(rowsToWrite(summary({ ready: 10, updated: 2 }), options())).toBe(12);
  });

  it("counts a row waiting on an instrument only while instruments are created", () => {
    expect(rowsToWrite(s, options())).toBe(15);
    expect(rowsToWrite(s, options({ create_missing_securities: false }))).toBe(12);
  });

  it("counts duplicates and near-duplicates only when the user asked for them", () => {
    expect(rowsToWrite(s, options({ import_duplicates: true }))).toBe(19);
    expect(rowsToWrite(s, options({ import_similar: true }))).toBe(16);
    expect(rowsToWrite(s, options({ import_duplicates: true, import_similar: true }))).toBe(20);
  });

  it("never counts an invalid or ignored row", () => {
    const skipped = summary({ ready: 1, invalid: 9, ignored: 5 });
    expect(rowsToWrite(skipped, options({ import_duplicates: true, import_similar: true }))).toBe(1);
  });
});
