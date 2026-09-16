/**
 * ISO 4217 numeric → alphabetic for codes this engine actually posts.
 * Profiles seed USD; ledger defaults and demo traces use 840. CurrencyId is
 * an unconstrained ISO 4217 numeric, so unknown codes fall back to digits.
 */
const ISO_4217_ALPHA: Record<number, string> = {
  124: "CAD",
  156: "CNY",
  392: "JPY",
  484: "MXN",
  826: "GBP",
  840: "USD",
  978: "EUR",
};

export function formatCurrencyCode(code: number): string {
  return ISO_4217_ALPHA[code] ?? String(code);
}
