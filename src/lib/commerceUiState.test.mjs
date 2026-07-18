import assert from "node:assert/strict";
import test from "node:test";
import { commerceTabAttributes, switchCommerceTab } from "./commerceUiState.js";

test("active Commerce tab exposes selected and keyboard state", () => {
  assert.deepEqual(commerceTabAttributes("orders", "orders"), { "aria-selected": true, tabIndex: 0 });
  assert.deepEqual(commerceTabAttributes("orders", "contracts"), { "aria-selected": false, tabIndex: -1 });
});

test("switching Commerce modules preserves independent filter state", () => {
  const orderFilters = { side: "Sell", location: "Jita", sort: "value" };
  const contractFilters = { direction: "Issued", status: "outstanding", availability: "public" };
  const switched = switchCommerceTab({ tab: "orders", orderFilters, contractFilters }, "contracts");
  assert.equal(switched.tab, "contracts");
  assert.strictEqual(switched.orderFilters, orderFilters);
  assert.strictEqual(switched.contractFilters, contractFilters);
});
