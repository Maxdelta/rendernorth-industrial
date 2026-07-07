import { useEffect, useState } from "react";
import {
  getBlueprintSummary,
  listBlueprints,
  getMissingBlueprintReport,
  type BlueprintSummary,
  type BlueprintRecord,
  type MissingBlueprintReport,
} from "../lib/backend";
import { Panel } from "../components/Panel";
import { StatCard } from "../components/StatCard";

function statusTone(status: string): "nominal" | "furnace" | "coolant" {
  if (status === "idle" || status === "in_use") return "nominal";
  if (status === "researching" || status === "copying") return "furnace";
  return "coolant";
}

export function BlueprintsPage() {
  const [summary, setSummary] = useState<BlueprintSummary | null>(null);
  const [blueprints, setBlueprints] = useState<BlueprintRecord[] | null>(null);
  const [missing, setMissing] = useState<MissingBlueprintReport[] | null>(null);

  useEffect(() => {
    let live = true;
    getBlueprintSummary().then((s) => live && setSummary(s));
    listBlueprints().then((b) => live && setBlueprints(b));
    getMissingBlueprintReport().then((m) => live && setMissing(m));
    return () => {
      live = false;
    };
  }, []);

  if (!summary) {
    return <div className="data-source">Bringing Blueprint Engine online…</div>;
  }

  return (
    <div className="dash">
      <div className="stat-strip">
        <StatCard label="Total Blueprints" value={String(summary.totalBlueprints)} tone="coolant" keel="coolant" note="BPOs and BPCs combined" />
        <StatCard label="BPO Count" value={String(summary.bpoCount)} tone="nominal" keel="nominal" note="originals owned" />
        <StatCard label="BPC Count" value={String(summary.bpcCount)} tone="coolant" keel="coolant" note="copies owned" />
        <StatCard label="Research Complete" value={String(summary.researchComplete)} tone="nominal" keel="nominal" note="not currently researching" />
        <StatCard label="Copies" value={String(summary.copies)} tone="coolant" keel="coolant" note="total runs remaining across BPCs" />
        <StatCard
          label="Missing For Operations"
          value={String(summary.missingForOperations)}
          tone={summary.missingForOperations > 0 ? "alert" : "nominal"}
          keel={summary.missingForOperations > 0 ? "alert" : "nominal"}
          note="required blueprint types not owned"
        />
      </div>

      <Panel title="Blueprint Library" keel="furnace" className="dash-hero">
        {!blueprints ? (
          <div className="data-source">Loading blueprints…</div>
        ) : (
          <div className="inv-table">
            <div className="inv-row bp-row bp-head">
              <div>Blueprint</div>
              <div>Type</div>
              <div>ME</div>
              <div>TE</div>
              <div>Runs</div>
              <div>Location</div>
              <div>Owner</div>
              <div>Status</div>
              <div>Operation</div>
            </div>
            {blueprints.map((b) => (
              <div className="inv-row bp-row" key={b.blueprintId}>
                <div className="inv-name">{b.typeName}</div>
                <div className="bp-type">{b.isCopy ? "BPC" : "BPO"}</div>
                <div className="inv-qty">{b.meLevel}</div>
                <div className="inv-qty">{b.teLevel}</div>
                <div className="inv-qty">{b.runsRemaining ?? "∞"}</div>
                <div className="inv-location">{b.locationName}</div>
                <div className="inv-owner">{b.ownerName}</div>
                <div className={`inv-status ${statusTone(b.status)}`}>{b.status.replace(/_/g, " ")}</div>
                <div className="inv-operation">{b.linkedOperation ?? "—"}</div>
              </div>
            ))}
          </div>
        )}
      </Panel>

      {missing && (
        <Panel title="Missing Blueprint Report" keel={missing.length > 0 ? "alert" : "nominal"} className="dash-hero">
          {missing.length === 0 ? (
            <p className="ph-mission">Every required blueprint type is owned somewhere. Detection only — nothing here acquires or researches a blueprint.</p>
          ) : (
            <div className="conflict-list">
              {missing.map((m) => (
                <div className="conflict-row" key={m.typeName}>
                  <div className="conflict-type">not owned</div>
                  <div className="conflict-desc">
                    {m.typeName} — required by {m.requiredByOperations.join(", ")}
                  </div>
                </div>
              ))}
            </div>
          )}
        </Panel>
      )}

      <div className="data-source" style={{ gridColumn: "1 / -1" }}>
        Blueprint Engine — industrial capability records, not just inventory items.
      </div>
    </div>
  );
}
