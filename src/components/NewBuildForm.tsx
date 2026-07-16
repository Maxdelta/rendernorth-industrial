import { useEffect, useState } from "react";
import {
  createRealOperation,
  listOwnedBlueprintCandidates,
  type OwnedBlueprintCandidate,
  type TypeSearchResult,
  type CreatedOperation,
  type InventoryScope,
} from "../lib/backend";
import { BuildTargetSearch } from "./BuildTargetSearch";
import { Panel } from "./Panel";
import { LocationDisplay } from "./LocationDisplay";

interface NewBuildFormProps {
  onCreated: (result: CreatedOperation) => void;
  onCancel: () => void;
}

/** "Build 3 Navy Revelations" / "Build 1 Avatar" — a simple, always-editable starting point, never a blocker. */
function autoGoal(targetName: string, quantity: number): string {
  if (quantity === 1) return `Build 1 ${targetName}`;
  const plural = targetName.endsWith("s") ? targetName : `${targetName}s`;
  return `Build ${quantity} ${plural}`;
}

export function NewBuildForm({ onCreated, onCancel }: NewBuildFormProps) {
  const [target, setTarget] = useState<TypeSearchResult | null>(null);
  const [quantity, setQuantity] = useState(1);
  const [mode, setMode] = useState<"assumed" | "owned">("assumed");
  const [ownedBlueprints, setOwnedBlueprints] = useState<OwnedBlueprintCandidate[]>([]);
  const [selectedBlueprintKey, setSelectedBlueprintKey] = useState<string | null>(null);
  const [assumedMe, setAssumedMe] = useState(0);
  const [assumedTe, setAssumedTe] = useState(0);
  const [assumedIsBpc, setAssumedIsBpc] = useState(false);
  const [assumedRuns, setAssumedRuns] = useState<number | "">("");

  const [showAdvanced, setShowAdvanced] = useState(false);
  const [customName, setCustomName] = useState("");
  const [inventoryScope, setInventoryScope] = useState<InventoryScope>("all_included_inventory");
  const [priority, setPriority] = useState(3);
  const [deadline, setDeadline] = useState("");
  const [notes, setNotes] = useState("");

  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!target) {
      setOwnedBlueprints([]);
      setMode("assumed");
      return;
    }
    listOwnedBlueprintCandidates(target.typeId, quantity).then((matches) => {
      setOwnedBlueprints(matches);
      if (matches.length > 0) {
        setMode("owned");
        setSelectedBlueprintKey((current) =>
          current && matches.some((candidate) => candidateKey(candidate) === current)
            ? current
            : candidateKey(matches[0]),
        );
      } else {
        setMode("assumed");
        setSelectedBlueprintKey(null);
      }
    }).catch((err) => {
      setError(String(err));
      setOwnedBlueprints([]);
      setMode("assumed");
      setSelectedBlueprintKey(null);
    });
  }, [target, quantity]);

  function candidateKey(candidate: OwnedBlueprintCandidate): string {
    return candidate.source === "manual"
      ? `manual:${candidate.manualBlueprintId}`
      : `esi:${candidate.characterId}:${candidate.itemId}`;
  }

  const selectedBlueprint =
    ownedBlueprints.find((candidate) => candidateKey(candidate) === selectedBlueprintKey) ?? null;

  const canSubmit = target !== null && quantity >= 1 && !submitting && (mode !== "owned" || selectedBlueprint !== null);

  async function handleSubmit() {
    if (!target) return;
    setSubmitting(true);
    setError(null);
    try {
      const goal = customName.trim() || autoGoal(target.name, quantity);
      const result = await createRealOperation({
        goal,
        priority,
        deadline: deadline || null,
        notes: notes || null,
        typeId: target.typeId,
        quantityRequested: quantity,
        blueprintMode: mode,
        ownedBlueprintId: mode === "owned" && selectedBlueprint?.source === "manual" ? selectedBlueprint.manualBlueprintId : null,
        selectedBlueprintSource: mode === "owned" ? selectedBlueprint?.source ?? null : null,
        manualBlueprintId: mode === "owned" && selectedBlueprint?.source === "manual" ? selectedBlueprint.manualBlueprintId : null,
        characterBlueprintCharacterId: mode === "owned" && selectedBlueprint?.source === "esi_character" ? selectedBlueprint.characterId : null,
        characterBlueprintItemId: mode === "owned" && selectedBlueprint?.source === "esi_character" ? selectedBlueprint.itemId : null,
        assumedMe: mode === "assumed" ? assumedMe : null,
        assumedTe: mode === "assumed" ? assumedTe : null,
        assumedIsBpc: mode === "assumed" ? assumedIsBpc : null,
        assumedRuns: mode === "assumed" && assumedIsBpc && assumedRuns !== "" ? Number(assumedRuns) : null,
        inventoryScope,
      });
      onCreated(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <Panel title="New Build" keel="furnace" className="dash-hero">
      <div className="new-op-form">
        <div className="new-op-field">
          <span className="ops-field-label">What do you want to build?</span>
          <BuildTargetSearch selected={target} onSelect={setTarget} />
        </div>

        {target && (
          <>
            <label className="new-op-field">
              <span className="ops-field-label">Quantity</span>
              <input
                className="sd-path-input"
                type="number"
                min={1}
                value={quantity}
                onChange={(e) => setQuantity(Math.max(1, Number(e.target.value)))}
                style={{ maxWidth: 140 }}
              />
            </label>

            <div className="new-op-field">
              <span className="ops-field-label">Blueprint Source</span>
              <div className="new-op-mode-toggle">
                <button className={mode === "assumed" ? "target-select enabled active" : "target-select"} onClick={() => setMode("assumed")}>
                  Assumption
                </button>
                <button
                  className={mode === "owned" ? "target-select enabled active" : "target-select"}
                  onClick={() => setMode("owned")}
                  disabled={ownedBlueprints.length === 0}
                  title={ownedBlueprints.length === 0 ? "No owned blueprint matches this target yet" : undefined}
                >
                  Owned Blueprint{ownedBlueprints.length > 0 ? "" : " (none owned)"}
                </button>
              </div>
            </div>

            {mode === "owned" ? (
              <div className="new-op-field">
                {ownedBlueprints.map((b) => (
                  <label key={candidateKey(b)} className="new-op-radio-row">
                    <input
                      type="radio"
                      name="ownedBlueprint"
                      checked={selectedBlueprintKey === candidateKey(b)}
                      onChange={() => setSelectedBlueprintKey(candidateKey(b))}
                    />
                    <span>
                      {b.isCopy ? "BPC" : "BPO"} · {b.ownerName} · {b.sourceLabel} · ME {b.me} · TE {b.te}
                      {" · "}{b.runsRemaining !== null ? `${b.runsRemaining} runs left` : "Infinite"}
                      {b.resolvedLocation && <><br /><LocationDisplay location={b.resolvedLocation}/></>}
                    </span>
                  </label>
                ))}
              </div>
            ) : (
              <div className="new-op-row">
                <label className="new-op-field">
                  <span className="ops-field-label">Assumed ME</span>
                  <input className="sd-path-input" type="number" min={0} max={10} value={assumedMe} onChange={(e) => setAssumedMe(Number(e.target.value))} />
                </label>
                <label className="new-op-field">
                  <span className="ops-field-label">Assumed TE</span>
                  <input className="sd-path-input" type="number" min={0} max={20} value={assumedTe} onChange={(e) => setAssumedTe(Number(e.target.value))} />
                </label>
                <label className="new-op-radio-row">
                  <input type="checkbox" checked={assumedIsBpc} onChange={(e) => setAssumedIsBpc(e.target.checked)} />
                  Planning as a BPC (limited runs)
                </label>
                {assumedIsBpc && (
                  <label className="new-op-field">
                    <span className="ops-field-label">Assumed Runs Available</span>
                    <input
                      className="sd-path-input"
                      type="number"
                      min={1}
                      value={assumedRuns}
                      onChange={(e) => setAssumedRuns(e.target.value === "" ? "" : Number(e.target.value))}
                    />
                  </label>
                )}
              </div>
            )}

            <div className="new-op-preview">
              Will be named <strong>{customName.trim() || autoGoal(target.name, quantity)}</strong> unless changed below.
            </div>
          </>
        )}

        <button className="advanced-toggle" onClick={() => setShowAdvanced((v) => !v)}>
          {showAdvanced ? "▾" : "▸"} Advanced (name, priority, deadline, notes)
        </button>

        {showAdvanced && (
          <div className="advanced-section">
            <label className="new-op-field">
              <span className="ops-field-label">Name (optional — auto-generated if left blank)</span>
              <input
                className="sd-path-input"
                value={customName}
                onChange={(e) => setCustomName(e.target.value)}
                placeholder={target ? autoGoal(target.name, quantity) : "Select a build target first"}
              />
            </label>

            <div className="new-op-field">
              <span className="ops-field-label">Inventory Scope</span>
              <p className="ph-mission" style={{ marginTop: 0, marginBottom: 8 }}>
                Which inventory counts toward this build's coverage. Only "All Included Inventory" is active today —
                the others are placeholders for future location-aware filtering (no route/jump calculation or ESI
                yet).
              </p>
              <div className="inv-scope-list">
                {(
                  [
                    { value: "build_location_only", label: "Build Location Only" },
                    { value: "same_solar_system", label: "Same Solar System" },
                    { value: "within_n_jumps", label: "Within N Jumps" },
                    { value: "selected_locations", label: "Selected Locations" },
                    { value: "all_included_inventory", label: "All Included Inventory" },
                  ] as const
                ).map((opt) => {
                  const enabled = opt.value === "all_included_inventory";
                  return (
                    <label key={opt.value} className="new-op-radio-row">
                      <input
                        type="radio"
                        name="inventoryScope"
                        checked={inventoryScope === opt.value}
                        disabled={!enabled}
                        onChange={() => enabled && setInventoryScope(opt.value)}
                      />
                      {opt.label}
                      {!enabled && <span className="demo-badge">COMING SOON</span>}
                    </label>
                  );
                })}
              </div>
            </div>
            <div className="new-op-row">
              <label className="new-op-field">
                <span className="ops-field-label">Priority</span>
                <input
                  className="sd-path-input"
                  type="number"
                  min={1}
                  max={5}
                  value={priority}
                  onChange={(e) => setPriority(Number(e.target.value))}
                />
              </label>
              <label className="new-op-field">
                <span className="ops-field-label">Deadline (optional)</span>
                <input className="sd-path-input" type="date" value={deadline} onChange={(e) => setDeadline(e.target.value)} />
              </label>
            </div>
            <label className="new-op-field">
              <span className="ops-field-label">Notes (optional)</span>
              <input className="sd-path-input" value={notes} onChange={(e) => setNotes(e.target.value)} />
            </label>
          </div>
        )}

        {error && (
          <div className="sd-error">
            <div className="conflict-type">could not create build</div>
            <div className="conflict-desc">{error}</div>
          </div>
        )}

        <div className="new-op-actions">
          <button className="target-select enabled" onClick={handleSubmit} disabled={!canSubmit}>
            {submitting ? "Creating…" : "Create Build"}
          </button>
          <button className="target-select" onClick={onCancel} disabled={submitting}>
            Cancel
          </button>
        </div>
      </div>
    </Panel>
  );
}
