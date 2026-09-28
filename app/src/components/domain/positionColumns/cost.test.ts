/** A purchase figure is only ever offered under a named method (ADR-0035). */

import { describe, expect, it } from "vitest";
import { needsCostQuery, resolveColumnIds } from "./cost";

describe("resolveColumnIds", () => {
  it("leaves a choice made after the naming alone", () => {
    const ids = ["symbol", "cost-fifo", "unreal-average"];
    expect(resolveColumnIds(ids, "FIFO")).toBe(ids);
  });

  it("reads a choice stored before the naming as the portfolio's own method", () => {
    expect(resolveColumnIds(["cost", "unrealized", "realized"], "AVERAGE_COST")).toEqual([
      "cost-average",
      "unreal-average",
      "real-average",
    ]);
    expect(resolveColumnIds(["cost"], "FIFO")).toEqual(["cost-fifo"]);
    // A portfolio with no method named yet is FIFO, as everywhere else.
    expect(resolveColumnIds(["cost"], undefined)).toEqual(["cost-fifo"]);
  });

  it("does not show one column twice when both spellings were stored", () => {
    expect(resolveColumnIds(["cost", "cost-fifo"], "FIFO")).toEqual(["cost-fifo"]);
  });
});

describe("needsCostQuery", () => {
  it("is false while every column is the portfolio's own method", () => {
    expect(needsCostQuery(["symbol", "cost-fifo", "real-fifo"], "FIFO")).toBe(false);
  });

  it("is true for the other method, which costs a second holdings pass", () => {
    expect(needsCostQuery(["cost-average"], "FIFO")).toBe(true);
    expect(needsCostQuery(["cost-fifo"], "AVERAGE_COST")).toBe(true);
  });

  it("is true for a unit price, which the position row does not carry", () => {
    expect(needsCostQuery(["costunit-fifo"], "FIFO")).toBe(true);
  });
});
