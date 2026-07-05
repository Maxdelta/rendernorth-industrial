import { useEffect, useState } from "react";
import { listBuildTargets, selectBuildTarget, type BuildTargetSummary } from "../lib/backend";
import { Panel } from "../components/Panel";
import { TargetPicker } from "../components/TargetPicker";

export function BuildTargetsPage() {
  const [targets, setTargets] = useState<BuildTargetSummary[]>([]);

  useEffect(() => {
    let live = true;
    listBuildTargets().then((t) => live && setTargets(t));
    return () => {
      live = false;
    };
  }, []);

  async function handleSelect(projectId: number) {
    await selectBuildTarget(projectId);
    setTargets(await listBuildTargets());
  }

  return (
    <div className="placeholder" style={{ maxWidth: 860 }}>
      <Panel title="Build Targets" keel="furnace">
        <p className="ph-mission">
          Every operation starts here: select any manufacturable target and Mission Control re-centers on it.
          Sprint 002 ships five demo targets; once the SDE lands, this list becomes a search over every
          blueprint in EVE industry data — hulls, structures, and components alike. No ship is special-cased.
        </p>
        <TargetPicker targets={targets} onSelect={handleSelect} />
        <div className="ph-sprint">Full pipeline per target: Select → Load Blueprint Requirements → Calculate Materials → Compare Inventory → Identify Missing Inputs → Recommend Next Action</div>
      </Panel>
    </div>
  );
}
