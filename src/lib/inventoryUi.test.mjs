import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { createRequestGate } from "./requestGate.js";

const inventorySource = await readFile(new URL("../pages/Inventory.tsx", import.meta.url), "utf8");
const marketSource = await readFile(new URL("../components/MarketInventory.tsx", import.meta.url), "utf8");
const styles = await readFile(new URL("../styles/app.css", import.meta.url), "utf8");
const pageSource = inventorySource.slice(inventorySource.indexOf("export function InventoryPage"));
const rawSource = inventorySource.slice(
  inventorySource.indexOf("function SynchronizedAssetsSection"),
  inventorySource.indexOf("function ManualInventorySection"),
);

test("Inventory Search and Market Valuation renders before raw asset details", () => {
  assert.ok(pageSource.indexOf('data-inventory-module="market-valuation"') >= 0);
  assert.ok(pageSource.indexOf("<SynchronizedAssetsSection />") > pageSource.indexOf('data-inventory-module="market-valuation"'));
});

test("raw synchronized asset details are collapsed by default", () => {
  assert.match(rawSource, /const \[expanded, setExpanded\] = useState\(false\)/);
  assert.match(rawSource, /data-inventory-module="raw-asset-details"/);
  assert.match(rawSource, /open=\{expanded\}/);
});

test("expanding raw details displays a bounded synchronized table", () => {
  assert.match(rawSource, /\{expanded && <div className="inventory-details-body">/);
  assert.match(rawSource, /synchronized-assets-table-wrap/);
  assert.match(styles, /\.synchronized-assets-table-wrap\s*\{[\s\S]*max-height:[^;]+;[\s\S]*overflow:\s*auto;/);
  assert.match(styles, /\.synchronized-assets-table-head\s*\{[\s\S]*position:\s*sticky;/);
});

test("collapsed raw details neither fetch nor render synchronized rows", () => {
  assert.ok(rawSource.indexOf("if (!expanded) return;") < rawSource.indexOf("listSyncedAssets(scope)"));
  assert.match(rawSource, /const visibleAssets = assets\.slice/);
  assert.match(rawSource, /visibleAssets\.map/);
  assert.doesNotMatch(rawSource, /assets\.map/);
});

test("raw details paginate large inventories to a safe row count", () => {
  assert.match(inventorySource, /RAW_ASSET_PAGE_SIZE = 100/);
  assert.match(rawSource, /Previous/);
  assert.match(rawSource, /Next/);
});

test("primary inventory totals use all rows while rendering a safe page", () => {
  assert.match(marketSource, /INVENTORY_PAGE_SIZE = 100/);
  assert.match(marketSource, /inventoryResult\?\.rows\.slice/);
  assert.match(marketSource, /visibleInventoryRows\.map/);
  assert.doesNotMatch(marketSource, /inventoryResult\.rows\.map/);
  assert.match(marketSource, /inventoryResult\.matchingStacks/);
});

test("primary Asset Source offers Personal, Corporation, and Both", () => {
  assert.match(marketSource, /type AssetScope = "personal" \| "corporation" \| "both"/);
  assert.match(marketSource, /assetScope: "both"/);
  assert.match(marketSource, /aria-label="Asset Source"/);
});

test("primary rows keep owner type, owner, division, location, and source visible", () => {
  for (const heading of ["Owner Type", "Owner Name", "Division", "Owned item location", "Source"]) {
    assert.ok(marketSource.includes(`>${heading}<`), heading);
  }
  assert.match(marketSource, /row\.ownerType/);
  assert.match(marketSource, /row\.division/);
  assert.match(marketSource, /row\.source/);
});

test("Market Search quotes by type and ignores owned Asset Source", () => {
  const quoteSource = marketSource.slice(marketSource.indexOf("async function loadQuote"), marketSource.indexOf("useEffect(() =>"));
  assert.match(quoteSource, /getMarketQuote\(type\.typeId, requested\)/);
  assert.doesNotMatch(quoteSource, /assetScope|searchInventoryMarket/);
});

test("raw-detail filtering cannot reset primary mode, filters, or snapshot", () => {
  assert.match(rawSource, /const \[scope, setScope\] = useState<OwnerScope>/);
  assert.match(marketSource, /const \[mode, setMode\] = useState<SearchMode>/);
  assert.match(marketSource, /const \[input, setInput\] = useState<InventorySearchInput>/);
  assert.match(marketSource, /const \[profile, setProfile\] = useState<MarketProfile/);
  assert.doesNotMatch(rawSource, /setMode|setInput|setProfile/);
});

test("only the newest asynchronous asset or market response remains current", () => {
  assert.match(rawSource, /requestGate\.current\.begin\(\)/);
  assert.match(marketSource, /marketRequestGate\.current\.begin\(\)/);
  const gate = createRequestGate();
  const personal = gate.begin();
  const corporation = gate.begin();
  const both = gate.begin();
  assert.equal(personal(), false);
  assert.equal(corporation(), false);
  assert.equal(both(), true);
  gate.invalidate();
  assert.equal(both(), false);
});
