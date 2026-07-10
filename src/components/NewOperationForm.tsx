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

interface NewOperationFormProps {
  onCreated: (result: CreatedOperation) => void;
  onCancel: () => void;
}

export function NewOperationForm({ onCreated, onCancel }: NewOperationFormProps) {
  const [goal, setGoal] = useState("");
  const [priority, setPriority] = useState(3);
  const [deadline, setDeadline] = useState("");
  const [notes, setNotes] = useState("");
  const [target, setTarget] = useState<TypeSearchResult | null>(null);
  const [quantity, setQuantity] = useState(1);
  const [mode, setMode] = useState<"assumed" | "owned">("assumed");
  const [ownedBlueprints, setOwnedBlueprints] = useState<BlueprintRecord[]>([]);
  const [selectedBlueprintId, setSelectedBlueprintId] = useState<number | null>(null);
  const [assumedMe, setAssumedMe] = useState(0);
  const [assumedTe, setAssumedTe] = useState(0);
  const [assumedIsBpc, setAssumedIsBpc] = useState(false);
  const [assumedRuns, setAssumedRuns] = useState<number | "">("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!target) return;
    listBlueprints().then((all) => setOwnedBlueprints(all.filter((b) => b.typeName === target.name)));
  }, [target]);

  async function handleSubmit() {
    if (!goal.trim()) {
      setError("Goal is required.");
      return;
    }
    setSubmitting(true);
    setError(null);
    try {
      const result = await createRealOperation({
        goal: goal.trim(),
        priority,
        deadline: deadline || null,
        notes: notes || null,
        typeId: target?.typeId ?? null,
        quantityRequested: target ? quantity : null,
        blueprintMode: target ? mode : null,
        ownedBlueprintId: target && mode === "owned" ? selectedBlueprintId : null,
        assumedMe: target && mode === "assumed" ? assumedMe : null,
        assumedTe: target && mode === "assumed" ? assumedTe : null,
        assumedIsBpc: target && mode === "assumed" ? assumedIsBpc : null,
        assumedRuns: target && mode === "assumed" && assumedIsBpc && assumedRuns !== "" ? Number(assumedRuns) : null,
      });
      onCreated(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <Panel title="New Operation" keel="furnace" className="dash-hero">
      <div className="new-op-form">
        <label className="new-op-field">
          <span className="ops-field-label">Goal</span>
          <input className="sd-path-input" value={goal} onChange={(e) => setGoal(e.target.value)} placeholder="e.g. Build 3 Rifters" />
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

        <div className="new-op-field">
          <span className="ops-field-label">Build Target (optional)</span>
          <BuildTargetSearch selected={target} onSelect={setTarget} />
        </div>

        {target && (
          <>
            <label className="new-op-field">
              <span className="ops-field-label">Quantity Requested</span>
              <input
                className="sd-path-input"
                type="number"
                min={1}
                value={quantity}
                onChange={(e) => setQuantity(Math.max(1, Number(e.target.value)))}
              />
            </label>

            <div className="new-op-field">
              <span className="ops-field-label">Blueprint Source</span>
              <div className="new-op-mode-toggle">
                <button className={mode === "assumed" ? "target-select enabled active" : "target-select"} onClick={() => setMode("assumed")}>
                  Assumption
                </button>
                <button className={mode === "owned" ? "target-select enabled active" : "target-select"} onClick={() => setMode("owned")}>
                  Owned Blueprint
                </button>
              </div>
            </div>

            {mode === "owned" ? (
              ownedBlueprints.length === 0 ? (
                <p className="ph-mission">No owned blueprint matches this type name yet — use an assumption instead.</p>
              ) : (
                <div className="new-op-field">
                  <span className="ops-field-label">Owned Blueprint</span>
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
              )
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
          </>
        )}

        {error && (
          <div className="sd-error">
            <div className="conflict-type">could not create operation</div>
            <div className="conflict-desc">{error}</div>
          </div>
        )}

        <div className="new-op-actions">
          <button className="target-select enabled" onClick={handleSubmit} disabled={submitting}>
            {submitting ? "Creating…" : "Create Operation"}
          </button>
          <button className="target-select" onClick={onCancel} disabled={submitting}>
            Cancel
          </button>
        </div>
      </div>
    </Panel>
  );
}
