import { Panel, type Keel } from "./Panel";

interface StatCardProps {
  label: string;
  value: string;
  note?: string;
  tone?: "furnace" | "coolant" | "nominal" | "alert" | "plain";
  keel?: Keel;
}

export function StatCard({ label, value, note, tone = "plain", keel = "none" }: StatCardProps) {
  const toneClass = tone === "plain" ? "" : tone;
  return (
    <Panel keel={keel}>
      <div className="stat-label">{label}</div>
      <div className={`stat-value ${toneClass}`.trim()}>{value}</div>
      {note && <div className="stat-note">{note}</div>}
    </Panel>
  );
}
