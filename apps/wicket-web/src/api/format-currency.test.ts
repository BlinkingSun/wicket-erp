import { describe, expect, it } from "vitest";
import { formatCurrencyCode } from "./format-currency";

describe("formatCurrencyCode", () => {
  it("maps USD numeric 840 to the alphabetic code", () => {
    expect(formatCurrencyCode(840)).toBe("USD");
  });

  it("falls back to the numeric code for an unmapped ISO 4217 value", () => {
    expect(formatCurrencyCode(999)).toBe("999");
    expect(formatCurrencyCode(0)).toBe("0");
  });
});
