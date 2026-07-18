import assert from "node:assert/strict";
import test from "node:test";
import { formatCompactIsk, formatExactIsk } from "./isk.js";

test("compact ISK formatter uses exact K/M/B/T thresholds", () => {
  assert.equal(formatCompactIsk("1250.00").compact, "1.25 K ISK");
  assert.equal(formatCompactIsk("4300000.00").compact, "4.30 M ISK");
  assert.equal(formatCompactIsk("27319999999.00").compact, "27.32 B ISK");
  assert.equal(formatCompactIsk("1425000000000.00").compact, "1.43 T ISK");
  assert.equal(formatCompactIsk("999.99").compact, "999.99 ISK");
});

test("exact ISK formatter preserves arbitrary-size decimal strings", () => {
  const input = "999999999999999999999999.42";
  assert.equal(formatExactIsk(input), "999,999,999,999,999,999,999,999.42 ISK");
  assert.equal(formatCompactIsk(input).exact, "999,999,999,999,999,999,999,999.42 ISK");
});

test("compact ISK formatter handles negative, zero, and malformed values", () => {
  assert.equal(formatCompactIsk("-4300000.00").compact, "-4.30 M ISK");
  assert.equal(formatCompactIsk("0.00").compact, "0.00 ISK");
  assert.deepEqual(formatCompactIsk("not-a-number"), {
    compact: "not-a-number ISK",
    exact: "not-a-number ISK",
    valid: false,
  });
  assert.equal(formatCompactIsk(null).compact, "—");
});
