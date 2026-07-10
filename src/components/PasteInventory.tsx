import { useState } from "react";
import { searchEveTypes, addManualInventoryBulk, type NewManualInventoryEntry } from "../lib/backend";
import { parsePastedInventory, type ParsedInventoryRow } from "../lib/pasteInventoryParser";
import { Panel } from "./Panel";

interface MatchedRow {
  name: string;
  quantity: number;
  typeId: number | null;
  typeName: string | null;
}

interface PasteInventoryProps {
  onClose: () => void;
  onImported: () => void;
}

export function PasteInventory({ onClose, onImported }: PasteInventoryProps) {
  const [text, setText] = useState("");
  const [matched, setMatched] = useState<MatchedRow[] | null>(null);
  const [unmatched, setUnmatched] = useState<ParsedInventoryRow[]>([]);
  const [checking, setChecking] = useState(false);
  const [confirming, setConfirming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [done, setDone] = useState<number | null>(null);

  async function handleCheck() {
    setChecking(true);
    setError(null);
    setDone(null);
    try {
      const parsed = parsePastedInventory(text);
      const invalid = parsed.filter((p) => p.problem !== null);
      const valid = parsed.filter((p) => p.problem === null);

      const uniqueNames = [...new Set(valid.map((v) => v.name as string))];
      const matches = new Map<string, { typeId: number; typeName: string } | null>();
      for (const name of uniqueNames) {
        const results = await searchEveTypes(name, 5);
        const exact = results.find((r) => r.name.toLowerCase() === name.toLowerCase());
        matches.set(name, exact ? { typeId: exact.typeId, typeName: exact.name } : null);
      }

      const merged = new Map<string, MatchedRow>();
      for (const v of valid) {
        const m = matches.get(v.name as string) ?? null;
        const key = m ? `type:${m.typeId}` : `unmatched:${v.name}`;
        const existing = merged.get(key);
        if (existing) {
          existing.quantity += v.quantity as number;
        } else {
          merged.set(key, {
            name: v.name as string,
            quantity: v.quantity as number,
            typeId: m?.typeId ?? null,
            typeName: m?.typeName ?? null,
          });
        }
      }

      setMatched([...merged.values()]);
      setUnmatched(invalid);
    } catch (err) {
      setError(String(err));
    } finally {
      setChecking(false);
    }
  }

  async function handleConfirm() {
    if (!matched) return;
    const toImport = matched.filter((m) => m.typeId !== null);
    if (toImport.length === 0) return;
    setConfirming(true);
    setError(null);
    try {
      const entries: NewManualInventoryEntry[] = toImport.map((m) => ({
        typeId: m.typeId as number,
        quantity: m.quantity,
        locationName: null,
      }));
      const count = await addManualInventoryBulk(entries);
      setDone(count);
      onImported();
    } catch (err) {
      setError(String(err));
    } finally {
      setConfirming(false);
    }
  }

  const matchedCount = matched?.filter((m) => m.typeId !== null).length ?? 0;
  const unmatchedTypeCount = matched?.filter((m) => m.typeId === null).length ?? 0;

  return (
    <Panel
      title="Paste Inventory"
      keel="furnace"
      className="dash-hero"
      headerRight={
        <button className="target-select" onClick={onClose}>
          Close
        </button>
      }
    >
      <p className="ph-mission">
        Paste rows of <strong>Type Name</strong> and <strong>Quantity</strong> — tab, comma, or multiple-space
        separated, with or without a header row. Commas inside quantities (thousands separators) are handled.
        Nothing is saved until you review and confirm below.
      </p>

      <textarea
        className="paste-inv-textarea"
        placeholder={"Tritanium\t125000000\nPyerite\t42000000\nMegacyte\t850000\nCapital Armor Plates\t37"}
        value={text}
        onChange={(e) => {
          setText(e.target.value);
          setMatched(null);
          setDone(null);
        }}
        rows={10}
      />

      <div className="new-op-actions">
        <button className="target-select enabled" onClick={handleCheck} disabled={checking || !text.trim()}>
          {checking ? "Checking…" : "Preview"}
        </button>
      </div>

      {error && (
        <div className="sd-error">
          <div className="conflict-desc">{error}</div>
        </div>
      )}

      {matched && (
        <>
          <div className="res-summary-grid" style={{ marginTop: 14 }}>
            <div className="res-summary-cell">
              <div className="res-summary-label">Matched Types</div>
              <div className="res-summary-value nominal">{matchedCount}</div>
            </div>
            <div className="res-summary-cell">
              <div className="res-summary-label">Unmatched Types</div>
              <div className={`res-summary-value ${unmatchedTypeCount > 0 ? "furnace" : "nominal"}`}>{unmatchedTypeCount}</div>
            </div>
            <div className="res-summary-cell">
              <div className="res-summary-label">Invalid Lines</div>
              <div className={`res-summary-value ${unmatched.length > 0 ? "alert" : "nominal"}`}>{unmatched.length}</div>
            </div>
          </div>

          <div className="inv-table" style={{ marginTop: 14 }}>
            <div className="inv-row bp-row bp-head">
              <div>Parsed Name</div>
              <div>Quantity</div>
              <div>Matched Type</div>
              <div>Status</div>
            </div>
            {matched.map((m, i) => (
              <div className="inv-row bp-row" key={i}>
                <div className="inv-name">{m.name}</div>
                <div className="inv-qty">{m.quantity.toLocaleString()}</div>
                <div className="inv-location">{m.typeName ?? "—"}</div>
                <div className={`inv-status ${m.typeId !== null ? "nominal" : "alert"}`}>
                  {m.typeId !== null ? "Matched" : "No match"}
                </div>
              </div>
            ))}
          </div>

          {unmatched.length > 0 && (
            <div className="conflict-list" style={{ marginTop: 10 }}>
              {unmatched.map((u, i) => (
                <div className="conflict-row" key={i}>
                  <div className="conflict-type">{u.problem === "invalid_quantity" ? "bad quantity" : "unreadable line"}</div>
                  <div className="conflict-desc">{u.rawLine}</div>
                </div>
              ))}
            </div>
          )}

          <div className="new-op-actions" style={{ marginTop: 14 }}>
            <button className="target-select enabled" onClick={handleConfirm} disabled={confirming || matchedCount === 0}>
              {confirming ? "Saving…" : `Confirm ${matchedCount} Item${matchedCount === 1 ? "" : "s"}`}
            </button>
          </div>
        </>
      )}

      {done !== null && (
        <div className="data-source" style={{ marginTop: 10 }}>
          Saved {done} manual inventory entr{done === 1 ? "y" : "ies"}.
        </div>
      )}
    </Panel>
  );
}
