const EXACT_ISK = /^([+-]?)(\d+)(?:\.(\d+))?$/;

function parseExact(value) {
  if (typeof value !== "string") return null;
  const trimmed = value.trim();
  const match = EXACT_ISK.exec(trimmed);
  if (!match) return null;
  const negative = match[1] === "-";
  const whole = match[2].replace(/^0+(?=\d)/, "");
  const fraction = match[3] ?? "";
  const scale = 10n ** BigInt(fraction.length);
  const digits = BigInt(`${whole}${fraction}`);
  return { negative, whole, fraction, scale, digits };
}

function groupWhole(whole) {
  return whole.replace(/\B(?=(\d{3})+(?!\d))/g, ",");
}

/** Format an exact decimal string without converting it through Number. */
export function formatExactIsk(value) {
  if (value == null) return "—";
  const parsed = parseExact(value);
  if (!parsed) return `${String(value).trim()} ISK`;
  const fraction = parsed.fraction.length === 0
    ? "00"
    : parsed.fraction.length === 1
      ? `${parsed.fraction}0`
      : parsed.fraction;
  const sign = parsed.negative && parsed.digits !== 0n ? "-" : "";
  return `${sign}${groupWhole(parsed.whole)}.${fraction} ISK`;
}

/**
 * Compact an exact decimal string to K/M/B/T for display. The returned exact
 * string remains suitable for title text and accessible labels.
 */
export function formatCompactIsk(value) {
  const exact = formatExactIsk(value);
  const parsed = value == null ? null : parseExact(value);
  if (!parsed) return { compact: exact, exact, valid: false };
  const units = [
    ["T", 1_000_000_000_000n],
    ["B", 1_000_000_000n],
    ["M", 1_000_000n],
    ["K", 1_000n],
  ];
  const unit = units.find(([, threshold]) => parsed.digits >= threshold * parsed.scale);
  if (!unit) return { compact: exact, exact, valid: true };
  const [suffix, threshold] = unit;
  const denominator = threshold * parsed.scale;
  const roundedHundredths = (parsed.digits * 100n + denominator / 2n) / denominator;
  const sign = parsed.negative && parsed.digits !== 0n ? "-" : "";
  const compact = `${sign}${roundedHundredths / 100n}.${String(roundedHundredths % 100n).padStart(2, "0")} ${suffix} ISK`;
  return { compact, exact, valid: true };
}
