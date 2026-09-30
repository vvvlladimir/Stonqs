import { describe, expect, it } from "vitest";
import { taxRateOf, windowFrom } from "./model";

describe("windowFrom", () => {
  it("counts whole years back", () => {
    expect(windowFrom("2026-09-30", 1)).toBe("2025-09-30");
    expect(windowFrom("2026-09-30", 5)).toBe("2021-09-30");
  });

  it("clamps a leap day onto a common year", () => {
    expect(windowFrom("2024-02-29", 1)).toBe("2023-02-28");
    expect(windowFrom("2024-02-29", 5)).toBe("2019-02-28");
  });

  it("keeps a leap day when the year back is a leap year too", () => {
    expect(windowFrom("2024-02-29", 4)).toBe("2020-02-29");
    // 2000 is a leap year, 1900 is not: the century rule, both ways.
    expect(windowFrom("2004-02-29", 4)).toBe("2000-02-29");
    expect(windowFrom("2004-02-29", 104)).toBe("1900-02-28");
  });
});

describe("taxRateOf", () => {
  it("takes a percent, a comma included, and refuses the rest", () => {
    expect(taxRateOf("")).toEqual({ invalid: false });
    expect(taxRateOf("26,375")).toEqual({ rate: "26.375", invalid: false });
    expect(taxRateOf("101")).toEqual({ invalid: true });
    expect(taxRateOf("-1")).toEqual({ invalid: true });
    expect(taxRateOf("abc")).toEqual({ invalid: true });
  });
});
