import type { BuildTargetSummary } from "../lib/backend";

interface TargetPickerProps {
  targets: BuildTargetSummary[];
  onSelect: (projectId: number) => void;
}

/** Generic build-target list with selection. Any manufacturable item; no hull special-cased. */
export function TargetPicker({ targets, onSelect }: TargetPickerProps) {
  return (
    <div className="target-picker" role="listbox" aria-label="Build targets">
      {targets.map((t) => (
        <div className={t.isSelected ? "target-row selected" : "target-row"} key={t.projectId}>
          <div className="target-info">
            <div className="target-name">{t.name}</div>
            <div className="target-class">{t.className}</div>
          </div>
          <div className="target-progress">{Math.round(t.overallProgress * 100)}%</div>
          <div className="target-status">{t.status}</div>
          {t.isSelected ? (
            <span className="target-active">ACTIVE OPERATION</span>
          ) : (
            <button className="target-select enabled" onClick={() => onSelect(t.projectId)}>
              Select
            </button>
          )}
        </div>
      ))}
    </div>
  );
}
