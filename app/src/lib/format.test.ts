/** Money crosses IPC as a decimal string and is rounded here, never by a JS number. */

import { describe, expect, it } from "vitest";
import { formatDecimal, formatPercent, formatQuantity, signOf, toNumber } from "./format";

describe("formatDecimal", () => {
  it("rounds half away from zero on the string itself", () => {
    // 0.005 is not representable as a double: (0.005).toFixed(2) is "0.01" here but the same
    // arithmetic on 1.005 gives "1.00". The digits are carried by hand for exactly that reason.
    expect(formatDecimal("1.005")).toBe("1.01");
    expect(formatDecimal("-1.005")).toBe("-1.01");
    expect(formatDecimal("2.675")).toBe("2.68");
  });

  it("carries the rounding through every nine", () => {
    expect(formatDecimal("9.999")).toBe("10.00");
    expect(formatDecimal("0.999", { digits: 0 })).toBe("1");
  });

  it("keeps full precision that a number would lose", () => {
    expect(formatDecimal("123456789012345678901234.56")).toBe("123,456,789,012,345,678,901,234.56");
  });

  it("never prints a signed zero", () => {
    expect(formatDecimal("-0.001")).toBe("0.00");
    expect(formatDecimal("0", { signed: true })).toBe("0.00");
    expect(formatDecimal("1", { signed: true })).toBe("+1.00");
  });

  it("reads an unusable value as zero rather than NaN", () => {
    expect(formatDecimal("")).toBe("0.00");
    expect(formatDecimal("n/a")).toBe("0.00");
  });
});

describe("formatPercent", () => {
  it("shifts the decimal point instead of multiplying by a hundred", () => {
    // 0.001 * 100 is 0.1 with a double's noise; the digits move two places instead.
    expect(formatPercent("0.001")).toBe("0.10 %");
    expect(formatPercent("-0.12345")).toBe("-12.35 %");
    expect(formatPercent("1")).toBe("100.00 %");
  });
});

describe("formatQuantity", () => {
  it("drops trailing zeros and stops at eight decimals", () => {
    expect(formatQuantity("10.500000")).toBe("10.5");
    expect(formatQuantity("0.000000012345")).toBe("0.00000001");
    expect(formatQuantity("3")).toBe("3");
  });
});

describe("signOf", () => {
  it("calls a value that rounds to nothing zero, so a hair is not coloured as a gain", () => {
    expect(signOf("0.0001")).toBe("zero");
    expect(signOf("-0.0001")).toBe("zero");
    expect(signOf("0.01")).toBe("positive");
    expect(signOf("-0.01")).toBe("negative");
  });
});

describe("toNumber", () => {
  it("is null for what cannot be sorted, not zero", () => {
    expect(toNumber(null)).toBeNull();
    expect(toNumber("")).toBeNull();
    expect(toNumber("abc")).toBeNull();
    expect(toNumber("1.5")).toBe(1.5);
  });
});
