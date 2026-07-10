import { useEffect, useState } from "react";
import {
  createRealOperation,
  listBlueprints,
  type BlueprintRecord,
  type TypeSearchResult,
  type CreatedOperation,
} from "../lib/backend";
import { BuildTargetSearch } from "./BuildTargetSearch";
import { Panel } from "./Panel";

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
  const [ownedBlueprints, setOwnedBlueprints] = useState<BlueprintRecord[]>([]);
  const [selectedBlueprintId, setSelectedBlueprintId] = useState<number | null>(null);
  const [assumedMe, setAssumedMe] = useState(0);
  const [assumedTe, setAssumedTe] = useState(0);
  const [assumedIsBpc, setAssumedIsBpc] = useState(false);
  const [assumedRuns, setAssumedRuns] = useState<number | "">("");

  const [showAdvanced, setShowAdvanced] = useState(false);
  const [customName, setCustomName] = useState("");
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
    listBlueprints().then((all) => {
      const matches = all.filter((b) => b.typeName === target.name);
      setOwnedBlueprints(matches);
      if (matches.length > 0) {
        setMode("owned");
        setSelectedBlueprintId(matches[0].blueprintId);
      }
    });
  }, [target]);

  const canSubmit = target !== null && quantity >= 1 && !submitting;

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
        ownedBlueprintId: mode === "owned" ? selectedBlueprintId : null,
        assumedMe: mode === "assumed" ? assumedMe : null,
        assumedTe: mode === "assumed" ? assumedTe : null,
        assumedIsBpc: mode === "assumed" ? assumedIsBpc : null,
        assumedRuns: mode === "assumed" && assumedIsBpc && assumedRuns !== "" ? Number(assumedRuns) : null,
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
                  <label key={b.blueprintId} className="new-op-radio-row">
                    <input
                      type="radio"
                      name="ownedBlueprint"
                      checked={selectedBlueprintId === b.blueprintId}
                      onChange={() => setSelectedBlueprintId(b.blueprintId)}
                    />
                    {b.isCopy ? "BPC" : "BPO"} · ME {b.meLevel} · TE {b.teLevel}
                    {b.runsRemaining !== null ? ` · ${b.runsRemaining} runs left` : ""}
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
